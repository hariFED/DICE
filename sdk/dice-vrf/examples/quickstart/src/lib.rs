// ============================================================================
// DICE VRF — Plug-and-play integration template
// ============================================================================
//
// THIS FILE IS A COMPLETE, COMPILABLE dApp SKELETON. Three things to change:
//
//   1. declare_id!(...) below — replace with your own program ID.
//   2. OUTCOME_FORMULA (constant below) — encode how randomness maps to your
//      game's outcome (dice, spin, flip, pick-winner, whatever).
//   3. GameState struct — store whatever you want to persist per round.
//
// Everything else — the DICE wiring, the callback handler, the discriminator
// check, the account glue — stays as-is.
//
// ------------ End-to-end flow ------------------------------------------------
//
//   Player TX   →   play() on this program
//                   ├── creates a GameState PDA (stores bet)
//                   ├── transfers wager into vault
//                   └── CPI → dice::request_randomness_auto()
//                              │
//                              ↓
//                   [DICE coord dispatches round to 4 ESP32-S3 devices,
//                    commit-reveal runs, single-shot submit_round_v2 TX
//                    finalizes randomness, auto-Idles the channel]
//                              │
//                              ↓
//   Coord TX    →   dice::deliver_callback() on DICE program
//                   └── CPI → dice_callback() on THIS program
//                              ├── reads `randomness: [u8; 32]` from ix data
//                              ├── computes outcome via OUTCOME_FORMULA
//                              ├── mutates GameState
//                              └── done.
//
//   Player TX   →   claim() on this program  (only if they won)
//                   └── vault → player.
//
// -----------------------------------------------------------------------------

use anchor_lang::prelude::*;

// ── 1. YOUR PROGRAM ID ───────────────────────────────────────────────────────
// Replace with the pubkey from your own keypair (target/deploy/<your>-keypair.json).
declare_id!("1111111111111111111111111111111111111111111");

// ── DICE's callback discriminator — DO NOT CHANGE ────────────────────────────
// SHA-256("global:dice_callback")[0..8]. DICE's deliver_callback CPIs into our
// program with this as the ix discriminator; Anchor's dispatch routes it to
// the `dice_callback` handler below.
const _DICE_CALLBACK_DISCRIMINATOR: [u8; 8] = [128, 131, 129, 45, 53, 113, 215, 151];

#[program]
pub mod dice_vrf_quickstart {
    use super::*;

    // ── Player entry point — ONE TX from the client ─────────────────────────
    //
    // The client sends a single TX calling this. Inside, we:
    //   (a) set up your game state,
    //   (b) transfer the wager,
    //   (c) CPI into DICE to request randomness.
    //
    // A few seconds later, DICE's coordinator will CPI into `dice_callback`
    // below with the result. You don't have to do anything else client-side
    // except poll the GameState PDA (or accountSubscribe to it) until
    // `settled == true`.
    pub fn play(ctx: Context<Play>, bet: u8, wager: u64, round_nonce: u64) -> Result<()> {
        // (a) Persist the bet.
        let g = &mut ctx.accounts.game;
        g.player       = ctx.accounts.player.key();
        g.bet          = bet;
        g.wager        = wager;
        g.round_nonce  = round_nonce;
        g.outcome      = 0;
        g.settled      = false;
        g.randomness   = [0u8; 32];
        g.bump         = ctx.bumps.game;

        // (b) Take the wager.
        anchor_lang::system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                anchor_lang::system_program::Transfer {
                    from: ctx.accounts.player.to_account_info(),
                    to:   ctx.accounts.vault.to_account_info(),
                },
            ),
            wager,
        )?;

        // (c) CPI into DICE. ONE LINE OF "OH SO THAT'S THE INTEGRATION".
        //     DICE will run the commit-reveal protocol and eventually call
        //     our `dice_callback` handler below.
        let dice_accounts = dice::cpi::accounts::RequestRandomnessAuto {
            authority:      ctx.accounts.player.to_account_info(),
            channel:        ctx.accounts.dice_channel.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
        };
        dice::cpi::request_randomness_auto(
            CpiContext::new(ctx.accounts.dice_program.to_account_info(), dice_accounts),
            4, // 4 nodes — change to taste (min 4, max 50 per DICE constants)
        )?;

        Ok(())
    }

    // ── DICE's callback lands here — YOU WRITE THE FORMULA ──────────────────
    //
    // Instruction data layout (set by DICE's deliver_callback):
    //   [0..8]   DICE_CALLBACK_DISCRIMINATOR (Anchor routes to this fn by name)
    //   [8..40]  channel_key (Pubkey — the DiceChannel that produced the result)
    //   [40..72] randomness ([u8; 32] — hardware-backed random bytes)
    //
    // The game PDA is passed as remaining_accounts[0] by the coordinator.
    pub fn dice_callback(
        ctx: Context<DiceVrfCallback>,
        _channel_key: Pubkey,
        randomness: [u8; 32],
    ) -> Result<()> {
        let g = &mut ctx.accounts.game;
        require!(!g.settled, QuickstartError::AlreadySettled);

        // ── 2. YOUR OUTCOME FORMULA ─────────────────────────────────────────
        // Examples:
        //   Dice (1-6):        (u32::from_le_bytes(rand[0..4]) % 6) as u8 + 1
        //   Coin flip (0/1):    randomness[0] & 1
        //   Weighted wheel:     walk segment_weights until sum > (rand[0..4] % total_weight)
        //   Lottery winner:     randomness[0..8] as u64 % num_players
        //
        // Here: trivial 1-6 dice.
        let u32_rand = u32::from_le_bytes([
            randomness[0], randomness[1], randomness[2], randomness[3],
        ]);
        let outcome = (u32_rand % 6) as u8 + 1;

        g.outcome     = outcome;
        g.settled     = true;
        g.randomness  = randomness;

        let won = outcome == g.bet;
        msg!("Outcome={}, bet={}, {}!", outcome, g.bet, if won { "WIN" } else { "LOSE" });

        Ok(())
    }

    // ── Player claims winnings ──────────────────────────────────────────────
    pub fn claim(ctx: Context<Claim>) -> Result<()> {
        let g = &ctx.accounts.game;
        require!(g.settled,               QuickstartError::NotSettled);
        require!(g.outcome == g.bet,      QuickstartError::DidNotWin);

        // Payout = 5x wager (1-in-6 odds → 5x is the fair multiplier; tune to
        // your game's house edge). Lamports moved directly between PDAs we
        // own; no system_program CPI needed since both are program-owned.
        let payout = g.wager.checked_mul(5).ok_or(QuickstartError::Overflow)?;
        **ctx.accounts.vault.to_account_info().try_borrow_mut_lamports()? -= payout;
        **ctx.accounts.player.to_account_info().try_borrow_mut_lamports()? += payout;
        Ok(())
    }
}

// ─── Accounts ───────────────────────────────────────────────────────────────

#[derive(Accounts)]
#[instruction(bet: u8, wager: u64, round_nonce: u64)]
pub struct Play<'info> {
    #[account(mut)]
    pub player: Signer<'info>,

    // ── 3. YOUR GAME STATE ──────────────────────────────────────────────────
    // This is the PDA the `dice_callback` handler will mutate. Shape it any
    // way you want — just keep the `bump` field and the Anchor account macros.
    #[account(
        init, payer = player,
        space = 8 + GameState::LEN,
        seeds = [b"game", player.key().as_ref(), &round_nonce.to_le_bytes()],
        bump,
    )]
    pub game: Account<'info, GameState>,

    /// CHECK: vault PDA holds lamports for payouts.
    #[account(mut, seeds = [b"vault"], bump)]
    pub vault: AccountInfo<'info>,

    // ── DICE wiring — three accounts ────────────────────────────────────────
    /// CHECK: the DiceChannel PDA owned by the DICE program. Created once via
    /// `init_channel(callback_program_id = <THIS_PROGRAM_ID>)`.
    #[account(mut)]
    pub dice_channel: AccountInfo<'info>,
    /// CHECK: DICE program — `FMwPuCjkfZXN2MuNJQiUzZC3hnxHcD8mrTuntsqA84XD`.
    pub dice_program: Program<'info, dice::program::Dice>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct DiceVrfCallback<'info> {
    /// The game PDA the coordinator passes in remaining_accounts.
    /// Anchor routes on discriminator so DICE's deliver_callback lands here.
    #[account(mut)]
    pub game: Account<'info, GameState>,
}

#[derive(Accounts)]
pub struct Claim<'info> {
    #[account(mut)]
    pub player: Signer<'info>,

    #[account(
        mut,
        has_one  = player,
        constraint = game.settled @ QuickstartError::NotSettled,
    )]
    pub game: Account<'info, GameState>,

    /// CHECK: vault PDA.
    #[account(mut, seeds = [b"vault"], bump)]
    pub vault: AccountInfo<'info>,
}

// ─── State ──────────────────────────────────────────────────────────────────

#[account]
pub struct GameState {
    pub player:       Pubkey,
    pub bet:          u8,
    pub wager:        u64,
    pub round_nonce:  u64,
    pub outcome:      u8,
    pub settled:      bool,
    pub randomness:   [u8; 32],
    pub bump:         u8,
}

impl GameState {
    const LEN: usize = 32 + 1 + 8 + 8 + 1 + 1 + 32 + 1;
}

#[error_code]
pub enum QuickstartError {
    #[msg("Game already settled")]
    AlreadySettled,
    #[msg("Game not yet settled")]
    NotSettled,
    #[msg("Player did not win")]
    DidNotWin,
    #[msg("Payout overflow")]
    Overflow,
}
