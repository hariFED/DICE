use anchor_lang::prelude::*;

use crate::constants::SEED_CHANNEL;
use crate::error::DiceError;
use crate::state::{ChannelStatus, DiceChannel};

#[derive(Accounts)]
pub struct SubmitCommitV2<'info> {
    /// The protocol coordinator.
    pub coordinator: Signer<'info>,

    #[account(
        mut,
        seeds = [SEED_CHANNEL, channel.authority.as_ref(), &channel.channel_index.to_le_bytes()],
        bump,
    )]
    pub channel: Account<'info, DiceChannel>,
}

pub fn handler(
    ctx: Context<SubmitCommitV2>,
    round_id: u64,
    device_id: [u8; 32],
    device_pubkey: [u8; 33],
    commit_hash: [u8; 32],
) -> Result<()> {
    let clock = Clock::get()?;
    let channel = &mut ctx.accounts.channel;

    // Validate coordinator is authorized for this channel
    require!(
        ctx.accounts.coordinator.key() == channel.coordinator,
        DiceError::UnauthorizedCoordinator
    );

    // Validate round_id to prevent replay from previous rounds
    require!(channel.round_id == round_id, DiceError::RoundAlreadyFinalized);

    // Channel must be in Pending or CommitPhase
    require!(
        channel.status == ChannelStatus::Pending || channel.status == ChannelStatus::CommitPhase,
        DiceError::RoundAlreadyFinalized
    );

    // Check commit deadline
    require!(
        clock.slot <= channel.commit_deadline_slot,
        DiceError::RoundTimedOut
    );

    // Verify device_id matches device_pubkey
    require!(
        device_id == crate::constants::device_id(&device_pubkey),
        DiceError::InvalidDeviceId
    );

    // Check we haven't exceeded node_count
    let idx = channel.commits_received as usize;
    require!(idx < channel.node_count as usize, DiceError::InvalidNodeCount);

    // Check this device hasn't already committed (scan existing entries)
    for i in 0..idx {
        require!(
            channel.device_ids[i] != device_id,
            DiceError::AlreadyCommitted
        );
    }

    // Write commit data inline
    channel.device_ids[idx] = device_id;
    channel.device_pubkeys[idx] = device_pubkey;
    channel.commit_hashes[idx] = commit_hash;
    channel.commits_received += 1;
    channel.status = ChannelStatus::CommitPhase;

    msg!(
        "Commit received: round_id={}, device={:?}, commits={}/{}",
        round_id,
        &device_pubkey[..4],
        channel.commits_received,
        channel.node_count
    );
    Ok(())
}
