use anchor_lang::prelude::*;

use crate::constants::{MAX_NODES_SELECTED, MIN_NODES_REQUIRED, SEED_CHANNEL};
use crate::error::DiceError;
use crate::state::{ChannelStatus, DiceChannel};

#[derive(Accounts)]
#[instruction(new_max_nodes: u8)]
pub struct ResizeChannel<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [SEED_CHANNEL, channel.authority.as_ref(), &channel.channel_index.to_le_bytes()],
        bump,
        has_one = authority,
        realloc = DiceChannel::space(new_max_nodes),
        realloc::payer = authority,
        realloc::zero = true,
    )]
    pub channel: Account<'info, DiceChannel>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<ResizeChannel>, new_max_nodes: u8) -> Result<()> {
    let channel = &mut ctx.accounts.channel;

    // Only resize when idle
    require!(
        channel.status == ChannelStatus::Idle,
        DiceError::RoundNotComplete
    );

    require!(
        new_max_nodes >= MIN_NODES_REQUIRED && new_max_nodes <= MAX_NODES_SELECTED,
        DiceError::InvalidNodeCount
    );

    let old = channel.max_nodes;
    let n = new_max_nodes as usize;
    channel.max_nodes = new_max_nodes;

    // Re-initialize arrays with new capacity
    channel.device_ids = vec![[0u8; 32]; n];
    channel.device_pubkeys = vec![[0u8; 33]; n];
    channel.commit_hashes = vec![[0u8; 32]; n];
    channel.entropies = vec![[0u8; 32]; n];
    channel.signatures = vec![[0u8; 64]; n];

    msg!(
        "Channel resized: max_nodes {} → {}",
        old,
        new_max_nodes
    );
    Ok(())
}
