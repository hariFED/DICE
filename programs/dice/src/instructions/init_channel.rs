use anchor_lang::prelude::*;

use crate::constants::{MAX_NODES_SELECTED, MIN_NODES_REQUIRED, SEED_CHANNEL};
use crate::error::DiceError;
use crate::state::{ChannelStatus, DiceChannel};

#[derive(Accounts)]
#[instruction(channel_index: u16, max_nodes: u8, callback_program_id: Pubkey)]
pub struct InitChannel<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        init,
        payer = authority,
        space = DiceChannel::space(max_nodes),
        seeds = [SEED_CHANNEL, authority.key().as_ref(), &channel_index.to_le_bytes()],
        bump,
    )]
    pub channel: Account<'info, DiceChannel>,

    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<InitChannel>,
    channel_index: u16,
    max_nodes: u8,
    callback_program_id: Pubkey,
) -> Result<()> {
    require!(
        max_nodes >= MIN_NODES_REQUIRED && max_nodes <= MAX_NODES_SELECTED,
        DiceError::InvalidNodeCount
    );

    let n = max_nodes as usize;
    let channel = &mut ctx.accounts.channel;

    channel.authority = ctx.accounts.authority.key();
    channel.channel_index = channel_index;
    channel.max_nodes = max_nodes;
    channel.status = ChannelStatus::Idle;
    channel.round_id = 0;
    channel.node_count = 0;
    channel.commits_received = 0;
    channel.reveals_received = 0;
    channel.created_slot = 0;
    channel.commit_deadline_slot = 0;
    channel.reveal_deadline_slot = 0;
    channel.balance = 0;
    channel.callback_program_id = callback_program_id;
    channel.randomness = [0u8; 32];

    // Initialize arrays with capacity
    channel.device_ids = vec![[0u8; 32]; n];
    channel.device_pubkeys = vec![[0u8; 33]; n];
    channel.commit_hashes = vec![[0u8; 32]; n];
    channel.entropies = vec![[0u8; 32]; n];
    channel.signatures = vec![[0u8; 64]; n];

    msg!(
        "Channel initialized: authority={}, index={}, max_nodes={}",
        channel.authority,
        channel_index,
        max_nodes
    );
    Ok(())
}
