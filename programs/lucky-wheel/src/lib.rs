// ============================================================================
// LUCKY WHEEL — Provably Fair Spin-to-Win using DICE VRF
// ============================================================================
//
// A lucky wheel game with configurable weighted segments. The house owner
// defines segments (name + multiplier + weight), players pay to spin,
// and DICE VRF determines which segment the wheel lands on.
//
// ## How DICE VRF Integration Works
//
// 1. House owner calls `create_wheel` with segment definitions
//    Example: [("2x", 2x, 40%), ("5x", 5x, 25%), ("10x", 10x, 10%),
//              ("0x", 0x, 20%), ("JACKPOT", 50x, 5%)]
//
// 2. Player calls `spin` with their wager amount
//
// 3. DICE VRF is requested → ESP32 hardware nodes generate entropy
//
// 4. `dice_callback` receives 32 bytes of randomness
//
// 5. Segment selection algorithm:
//    a. Convert first 4 bytes to u32 (uniform random)
//    b. Map to range [0, total_weight) using modular arithmetic
//    c. Walk segments, accumulating weights until the random point is reached
//    d. The segment where the point falls is the winner
//
// ## VRF Flow Diagram
//
//   Player                   Lucky Wheel Program         DICE VRF Oracle
//     |                            |                           |
//     |-- spin(wager=1000) ------->|                           |
//     |                            |-- request_randomness ---->|
//     |                            |                           |
//     |                            |   [ESP32-S3 hardware      |
//     |                            |    TRNG + ADC noise       |
//     |                            |    + timing jitter]       |
//     |                            |                           |
//     |                            |<-- dice_callback(rand) ---|
//     |                            |                           |
//     |                            | rand_u32 = u32(rand[0..4])
//     |                            | point = rand_u32 % total_weight
//     |                            |                           |
//     |                            | Segments:                 |
//     |                            | [0..40)   → 2x  (40%)    |
//     |                            | [40..65)  → 5x  (25%)    |
//     |                            | [65..75)  → 10x (10%)    |
//     |                            | [75..95)  → 0x  (20%)    |
//     |                            | [95..100) → 50x (5%)     |
//     |                            |                           |
//     |                            | point=72 → 10x segment!  |
//     |                            |                           |
//     |<-- claim() (payout) -------|                           |

use anchor_lang::prelude::*;

declare_id!("2P8i6supnfNb2LYg4Dx3aSaJmjqw2fo8y9TtJF57fn82");

const DICE_CALLBACK_DISCRIMINATOR: [u8; 8] = [128, 131, 129, 45, 53, 113, 215, 151];
const MAX_SEGMENTS: usize = 10;

#[program]
pub mod lucky_wheel {
    use super::*;

    /// Create a wheel with weighted segments.
    ///
    /// ## Parameters
    /// - `segment_weights`: Weight of each segment (must sum to > 0)
    /// - `segment_multipliers`: Payout multiplier for each segment (in basis points, 100 = 1x)
    /// - `segment_count`: Number of active segments (max 10)
    ///
    /// ## Example
    /// ```text
    /// weights:      [40, 25, 10, 20, 5]     → total = 100
    /// multipliers:  [200, 500, 1000, 0, 5000] → 2x, 5x, 10x, 0x, 50x
    /// ```
    pub fn create_wheel(
        ctx: Context<CreateWheel>,
        segment_weights: [u16; MAX_SEGMENTS],
        segment_multipliers: [u16; MAX_SEGMENTS],
        segment_count: u8,
    ) -> Result<()> {
        require!(segment_count >= 2 && segment_count as usize <= MAX_SEGMENTS, WheelError::InvalidSegmentCount);

        let total_weight: u32 = segment_weights.iter()
            .take(segment_count as usize)
            .map(|w| *w as u32)
            .sum();
        require!(total_weight > 0, WheelError::ZeroTotalWeight);

        let wheel = &mut ctx.accounts.wheel;
        wheel.authority = ctx.accounts.authority.key();
        wheel.segment_weights = segment_weights;
        wheel.segment_multipliers = segment_multipliers;
        wheel.segment_count = segment_count;
        wheel.total_weight = total_weight as u16;
        wheel.total_spins = 0;
        wheel.bump = ctx.bumps.wheel;

        msg!("Wheel created: {} segments, total_weight={}", segment_count, total_weight);
        Ok(())
    }

    /// Player spins the wheel. Wager is held until VRF resolves.
    pub fn spin(
        ctx: Context<Spin>,
        wager: u64,
        spin_id: u64,
    ) -> Result<()> {
        require!(wager > 0, WheelError::ZeroWager);

        let spin = &mut ctx.accounts.spin_account;
        spin.player = ctx.accounts.player.key();
        spin.wager = wager;
        spin.spin_id = spin_id;
        spin.segment_index = 255; // unresolved
        spin.multiplier = 0;
        spin.payout = 0;
        spin.settled = false;
        spin.randomness = [0u8; 32];
        spin.bump = ctx.bumps.spin_account;

        // Transfer wager to vault
        let cpi_ctx = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            anchor_lang::system_program::Transfer {
                from: ctx.accounts.player.to_account_info(),
                to: ctx.accounts.vault.to_account_info(),
            },
        );
        anchor_lang::system_program::transfer(cpi_ctx, wager)?;

        msg!("Spin placed: player={}, wager={}, spin_id={}", spin.player, wager, spin_id);
        Ok(())
    }

    /// Called by DICE VRF with hardware-generated randomness.
    ///
    /// ## Segment Selection Algorithm
    ///
    /// ```text
    /// 1. rand_u32 = u32::from_le_bytes(randomness[0..4])
    /// 2. point = rand_u32 % total_weight
    /// 3. accumulated = 0
    /// 4. for each segment i:
    ///      accumulated += weight[i]
    ///      if point < accumulated:
    ///        winner = segment i
    ///        break
    /// ```
    ///
    /// This produces a weighted random selection where the probability
    /// of landing on segment `i` is `weight[i] / total_weight`.
    pub fn dice_callback(
        ctx: Context<WheelCallback>,
        _channel_key: Pubkey,
        randomness: [u8; 32],
    ) -> Result<()> {
        let wheel = &ctx.accounts.wheel;
        let total_weight = wheel.total_weight;
        let segment_count = wheel.segment_count;
        let segment_weights = wheel.segment_weights;
        let segment_multipliers = wheel.segment_multipliers;

        let spin = &mut ctx.accounts.spin_account;
        require!(!spin.settled, WheelError::AlreadySettled);

        // Convert randomness to selection point
        let rand_u32 = u32::from_le_bytes([randomness[0], randomness[1], randomness[2], randomness[3]]);
        let point = rand_u32 % (total_weight as u32);

        // Walk segments to find winner
        let mut accumulated: u32 = 0;
        let mut winner_index: u8 = 0;
        let mut winner_multiplier: u16 = 0;

        for i in 0..segment_count as usize {
            accumulated += segment_weights[i] as u32;
            if point < accumulated {
                winner_index = i as u8;
                winner_multiplier = segment_multipliers[i];
                break;
            }
        }

        // Calculate payout: wager * multiplier / 100
        let payout = (spin.wager as u128 * winner_multiplier as u128 / 100) as u64;

        spin.randomness = randomness;
        spin.segment_index = winner_index;
        spin.multiplier = winner_multiplier;
        spin.payout = payout;
        spin.settled = true;

        // Update wheel stats
        let wheel_mut = &mut ctx.accounts.wheel;
        wheel_mut.total_spins += 1;

        msg!(
            "Wheel landed on segment {} (multiplier={}). Raw: 0x{:08x}, point={}/{}, payout={} lamports",
            winner_index,
            winner_multiplier,
            rand_u32,
            point,
            total_weight,
            payout
        );

        Ok(())
    }

    /// Player claims payout after the spin is settled.
    pub fn claim(ctx: Context<ClaimSpin>) -> Result<()> {
        let spin = &ctx.accounts.spin_account;
        require!(spin.settled, WheelError::NotSettled);
        require!(spin.payout > 0, WheelError::NoPayout);

        **ctx.accounts.vault.to_account_info().try_borrow_mut_lamports()? -= spin.payout;
        **ctx.accounts.player.to_account_info().try_borrow_mut_lamports()? += spin.payout;

        msg!("Payout: {} lamports to {}", spin.payout, ctx.accounts.player.key());
        Ok(())
    }
}

// ─── Accounts ───────────────────────────────────────────────────────────────

#[derive(Accounts)]
pub struct CreateWheel<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        init, payer = authority,
        space = 8 + Wheel::LEN,
        seeds = [b"wheel"], bump,
    )]
    pub wheel: Account<'info, Wheel>,

    /// CHECK: vault PDA holds lamports
    #[account(mut, seeds = [b"vault"], bump)]
    pub vault: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(wager: u64, spin_id: u64)]
pub struct Spin<'info> {
    #[account(mut)]
    pub player: Signer<'info>,

    #[account(
        init, payer = player,
        space = 8 + SpinAccount::LEN,
        seeds = [b"spin", player.key().as_ref(), &spin_id.to_le_bytes()],
        bump,
    )]
    pub spin_account: Account<'info, SpinAccount>,

    /// CHECK: vault PDA holds lamports
    #[account(mut, seeds = [b"vault"], bump)]
    pub vault: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct WheelCallback<'info> {
    #[account(mut)]
    pub spin_account: Account<'info, SpinAccount>,

    #[account(mut, seeds = [b"wheel"], bump = wheel.bump)]
    pub wheel: Account<'info, Wheel>,
}

#[derive(Accounts)]
pub struct ClaimSpin<'info> {
    #[account(mut)]
    pub player: Signer<'info>,

    #[account(mut, has_one = player, constraint = spin_account.settled @ WheelError::NotSettled)]
    pub spin_account: Account<'info, SpinAccount>,

    /// CHECK: vault PDA holds lamports
    #[account(mut, seeds = [b"vault"], bump)]
    pub vault: AccountInfo<'info>,
}

// ─── State ──────────────────────────────────────────────────────────────────

#[account]
pub struct Wheel {
    pub authority: Pubkey,
    pub segment_weights: [u16; MAX_SEGMENTS],       // weight per segment
    pub segment_multipliers: [u16; MAX_SEGMENTS],   // payout multiplier (basis points, 100 = 1x)
    pub segment_count: u8,
    pub total_weight: u16,
    pub total_spins: u64,
    pub bump: u8,
}

impl Wheel {
    const LEN: usize = 32 + (10 * 2) + (10 * 2) + 1 + 2 + 8 + 1; // 84
}

#[account]
pub struct SpinAccount {
    pub player: Pubkey,
    pub wager: u64,
    pub spin_id: u64,
    pub segment_index: u8,   // winning segment (255 = unresolved)
    pub multiplier: u16,     // winning multiplier (basis points)
    pub payout: u64,         // calculated payout in lamports
    pub settled: bool,
    pub randomness: [u8; 32],
    pub bump: u8,
}

impl SpinAccount {
    const LEN: usize = 32 + 8 + 8 + 1 + 2 + 8 + 1 + 32 + 1; // 93
}

// ─── Errors ─────────────────────────────────────────────────────────────────

#[error_code]
pub enum WheelError {
    #[msg("Segment count must be 2-10")]
    InvalidSegmentCount,
    #[msg("Total weight must be > 0")]
    ZeroTotalWeight,
    #[msg("Wager must be > 0")]
    ZeroWager,
    #[msg("Spin already settled")]
    AlreadySettled,
    #[msg("Spin not settled yet")]
    NotSettled,
    #[msg("No payout (0x multiplier)")]
    NoPayout,
}
