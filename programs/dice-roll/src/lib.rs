// ============================================================================
// DICE ROLL — Provably Fair Dice Game using DICE VRF
// ============================================================================
//
// A classic dice game where players bet on the outcome of a 1-6 roll.
// The result is determined by hardware-backed VRF from DICE oracle nodes.
//
// ## How DICE VRF Integration Works
//
// 1. Player calls `roll_dice` with their bet (number 1-6) and wager
// 2. The program creates a Roll account and stores the bet
// 3. Coordinator detects the on-chain request and dispatches to ESP32 nodes
// 4. ESP32 generates hardware entropy → commit → reveal → finalize
// 5. DICE program calls this program's `dice_callback` via CPI
// 6. The 32-byte randomness is reduced to 1-6 using modular arithmetic
// 7. If player guessed correctly, they win 5x their wager
//
// ## VRF Flow Diagram
//
//   Player                    Dice Roll Program           DICE VRF Oracle
//     |                            |                           |
//     |-- roll_dice(bet=3) ------->|                           |
//     |                            |-- request_randomness ---->|
//     |                            |                           |
//     |                            |     [ESP32 nodes run      |
//     |                            |      commit-reveal        |
//     |                            |      protocol]            |
//     |                            |                           |
//     |                            |<-- dice_callback(rand) ---|
//     |                            |                           |
//     |                            | result = (rand[0] % 6) + 1
//     |                            | if result == bet: WIN 5x  |
//     |                            |                           |
//     |<-- claim() (if won) -------|                           |

use anchor_lang::prelude::*;

declare_id!("C1UBkxcNM5g5kfJFDPCFkXVXAgrxsLksFoDTamk6XqJu");

const DICE_CALLBACK_DISCRIMINATOR: [u8; 8] = [128, 131, 129, 45, 53, 113, 215, 151];

#[program]
pub mod dice_roll {
    use super::*;

    /// Initialize the house. Creates a vault to hold wagers.
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let house = &mut ctx.accounts.house;
        house.authority = ctx.accounts.authority.key();
        house.total_rolls = 0;
        house.roll_distribution = [0u64; 6]; // track how many times each number appeared
        house.bump = ctx.bumps.house;
        msg!("Dice Roll house initialized");
        Ok(())
    }

    /// Player places a bet on a dice roll outcome (1-6).
    ///
    /// ## Parameters
    /// - `bet`: The number the player is betting on (1-6)
    /// - `wager`: Amount in lamports to bet
    /// - `roll_id`: Unique identifier for this roll (prevents duplicate bets)
    ///
    /// ## VRF Integration Point
    /// After this call, the coordinator detects the randomness request on-chain
    /// and dispatches a JobAssignment to ESP32-S3 hardware nodes.
    pub fn roll_dice(
        ctx: Context<RollDice>,
        bet: u8,
        wager: u64,
        roll_id: u64,
    ) -> Result<()> {
        require!(bet >= 1 && bet <= 6, DiceRollError::InvalidBet);
        require!(wager > 0, DiceRollError::ZeroWager);

        let roll = &mut ctx.accounts.roll;
        roll.player = ctx.accounts.player.key();
        roll.bet = bet;
        roll.wager = wager;
        roll.roll_id = roll_id;
        roll.result = 0;
        roll.settled = false;
        roll.randomness = [0u8; 32];
        roll.bump = ctx.bumps.roll;

        // Transfer wager to vault
        let cpi_ctx = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            anchor_lang::system_program::Transfer {
                from: ctx.accounts.player.to_account_info(),
                to: ctx.accounts.vault.to_account_info(),
            },
        );
        anchor_lang::system_program::transfer(cpi_ctx, wager)?;

        msg!(
            "Dice roll: player={}, bet={}, wager={} lamports, roll_id={}",
            roll.player, bet, wager, roll_id
        );
        Ok(())
    }

    /// Called by DICE VRF after hardware nodes generate randomness.
    ///
    /// ## How the randomness becomes a dice roll
    ///
    /// The 32-byte randomness from ESP32 hardware entropy is converted to
    /// a dice roll (1-6) using the first byte:
    ///
    /// ```text
    /// result = (randomness[0] % 6) + 1
    /// ```
    ///
    /// This produces a uniform distribution because:
    /// - randomness[0] is uniformly distributed in [0, 255]
    /// - 256 % 6 = 4, so values 0-3 have slightly higher probability
    /// - Bias is 4/256 = 1.56% — negligible for gaming
    /// - For zero-bias: use u32 from first 4 bytes, rejection sampling
    ///
    /// ## Instruction Data Layout (from DICE)
    /// ```text
    /// [0..8]   DICE_CALLBACK_DISCRIMINATOR
    /// [8..40]  channel_key: Pubkey (the DiceChannel that produced the result)
    /// [40..72] randomness: [u8; 32] (hardware-generated random bytes)
    /// ```
    pub fn dice_callback(
        ctx: Context<DiceRollCallback>,
        _channel_key: Pubkey,
        randomness: [u8; 32],
    ) -> Result<()> {
        let roll = &mut ctx.accounts.roll;
        require!(!roll.settled, DiceRollError::AlreadySettled);

        // Convert randomness to dice roll (1-6)
        // Use first 4 bytes as u32 for better distribution
        let rand_u32 = u32::from_le_bytes([randomness[0], randomness[1], randomness[2], randomness[3]]);
        let result = (rand_u32 % 6) as u8 + 1;

        roll.randomness = randomness;
        roll.result = result;
        roll.settled = true;

        let won = result == roll.bet;

        // Update house stats
        let house = &mut ctx.accounts.house;
        house.total_rolls += 1;
        house.roll_distribution[(result - 1) as usize] += 1;

        msg!(
            "Dice rolled: {} (raw: 0x{:08x}). Player bet {}. {}! Payout: {} lamports",
            result,
            rand_u32,
            roll.bet,
            if won { "WIN" } else { "LOSE" },
            if won { roll.wager * 5 } else { 0 }
        );

        Ok(())
    }

    /// Player claims winnings (5x wager) after a winning roll.
    pub fn claim(ctx: Context<ClaimRoll>) -> Result<()> {
        let roll = &ctx.accounts.roll;
        require!(roll.settled, DiceRollError::NotSettled);
        require!(roll.result == roll.bet, DiceRollError::PlayerLost);

        let payout = roll.wager * 5; // 1-in-6 chance → 5x payout (house edge: 16.7%)

        **ctx.accounts.vault.to_account_info().try_borrow_mut_lamports()? -= payout;
        **ctx.accounts.player.to_account_info().try_borrow_mut_lamports()? += payout;

        msg!("Payout: {} lamports to {}", payout, ctx.accounts.player.key());
        Ok(())
    }
}

// ─── Accounts ───────────────────────────────────────────────────────────────

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        init, payer = authority,
        space = 8 + House::LEN,
        seeds = [b"house"], bump,
    )]
    pub house: Account<'info, House>,

    /// CHECK: vault PDA holds lamports
    #[account(mut, seeds = [b"vault"], bump)]
    pub vault: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(bet: u8, wager: u64, roll_id: u64)]
pub struct RollDice<'info> {
    #[account(mut)]
    pub player: Signer<'info>,

    #[account(
        init, payer = player,
        space = 8 + Roll::LEN,
        seeds = [b"roll", player.key().as_ref(), &roll_id.to_le_bytes()],
        bump,
    )]
    pub roll: Account<'info, Roll>,

    /// CHECK: vault PDA holds lamports
    #[account(mut, seeds = [b"vault"], bump)]
    pub vault: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct DiceRollCallback<'info> {
    #[account(mut)]
    pub roll: Account<'info, Roll>,

    #[account(mut, seeds = [b"house"], bump = house.bump)]
    pub house: Account<'info, House>,
}

#[derive(Accounts)]
pub struct ClaimRoll<'info> {
    #[account(mut)]
    pub player: Signer<'info>,

    #[account(mut, has_one = player, constraint = roll.settled @ DiceRollError::NotSettled)]
    pub roll: Account<'info, Roll>,

    /// CHECK: vault PDA holds lamports
    #[account(mut, seeds = [b"vault"], bump)]
    pub vault: AccountInfo<'info>,
}

// ─── State ──────────────────────────────────────────────────────────────────

#[account]
pub struct House {
    pub authority: Pubkey,
    pub total_rolls: u64,
    pub roll_distribution: [u64; 6], // [count_1, count_2, ..., count_6]
    pub bump: u8,
}

impl House {
    const LEN: usize = 32 + 8 + (6 * 8) + 1; // 89
}

#[account]
pub struct Roll {
    pub player: Pubkey,
    pub bet: u8,        // 1-6
    pub wager: u64,
    pub roll_id: u64,
    pub result: u8,     // 1-6 (0 = unresolved)
    pub settled: bool,
    pub randomness: [u8; 32],
    pub bump: u8,
}

impl Roll {
    const LEN: usize = 32 + 1 + 8 + 8 + 1 + 1 + 32 + 1; // 84
}

// ─── Errors ─────────────────────────────────────────────────────────────────

#[error_code]
pub enum DiceRollError {
    #[msg("Bet must be between 1 and 6")]
    InvalidBet,
    #[msg("Wager must be greater than zero")]
    ZeroWager,
    #[msg("Roll has already been settled")]
    AlreadySettled,
    #[msg("Roll has not been settled yet")]
    NotSettled,
    #[msg("Player did not win")]
    PlayerLost,
}
