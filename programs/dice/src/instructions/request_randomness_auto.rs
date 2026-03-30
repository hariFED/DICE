use anchor_lang::prelude::*;

use crate::constants::{
    COMMIT_TIMEOUT_SLOTS, MAX_NODES_SELECTED, MIN_NODES_REQUIRED, REQUEST_FEE_LAMPORTS,
    SEED_CHANNEL,
};
use crate::error::DiceError;
use crate::state::{ChannelStatus, DiceChannel};

/// Zero-friction randomness request.
///
/// If the channel doesn't exist yet, it is created automatically (developer pays rent).
/// If the channel exists and is Idle, a new round is started.
/// The fee (0.002 SOL) is transferred from the developer's wallet to the channel
/// balance automatically if the channel balance is insufficient.
///
/// This is the only instruction a developer needs to call — everything else is automatic.
#[derive(Accounts)]
#[instruction(channel_index: u16, max_nodes: u8, node_count: u8, callback_program_id: Pubkey)]
pub struct RequestRandomnessAuto<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        init_if_needed,
        payer = authority,
        space = DiceChannel::space(max_nodes),
        seeds = [SEED_CHANNEL, authority.key().as_ref(), &channel_index.to_le_bytes()],
        bump,
    )]
    pub channel: Account<'info, DiceChannel>,

    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<RequestRandomnessAuto>,
    channel_index: u16,
    max_nodes: u8,
    node_count: u8,
    callback_program_id: Pubkey,
) -> Result<()> {
    let clock = Clock::get()?;
    let channel = &mut ctx.accounts.channel;

    // Validate node counts
    require!(
        max_nodes >= MIN_NODES_REQUIRED && max_nodes <= MAX_NODES_SELECTED,
        DiceError::InvalidNodeCount
    );
    require!(
        node_count >= MIN_NODES_REQUIRED && node_count <= max_nodes,
        DiceError::InvalidNodeCount
    );

    // If channel is brand new (round_id == 0 and authority is default), initialize it.
    if channel.authority == Pubkey::default() {
        let n = max_nodes as usize;
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
        channel.device_ids = vec![[0u8; 32]; n];
        channel.device_pubkeys = vec![[0u8; 33]; n];
        channel.commit_hashes = vec![[0u8; 32]; n];
        channel.entropies = vec![[0u8; 32]; n];
        channel.signatures = vec![[0u8; 64]; n];

        msg!(
            "Channel auto-created: authority={}, index={}, max_nodes={}",
            channel.authority,
            channel_index,
            max_nodes
        );
    } else {
        // Update callback if changed
        channel.callback_program_id = callback_program_id;
    }

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
    channel.commit_deadline_slot = clock.slot + COMMIT_TIMEOUT_SLOTS;
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
