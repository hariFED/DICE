use anchor_lang::prelude::*;

use crate::constants::{COMMIT_TIMEOUT_SLOTS, MIN_NODES_REQUIRED, REQUEST_FEE_LAMPORTS, SEED_CHANNEL};
use crate::error::DiceError;
use crate::state::{ChannelStatus, DiceChannel};

#[derive(Accounts)]
pub struct RequestRandomnessV2<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [SEED_CHANNEL, channel.authority.as_ref(), &channel.channel_index.to_le_bytes()],
        bump,
        has_one = authority,
    )]
    pub channel: Account<'info, DiceChannel>,
}

pub fn handler(ctx: Context<RequestRandomnessV2>, node_count: u8) -> Result<()> {
    let clock = Clock::get()?;
    let channel = &mut ctx.accounts.channel;

    // Channel must be idle (previous round finished or first request)
    require!(
        channel.status == ChannelStatus::Idle,
        DiceError::RoundNotComplete
    );

    // Validate node count
    require!(
        node_count >= MIN_NODES_REQUIRED && node_count <= channel.max_nodes,
        DiceError::InvalidNodeCount
    );

    // Deduct protocol fee from prepaid balance
    // Fee scales with node count: base + per-node
    let fee = REQUEST_FEE_LAMPORTS;
    require!(channel.balance >= fee, DiceError::EscrowInsufficient);
    channel.balance = channel
        .balance
        .checked_sub(fee)
        .ok_or(error!(DiceError::EscrowInsufficient))?;

    // Reset the channel for a new round
    channel.reset_for_new_round(node_count);
    channel.created_slot = clock.slot;
    channel.commit_deadline_slot = clock.slot + COMMIT_TIMEOUT_SLOTS;
    channel.reveal_deadline_slot = 0; // set when commit phase ends

    msg!(
        "Randomness requested on channel: authority={}, index={}, round_id={}, nodes={}, fee={}",
        channel.authority,
        channel.channel_index,
        channel.round_id,
        node_count,
        fee
    );
    Ok(())
}
