use anchor_lang::prelude::*;

use crate::constants::{
    COMMIT_TIMEOUT_SLOTS, MIN_NODES_REQUIRED, REQUEST_FEE_LAMPORTS, SEED_CHANNEL,
};
use crate::error::DiceError;
use crate::state::{ChannelStatus, DiceChannel};

/// Auto-fund randomness request.
///
/// The channel MUST already exist (created via `init_channel`).
/// If the channel balance is insufficient, automatically transfers the
/// deficit from the developer's wallet. Then starts a new round.
///
/// This is the simplest way to request randomness — just call this.
/// The developer only needs to call `init_channel` once at setup time.
#[derive(Accounts)]
pub struct RequestRandomnessAuto<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [SEED_CHANNEL, channel.authority.as_ref(), &channel.channel_index.to_le_bytes()],
        bump,
        has_one = authority,
    )]
    pub channel: Account<'info, DiceChannel>,

    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<RequestRandomnessAuto>,
    node_count: u8,
) -> Result<()> {
    let clock = Clock::get()?;
    let channel = &mut ctx.accounts.channel;

    // Validate node count
    require!(
        node_count >= MIN_NODES_REQUIRED && node_count <= channel.max_nodes,
        DiceError::InvalidNodeCount
    );

    // Channel must be idle
    require!(
        channel.status == ChannelStatus::Idle,
        DiceError::RoundNotComplete
    );

    // Auto-fund: if balance < fee, transfer the difference from authority
    let fee = REQUEST_FEE_LAMPORTS;
    if channel.balance < fee {
        let deficit = fee - channel.balance;

        // CPI transfer from authority to channel
        let cpi_ctx = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            anchor_lang::system_program::Transfer {
                from: ctx.accounts.authority.to_account_info(),
                to: channel.to_account_info(),
            },
        );
        anchor_lang::system_program::transfer(cpi_ctx, deficit)?;
        channel.balance = channel
            .balance
            .checked_add(deficit)
            .ok_or(error!(DiceError::EscrowInsufficient))?;
    }

    // Deduct fee
    channel.balance = channel
        .balance
        .checked_sub(fee)
        .ok_or(error!(DiceError::EscrowInsufficient))?;

    // Reset for new round
    channel.reset_for_new_round(node_count);
    channel.created_slot = clock.slot;
    channel.commit_deadline_slot = clock.slot
        .checked_add(COMMIT_TIMEOUT_SLOTS)
        .ok_or(error!(DiceError::RoundTimedOut))?;
    channel.reveal_deadline_slot = 0;

    msg!(
        "Randomness requested (auto): authority={}, index={}, round_id={}, nodes={}, fee={}",
        channel.authority,
        channel.channel_index,
        channel.round_id,
        node_count,
        fee
    );
    Ok(())
}
