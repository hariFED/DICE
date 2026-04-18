use anchor_lang::prelude::*;
use solana_program::sysvar::slot_hashes;

use crate::constants::{
    COMMIT_TIMEOUT_SLOTS, MIN_NODES_REQUIRED, REQUEST_FEE_LAMPORTS, SEED_CHANNEL,
};
use crate::error::DiceError;
use crate::instructions::select_nodes::run_node_selection;
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

pub fn handler<'info>(
    ctx: Context<'info, RequestRandomnessAuto<'info>>,
    node_count: u8,
) -> Result<()> {
    let clock = Clock::get()?;
    let channel_key = ctx.accounts.channel.key();
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
            anchor_lang::system_program::System::id(),
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

    let round_id_now = channel.round_id;

    // v7.3 on-chain node selection (optional, backwards compatible).
    //
    // If the caller passed any `remaining_accounts`, the FIRST entry must
    // be the `SlotHashes` sysvar and the rest must be `DeviceRegistry`
    // PDAs. The Fisher-Yates shuffle in `run_node_selection` then picks
    // N devices deterministically and writes them into
    // `channel.device_pubkeys` before we return.
    //
    // If `remaining_accounts` is empty, we leave `device_pubkeys` zeroed
    // and fall back to the coordinator's off-chain SelectionEngine — the
    // pre-v7.3 behaviour. This keeps existing callers (stress_driver,
    // pulse_driver, coin_toss_driver that don't yet pass device PDAs)
    // working during the transition.
    let remaining = ctx.remaining_accounts;
    if !remaining.is_empty() {
        // First remaining account must be the SlotHashes sysvar. The helper
        // verifies the key inside.
        require!(
            remaining[0].key() == slot_hashes::id(),
            DiceError::RoundNotComplete
        );
        let slot_hashes_ai = &remaining[0];
        let device_registries = &remaining[1..];

        run_node_selection(
            channel,
            &channel_key,
            slot_hashes_ai,
            device_registries,
            round_id_now,
        )?;

        msg!(
            "Randomness requested (auto): authority={}, index={}, round_id={}, nodes={}, fee={}, on_chain_selected=true, candidate_pool={}",
            channel.authority,
            channel.channel_index,
            round_id_now,
            node_count,
            fee,
            device_registries.len()
        );
    } else {
        msg!(
            "Randomness requested (auto): authority={}, index={}, round_id={}, nodes={}, fee={}, on_chain_selected=false",
            channel.authority,
            channel.channel_index,
            round_id_now,
            node_count,
            fee
        );
    }

    Ok(())
}
