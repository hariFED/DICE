use anchor_lang::prelude::*;

use crate::constants::SEED_CHANNEL;
use crate::error::DiceError;
use crate::state::{ChannelStatus, DiceChannel};

/// Mark a stuck round as Failed and reset the channel to Idle.
///
/// Can be called by either the authority (developer) or the coordinator
/// when the commit or reveal deadline has passed without enough responses.
/// This prevents channels from being permanently stuck in a non-Idle state.
#[derive(Accounts)]
pub struct FailRound<'info> {
    /// Either the channel authority or coordinator can fail a round.
    pub caller: Signer<'info>,

    #[account(
        mut,
        seeds = [SEED_CHANNEL, channel.authority.as_ref(), &channel.channel_index.to_le_bytes()],
        bump,
    )]
    pub channel: Account<'info, DiceChannel>,
}

pub fn handler(ctx: Context<FailRound>) -> Result<()> {
    let clock = Clock::get()?;
    let channel = &mut ctx.accounts.channel;

    // Caller must be either authority or coordinator
    let caller = ctx.accounts.caller.key();
    require!(
        caller == channel.authority || caller == channel.coordinator,
        DiceError::UnauthorizedCoordinator
    );

    // Must be in an active round state (not Idle or already Failed)
    require!(
        channel.status == ChannelStatus::Pending
            || channel.status == ChannelStatus::CommitPhase
            || channel.status == ChannelStatus::RevealPhase
            || channel.status == ChannelStatus::Finalized,
        DiceError::RoundNotComplete
    );

    // For Pending/CommitPhase: check commit deadline
    // For RevealPhase: check reveal deadline
    // For Finalized: always allowed (callback may have failed permanently)
    let deadline_passed = match channel.status {
        ChannelStatus::Pending | ChannelStatus::CommitPhase => {
            channel.commit_deadline_slot > 0 && clock.slot > channel.commit_deadline_slot
        }
        ChannelStatus::RevealPhase => {
            channel.reveal_deadline_slot > 0 && clock.slot > channel.reveal_deadline_slot
        }
        ChannelStatus::Finalized => true, // Always allowed to unstick from Finalized
        _ => false,
    };

    require!(deadline_passed, DiceError::RoundTimedOut);

    msg!(
        "Round failed: round_id={}, previous_status={:?}, slot={}",
        channel.round_id,
        channel.status,
        clock.slot
    );

    channel.status = ChannelStatus::Idle;
    Ok(())
}
