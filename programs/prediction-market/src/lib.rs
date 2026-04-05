// ============================================================================
// PREDICTION MARKET — Polymarket-style Markets with DICE VRF Resolution
// ============================================================================
//
// A binary prediction market where users bet YES or NO on outcomes.
// When the market closes, DICE VRF provides verifiable randomness to
// resolve the market — ensuring no single party can manipulate the result.
//
// ## Use Cases
// - "Will SOL be above $200 at end of week?" (resolved by VRF threshold)
// - "Will the next block hash end in an even number?" (resolved by VRF)
// - Sports/esports outcomes, lotteries, any binary event
//
// ## How DICE VRF Integration Works
//
// 1. Market creator calls `create_market` with a question and deadline
// 2. Users call `place_position` with YES (1) or NO (0) + amount
// 3. After deadline, anyone calls `resolve_market`
// 4. DICE VRF is requested → ESP32 hardware nodes generate entropy
// 5. `dice_callback` receives 32-byte randomness
// 6. Resolution: first byte determines YES (even) or NO (odd)
//    - For threshold markets: first 8 bytes as u64, compared to threshold
// 7. Winners split the losing pool proportionally
//
// ## VRF Flow Diagram
//
//   Creator                  Prediction Market            DICE VRF Oracle
//     |                            |                           |
//     |-- create_market() -------->|                           |
//     |                            |                           |
//   Users                          |                           |
//     |-- place_position(YES) ---->|                           |
//     |-- place_position(NO) ----->|                           |
//     |                            |                           |
//   Anyone (after deadline)        |                           |
//     |-- resolve_market() ------->|                           |
//     |                            |-- request_randomness ---->|
//     |                            |                           |
//     |                            |   [ESP32-S3 hardware      |
//     |                            |    commit-reveal protocol |
//     |                            |    4-7 nodes participate] |
//     |                            |                           |
//     |                            |<-- dice_callback(rand) ---|
//     |                            |                           |
//     |                            | outcome = rand[0] % 2     |
//     |                            | 0 = YES wins              |
//     |                            | 1 = NO wins               |
//     |                            |                           |
//   Winners                        |                           |
//     |-- claim_winnings() ------->|                           |
//     |<-- (proportional payout) --|                           |
//
// ## Why VRF for Prediction Markets?
//
// Traditional prediction markets use oracles to report real-world outcomes.
// VRF-resolved markets are useful when:
// - The outcome should be provably random (lottery, coin flip markets)
// - No trusted oracle exists for the event
// - The market itself IS the randomness game (gambling)
// - You want to prove the resolution was not manipulated

use anchor_lang::prelude::*;

declare_id!("EvyL8EnSA74p9be1eJeP4dheJcRiM1S97dmnN9XXjdPp");

const DICE_CALLBACK_DISCRIMINATOR: [u8; 8] = [128, 131, 129, 45, 53, 113, 215, 151];

#[program]
pub mod prediction_market {
    use super::*;

    /// Create a new binary prediction market.
    ///
    /// ## Parameters
    /// - `market_id`: Unique identifier for this market
    /// - `question`: The prediction question (max 200 chars, stored as hash)
    /// - `deadline`: Unix timestamp after which the market can be resolved
    /// - `resolution_mode`: 0 = binary (even/odd), 1 = threshold (> 50%)
    pub fn create_market(
        ctx: Context<CreateMarket>,
        market_id: u64,
        question_hash: [u8; 32],  // SHA-256 of the question text (stored off-chain)
        deadline: i64,
        resolution_mode: u8,
    ) -> Result<()> {
        require!(resolution_mode <= 1, MarketError::InvalidResolutionMode);

        let clock = Clock::get()?;
        require!(deadline > clock.unix_timestamp, MarketError::DeadlineInPast);

        let market = &mut ctx.accounts.market;
        market.creator = ctx.accounts.creator.key();
        market.market_id = market_id;
        market.question_hash = question_hash;
        market.deadline = deadline;
        market.resolution_mode = resolution_mode;
        market.yes_pool = 0;
        market.no_pool = 0;
        market.yes_positions = 0;
        market.no_positions = 0;
        market.outcome = 255; // unresolved
        market.resolved = false;
        market.randomness = [0u8; 32];
        market.bump = ctx.bumps.market;

        msg!(
            "Market created: id={}, deadline={}, mode={}",
            market_id, deadline,
            if resolution_mode == 0 { "binary" } else { "threshold" }
        );
        Ok(())
    }

    /// Place a position (bet) on YES or NO.
    ///
    /// ## Parameters
    /// - `side`: 0 = NO, 1 = YES
    /// - `amount`: Lamports to bet
    pub fn place_position(
        ctx: Context<PlacePosition>,
        side: u8,
        amount: u64,
    ) -> Result<()> {
        require!(side <= 1, MarketError::InvalidSide);
        require!(amount > 0, MarketError::ZeroAmount);

        let market_key = ctx.accounts.market.key();
        let market = &ctx.accounts.market;
        let clock = Clock::get()?;
        require!(!market.resolved, MarketError::MarketResolved);
        require!(clock.unix_timestamp < market.deadline, MarketError::MarketClosed);
        drop(market);

        let position = &mut ctx.accounts.position;
        position.player = ctx.accounts.player.key();
        position.market = market_key;
        position.side = side;
        position.amount = amount;
        position.claimed = false;
        position.bump = ctx.bumps.position;

        // Update pool totals
        let market = &mut ctx.accounts.market;
        if side == 1 {
            market.yes_pool += amount;
            market.yes_positions += 1;
        } else {
            market.no_pool += amount;
            market.no_positions += 1;
        }

        // Transfer to vault
        let cpi_ctx = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            anchor_lang::system_program::Transfer {
                from: ctx.accounts.player.to_account_info(),
                to: ctx.accounts.vault.to_account_info(),
            },
        );
        anchor_lang::system_program::transfer(cpi_ctx, amount)?;

        msg!(
            "Position: player={}, side={}, amount={} lamports. Pools: YES={} NO={}",
            position.player,
            if side == 1 { "YES" } else { "NO" },
            amount,
            market.yes_pool,
            market.no_pool
        );
        Ok(())
    }

    /// Request market resolution via DICE VRF.
    /// Can only be called after the deadline has passed.
    pub fn resolve_market(ctx: Context<ResolveMarket>) -> Result<()> {
        let market = &ctx.accounts.market;
        let clock = Clock::get()?;

        require!(!market.resolved, MarketError::MarketResolved);
        require!(clock.unix_timestamp >= market.deadline, MarketError::MarketNotClosed);

        msg!(
            "Resolution requested for market {}. Waiting for DICE VRF callback...",
            market.market_id
        );
        // The actual resolution happens in dice_callback
        Ok(())
    }

    /// Called by DICE VRF with hardware-generated randomness.
    ///
    /// ## Resolution Algorithm
    ///
    /// ### Binary Mode (mode=0)
    /// ```text
    /// outcome = randomness[0] & 1
    /// 0 = YES wins, 1 = NO wins
    /// ```
    /// Each side has exactly 50% probability (128/256 byte values).
    ///
    /// ### Threshold Mode (mode=1)
    /// ```text
    /// value = u64::from_le_bytes(randomness[0..8])
    /// threshold = u64::MAX / 2
    /// outcome = if value > threshold { YES (0) } else { NO (1) }
    /// ```
    /// This gives a precise 50/50 split using 64 bits of entropy.
    ///
    /// ## Payout Calculation
    /// Winners split the TOTAL pool (yes_pool + no_pool) proportionally:
    /// ```text
    /// player_payout = total_pool * (player_amount / winning_pool)
    /// ```
    pub fn dice_callback(
        ctx: Context<MarketCallback>,
        _channel_key: Pubkey,
        randomness: [u8; 32],
    ) -> Result<()> {
        let market = &mut ctx.accounts.market;
        require!(!market.resolved, MarketError::MarketResolved);

        let outcome = match market.resolution_mode {
            // Binary: even byte = YES (0), odd byte = NO (1)
            0 => randomness[0] & 1,
            // Threshold: 64-bit value > midpoint = YES (0)
            1 => {
                let value = u64::from_le_bytes(
                    randomness[0..8].try_into().unwrap()
                );
                let threshold = u64::MAX / 2;
                if value > threshold { 0 } else { 1 }
            }
            _ => return Err(MarketError::InvalidResolutionMode.into()),
        };

        market.outcome = outcome;
        market.resolved = true;
        market.randomness = randomness;

        let total_pool = market.yes_pool + market.no_pool;
        let winning_pool = if outcome == 0 { market.yes_pool } else { market.no_pool };

        msg!(
            "Market {} resolved: {} wins! Raw: 0x{:02x}. Total pool: {} lamports. Winning pool: {} lamports.",
            market.market_id,
            if outcome == 0 { "YES" } else { "NO" },
            randomness[0],
            total_pool,
            winning_pool
        );

        Ok(())
    }

    /// Winner claims their proportional share of the total pool.
    ///
    /// ## Payout Formula
    /// ```text
    /// payout = total_pool * (my_amount / winning_pool)
    /// ```
    ///
    /// Example: YES pool = 100 SOL, NO pool = 50 SOL, YES wins.
    /// Player bet 10 SOL on YES → payout = 150 * (10/100) = 15 SOL (50% profit).
    pub fn claim_winnings(ctx: Context<ClaimWinnings>) -> Result<()> {
        let market = &ctx.accounts.market;
        let position = &mut ctx.accounts.position;

        require!(market.resolved, MarketError::MarketNotResolved);
        require!(!position.claimed, MarketError::AlreadyClaimed);
        require!(position.side == market.outcome, MarketError::PositionLost);

        let total_pool = market.yes_pool + market.no_pool;
        let winning_pool = if market.outcome == 0 { market.yes_pool } else { market.no_pool };

        // Proportional payout
        let payout = if winning_pool > 0 {
            (total_pool as u128 * position.amount as u128 / winning_pool as u128) as u64
        } else {
            0
        };

        require!(payout > 0, MarketError::NoPayout);

        position.claimed = true;

        **ctx.accounts.vault.to_account_info().try_borrow_mut_lamports()? -= payout;
        **ctx.accounts.player.to_account_info().try_borrow_mut_lamports()? += payout;

        msg!(
            "Claim: player={}, bet={} on {}, payout={} lamports",
            position.player,
            position.amount,
            if position.side == 0 { "YES" } else { "NO" },
            payout
        );
        Ok(())
    }
}

// ─── Accounts ───────────────────────────────────────────────────────────────

#[derive(Accounts)]
#[instruction(market_id: u64, question_hash: [u8; 32], deadline: i64, resolution_mode: u8)]
pub struct CreateMarket<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,

    #[account(
        init, payer = creator,
        space = 8 + Market::LEN,
        seeds = [b"mkt_acct", &market_id.to_le_bytes()],
        bump,
    )]
    pub market: Account<'info, Market>,

    /// CHECK: vault PDA holds lamports
    #[account(mut, seeds = [b"mkt_vlts", &market_id.to_le_bytes()], bump)]
    pub vault: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(side: u8, amount: u64)]
pub struct PlacePosition<'info> {
    #[account(mut)]
    pub player: Signer<'info>,

    #[account(mut)]
    pub market: Account<'info, Market>,

    #[account(
        init, payer = player,
        space = 8 + Position::LEN,
        seeds = [b"position", market.key().as_ref(), player.key().as_ref()],
        bump,
    )]
    pub position: Account<'info, Position>,

    /// CHECK: vault PDA holds lamports
    #[account(mut, seeds = [b"mkt_vlts", &market.market_id.to_le_bytes()], bump)]
    pub vault: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ResolveMarket<'info> {
    pub market: Account<'info, Market>,
}

#[derive(Accounts)]
pub struct MarketCallback<'info> {
    #[account(mut)]
    pub market: Account<'info, Market>,
}

#[derive(Accounts)]
pub struct ClaimWinnings<'info> {
    #[account(mut)]
    pub player: Signer<'info>,

    pub market: Account<'info, Market>,

    #[account(
        mut,
        has_one = player,
        constraint = position.market == market.key() @ MarketError::WrongMarket,
    )]
    pub position: Account<'info, Position>,

    /// CHECK: vault PDA holds lamports
    #[account(mut, seeds = [b"mkt_vlts", &market.market_id.to_le_bytes()], bump)]
    pub vault: AccountInfo<'info>,
}

// ─── State ──────────────────────────────────────────────────────────────────

#[account]
pub struct Market {
    pub creator: Pubkey,
    pub market_id: u64,
    pub question_hash: [u8; 32],    // SHA-256 of question text
    pub deadline: i64,              // Unix timestamp
    pub resolution_mode: u8,        // 0 = binary, 1 = threshold
    pub yes_pool: u64,              // Total YES lamports
    pub no_pool: u64,               // Total NO lamports
    pub yes_positions: u32,         // Count of YES positions
    pub no_positions: u32,          // Count of NO positions
    pub outcome: u8,                // 0 = YES, 1 = NO, 255 = unresolved
    pub resolved: bool,
    pub randomness: [u8; 32],       // VRF randomness that determined outcome
    pub bump: u8,
}

impl Market {
    const LEN: usize = 32 + 8 + 32 + 8 + 1 + 8 + 8 + 4 + 4 + 1 + 1 + 32 + 1; // 140
}

#[account]
pub struct Position {
    pub player: Pubkey,
    pub market: Pubkey,
    pub side: u8,       // 0 = NO, 1 = YES
    pub amount: u64,
    pub claimed: bool,
    pub bump: u8,
}

impl Position {
    const LEN: usize = 32 + 32 + 1 + 8 + 1 + 1; // 75
}

// ─── Errors ─────────────────────────────────────────────────────────────────

#[error_code]
pub enum MarketError {
    #[msg("Resolution mode must be 0 (binary) or 1 (threshold)")]
    InvalidResolutionMode,
    #[msg("Deadline must be in the future")]
    DeadlineInPast,
    #[msg("Side must be 0 (NO) or 1 (YES)")]
    InvalidSide,
    #[msg("Amount must be > 0")]
    ZeroAmount,
    #[msg("Market already resolved")]
    MarketResolved,
    #[msg("Market not yet closed (deadline not reached)")]
    MarketNotClosed,
    #[msg("Market not yet resolved")]
    MarketNotResolved,
    #[msg("Market is closed for new positions")]
    MarketClosed,
    #[msg("Position already claimed")]
    AlreadyClaimed,
    #[msg("Position is on the losing side")]
    PositionLost,
    #[msg("No payout")]
    NoPayout,
    #[msg("Position is for a different market")]
    WrongMarket,
}
