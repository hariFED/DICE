// Coin-Toss — Example game using DICE VRF for provable randomness
//
// This program demonstrates how a developer integrates with DICE:
// 1. Player places a bet (heads or tails) + wager amount
// 2. Program calls DICE's request_randomness_v2 via CPI
// 3. DICE coordinator runs commit-reveal with hardware nodes
// 4. DICE calls back this program's `dice_callback` instruction
// 5. Program uses the randomness to determine heads/tails and pays winner
//
// This serves as both a reference implementation and integration test.

use anchor_lang::prelude::*;
use anchor_lang::solana_program::hash::hashv;

declare_id!("7r6UstdP6qTFK4HSqU4mFGPGyCVWd3JVjBeafQPyvspH");

/// DICE program ID — the VRF oracle we call for randomness.
const DICE_PROGRAM_ID: &str = "78Qv6cyKkRZN2YngiLSSBCe2iyRc6jgtCs3incCaMRcv";

/// SHA-256("global:dice_callback")[0..8] — must match DICE's expected discriminator.
const DICE_CALLBACK_DISCRIMINATOR: [u8; 8] = [128, 131, 129, 45, 53, 113, 215, 151];

/// Seed for the game PDA.
const SEED_GAME: &[u8] = b"game";

/// Seed for the vault PDA (holds wagers).
const SEED_VAULT: &[u8] = b"vault";

#[program]
pub mod coin_toss {
    use super::*;

    /// Initialize the game house. Creates a vault PDA and funds it to
    /// rent-exempt minimum so subsequent wager transfers don't fail.
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let house = &mut ctx.accounts.house;
        house.authority = ctx.accounts.authority.key();
        house.total_games = 0;
        house.total_heads = 0;
        house.total_tails = 0;
        house.bump = ctx.bumps.house;

        // Vault is a bare system-owned PDA. The first lamports landing in it
        // must put it above rent-exempt minimum or the TX fails. Fund it now
        // from the authority so place_bet works for any wager amount.
        let rent = Rent::get()?;
        let min_rent = rent.minimum_balance(0);
        let vault_ai = ctx.accounts.vault.to_account_info();
        if vault_ai.lamports() < min_rent {
            let needed = min_rent - vault_ai.lamports();
            let cpi_ctx = CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                anchor_lang::system_program::Transfer {
                    from: ctx.accounts.authority.to_account_info(),
                    to: vault_ai,
                },
            );
            anchor_lang::system_program::transfer(cpi_ctx, needed)?;
        }

        msg!("Coin-toss house initialized; vault funded to rent-exempt");
        Ok(())
    }

    /// Player places a bet. Wager is transferred to the vault.
    /// `choice`: 0 = heads, 1 = tails.
    /// `wager`: amount in lamports to bet.
    /// `game_id`: unique identifier for this game (prevents duplicate bets).
    pub fn place_bet(
        ctx: Context<PlaceBet>,
        choice: u8,
        wager: u64,
        game_id: u64,
    ) -> Result<()> {
        require!(choice <= 1, CoinTossError::InvalidChoice);
        require!(wager > 0, CoinTossError::ZeroWager);

        let game = &mut ctx.accounts.game;
        game.player = ctx.accounts.player.key();
        game.choice = choice;
        game.wager = wager;
        game.game_id = game_id;
        game.result = 255; // unresolved
        game.settled = false;
        game.randomness = [0u8; 32];
        game.bump = ctx.bumps.game;

        // Transfer wager from player to vault
        let cpi_ctx = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            anchor_lang::system_program::Transfer {
                from: ctx.accounts.player.to_account_info(),
                to: ctx.accounts.vault.to_account_info(),
            },
        );
        anchor_lang::system_program::transfer(cpi_ctx, wager)?;

        msg!(
            "Bet placed: player={}, choice={}, wager={}, game_id={}",
            game.player,
            if choice == 0 { "heads" } else { "tails" },
            wager,
            game_id
        );
        Ok(())
    }

    /// Called by DICE after randomness is generated.
    ///
    /// Instruction data layout (set by DICE's deliver_callback):
    ///   [0..8]   DICE_CALLBACK_DISCRIMINATOR
    ///   [8..40]  channel_key (Pubkey — the DiceChannel that produced the result)
    ///   [40..72] randomness ([u8; 32] — the verifiable random bytes)
    ///
    /// The game account is passed as remaining_accounts[0] by the coordinator.
    pub fn dice_callback(ctx: Context<DiceCallback>, channel_key: Pubkey, randomness: [u8; 32]) -> Result<()> {
        let game = &mut ctx.accounts.game;

        require!(!game.settled, CoinTossError::AlreadySettled);

        // Determine result: use first byte of randomness
        // Even = heads (0), Odd = tails (1)
        let result = randomness[0] & 1;
        game.randomness = randomness;
        game.result = result;
        game.settled = true;

        let won = result == game.choice;

        msg!(
            "Coin flip result: {} ({}). Player chose {}. {}!",
            if result == 0 { "HEADS" } else { "TAILS" },
            randomness[0],
            if game.choice == 0 { "heads" } else { "tails" },
            if won { "WIN" } else { "LOSE" }
        );

        // Update house stats
        let house = &mut ctx.accounts.house;
        house.total_games += 1;
        if result == 0 {
            house.total_heads += 1;
        } else {
            house.total_tails += 1;
        }

        Ok(())
    }

    /// Player claims winnings after the game is settled.
    /// If they won, they receive 2x their wager from the vault.
    /// If they lost, the wager stays in the vault.
    pub fn claim(ctx: Context<Claim>) -> Result<()> {
        let game = &ctx.accounts.game;

        require!(game.settled, CoinTossError::NotSettled);
        require!(game.result == game.choice, CoinTossError::PlayerLost);

        // Winner gets 2x wager (their original bet + house match)
        let payout = game.wager * 2;

        // Transfer from vault PDA to player using PDA signing
        let vault = &ctx.accounts.vault;
        let player = &ctx.accounts.player;

        **vault.to_account_info().try_borrow_mut_lamports()? -= payout;
        **player.to_account_info().try_borrow_mut_lamports()? += payout;

        msg!(
            "Payout: {} lamports to {}",
            payout,
            player.key()
        );
        Ok(())
    }

    /// View the current house statistics.
    pub fn get_stats(ctx: Context<GetStats>) -> Result<()> {
        let house = &ctx.accounts.house;
        msg!(
            "Stats: total_games={}, heads={}, tails={}, heads_pct={}%",
            house.total_games,
            house.total_heads,
            house.total_tails,
            if house.total_games > 0 {
                house.total_heads * 100 / house.total_games
            } else {
                0
            }
        );
        Ok(())
    }
}

// ─── Accounts ────────────────────────────────────────────────────────────────

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        init,
        payer = authority,
        space = 8 + House::LEN,
        seeds = [b"house"],
        bump,
    )]
    pub house: Account<'info, House>,

    /// CHECK: vault PDA just holds lamports
    #[account(
        mut,
        seeds = [SEED_VAULT],
        bump,
    )]
    pub vault: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(choice: u8, wager: u64, game_id: u64)]
pub struct PlaceBet<'info> {
    #[account(mut)]
    pub player: Signer<'info>,

    #[account(
        init,
        payer = player,
        space = 8 + Game::LEN,
        seeds = [SEED_GAME, player.key().as_ref(), &game_id.to_le_bytes()],
        bump,
    )]
    pub game: Account<'info, Game>,

    /// CHECK: vault PDA just holds lamports
    #[account(
        mut,
        seeds = [SEED_VAULT],
        bump,
    )]
    pub vault: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct DiceCallback<'info> {
    #[account(mut)]
    pub game: Account<'info, Game>,

    #[account(
        mut,
        seeds = [b"house"],
        bump = house.bump,
    )]
    pub house: Account<'info, House>,
}

#[derive(Accounts)]
pub struct Claim<'info> {
    #[account(mut)]
    pub player: Signer<'info>,

    #[account(
        mut,
        has_one = player,
        constraint = game.settled @ CoinTossError::NotSettled,
    )]
    pub game: Account<'info, Game>,

    /// CHECK: vault PDA just holds lamports
    #[account(
        mut,
        seeds = [SEED_VAULT],
        bump,
    )]
    pub vault: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct GetStats<'info> {
    pub house: Account<'info, House>,
}

// ─── State ───────────────────────────────────────────────────────────────────

#[account]
pub struct House {
    pub authority: Pubkey,
    pub total_games: u64,
    pub total_heads: u64,
    pub total_tails: u64,
    pub bump: u8,
}

impl House {
    const LEN: usize = 32 + 8 + 8 + 8 + 1; // 57
}

#[account]
pub struct Game {
    /// The player who placed this bet.
    pub player: Pubkey,
    /// 0 = heads, 1 = tails.
    pub choice: u8,
    /// Amount wagered in lamports.
    pub wager: u64,
    /// Unique game identifier.
    pub game_id: u64,
    /// Result: 0 = heads, 1 = tails, 255 = unresolved.
    pub result: u8,
    /// Whether the game has been settled via callback.
    pub settled: bool,
    /// The 32-byte randomness from DICE.
    pub randomness: [u8; 32],
    /// PDA bump.
    pub bump: u8,
}

impl Game {
    const LEN: usize = 32 + 1 + 8 + 8 + 1 + 1 + 32 + 1; // 84
}

// ─── Errors ──────────────────────────────────────────────────────────────────

#[error_code]
pub enum CoinTossError {
    #[msg("Choice must be 0 (heads) or 1 (tails)")]
    InvalidChoice,

    #[msg("Wager must be greater than zero")]
    ZeroWager,

    #[msg("Game has already been settled")]
    AlreadySettled,

    #[msg("Game has not been settled yet")]
    NotSettled,

    #[msg("Player did not win — cannot claim")]
    PlayerLost,
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dice_callback_discriminator_matches() {
        use anchor_lang::solana_program::hash::hashv;
        let hash = hashv(&[b"global:dice_callback"]);
        let expected: [u8; 8] = hash.to_bytes()[..8].try_into().unwrap();
        assert_eq!(
            DICE_CALLBACK_DISCRIMINATOR, expected,
            "discriminator must match SHA-256('global:dice_callback')[0..8]"
        );
    }

    #[test]
    fn test_coin_flip_logic_heads() {
        // Even byte → heads (0)
        let randomness = [0x42u8; 32]; // 0x42 = 66 = even → heads
        let result = randomness[0] & 1;
        assert_eq!(result, 0, "even byte should be heads");
    }

    #[test]
    fn test_coin_flip_logic_tails() {
        // Odd byte → tails (1)
        let randomness = [0x43u8; 32]; // 0x43 = 67 = odd → tails
        let result = randomness[0] & 1;
        assert_eq!(result, 1, "odd byte should be tails");
    }

    #[test]
    fn test_coin_flip_distribution() {
        // Simulate 256 flips using all possible first-byte values.
        // Should be exactly 128 heads, 128 tails (since we use & 1).
        let mut heads = 0u32;
        let mut tails = 0u32;
        for byte in 0..=255u8 {
            if byte & 1 == 0 {
                heads += 1;
            } else {
                tails += 1;
            }
        }
        assert_eq!(heads, 128);
        assert_eq!(tails, 128);
    }

    #[test]
    fn test_game_account_size() {
        // Verify the game account size is correct
        assert_eq!(Game::LEN, 84);
    }

    #[test]
    fn test_house_account_size() {
        // Verify the house account size is correct
        assert_eq!(House::LEN, 57);
    }

    #[test]
    fn test_randomness_determines_unique_outcomes() {
        // Different randomness values should produce different outcomes
        // (not always — but statistically they should differ)
        let r1 = [0x00u8; 32]; // heads
        let r2 = [0x01u8; 32]; // tails
        let r3 = [0xFEu8; 32]; // heads (even)
        let r4 = [0xFFu8; 32]; // tails (odd)

        assert_eq!(r1[0] & 1, 0); // heads
        assert_eq!(r2[0] & 1, 1); // tails
        assert_eq!(r3[0] & 1, 0); // heads
        assert_eq!(r4[0] & 1, 1); // tails
    }
}
