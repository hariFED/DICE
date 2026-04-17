use anchor_lang::prelude::*;
use solana_program::hash::hashv;

use crate::constants::{REVEAL_TIMEOUT_SLOTS, SEED_CHANNEL};
use crate::error::DiceError;
use crate::state::{ChannelStatus, DiceChannel};

#[derive(Accounts)]
pub struct SubmitRevealV2<'info> {
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
    ctx: Context<SubmitRevealV2>,
    round_id: u64,
    device_id: [u8; 32],
    device_pubkey: [u8; 33],
    entropy: [u8; 32],
    signature: [u8; 64],
) -> Result<()> {
    let clock = Clock::get()?;
    let channel = &mut ctx.accounts.channel;

    // Validate coordinator is authorized for this channel
    require!(
        ctx.accounts.coordinator.key() == channel.coordinator,
        DiceError::UnauthorizedCoordinator
    );

    // Validate round_id
    require!(channel.round_id == round_id, DiceError::RoundAlreadyFinalized);

    // Must be in CommitPhase (all commits done) or RevealPhase
    // Transition to RevealPhase on first reveal if all commits are in
    if channel.status == ChannelStatus::CommitPhase
        && channel.commits_received >= channel.node_count
    {
        channel.status = ChannelStatus::RevealPhase;
        channel.reveal_deadline_slot = clock.slot + REVEAL_TIMEOUT_SLOTS;
    }

    require!(
        channel.status == ChannelStatus::RevealPhase,
        DiceError::RoundNotComplete
    );

    // Check reveal deadline
    if channel.reveal_deadline_slot > 0 {
        require!(
            clock.slot <= channel.reveal_deadline_slot,
            DiceError::RoundTimedOut
        );
    }

    // Verify device_id
    require!(
        device_id == crate::constants::device_id(&device_pubkey),
        DiceError::InvalidDeviceId
    );

    // Find this device's index in the commits (must have committed)
    let mut device_idx: Option<usize> = None;
    for i in 0..channel.commits_received as usize {
        if channel.device_ids[i] == device_id {
            device_idx = Some(i);
            break;
        }
    }
    let idx = device_idx.ok_or(error!(DiceError::UnauthorizedNode))?;

    // Check not already revealed (entropy should still be zeroed)
    require!(
        channel.entropies[idx] == [0u8; 32],
        DiceError::AlreadyCommitted // reusing error for "already revealed"
    );

    // Verify SHA-256(entropy) == commit_hash
    let computed_hash = hashv(&[&entropy]).to_bytes();
    require!(
        computed_hash == channel.commit_hashes[idx],
        DiceError::RevealMismatch
    );

    // Write reveal data inline
    channel.entropies[idx] = entropy;
    channel.signatures[idx] = signature;
    channel.reveals_received += 1;

    msg!(
        "Reveal received: round_id={}, device={:?}, reveals={}/{}",
        round_id,
        &device_pubkey[..4],
        channel.reveals_received,
        channel.node_count
    );
    Ok(())
}
