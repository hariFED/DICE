// On-chain deterministic node selection.
//
// STATUS (v7, 2026-04-15): this instruction is **not used by the deployed
// v2 channel flow**. In production:
//
//   - `request_randomness_auto` emits an on-chain event but does NOT call
//     `select_nodes`. Node selection happens off-chain inside the
//     coordinator's `SelectionEngine` (see `coordinator/src/selection.rs`),
//     which picks live, healthy, recently-pinged devices from its
//     in-memory `NodeRegistry`.
//
//   - The coordinator then dispatches `JobAssignment` messages directly
//     to the chosen devices over the mTLS WebSocket channel, runs the
//     commit-reveal protocol, and finally submits the bundled on-chain
//     transactions (submit_commit_v2 × N + submit_reveal_v2 × N +
//     finalize_v2 + claim_rewards_v2). The `device_pubkeys` stored in
//     the DiceChannel come from whichever order the commits arrived on
//     chain — not from this instruction.
//
// Why keep this instruction at all?
//
//   - It provides the `v7+` path for trust-minimized selection: any
//     third-party coordinator (or the chain itself, via a future permissionless
//     crank) can call `select_nodes` to pick nodes via SlotHashes randomness
//     instead of trusting a single off-chain coordinator's choice.
//   - It is the forward-compat primitive for audit-logged selection: every
//     selection is a transaction hash anyone can verify after the fact.
//   - Removing it would break the instruction layout expected by any
//     downstream integration that already references it by discriminator.
//
// Until `request_randomness_auto` is changed to require `select_nodes`
// to have been called first (or the runtime makes it a CPI prerequisite),
// treat this instruction as DORMANT. The `SelectionEngine` in the
// coordinator is the de-facto selection authority in v7.

use anchor_lang::prelude::*;
use anchor_lang::solana_program::hash::hashv;
use anchor_lang::solana_program::sysvar::slot_hashes;

use crate::constants::SEED_CHANNEL;
use crate::error::DiceError;
use crate::state::{ChannelStatus, DiceChannel};

/// DeviceRegistry account discriminator: SHA-256("account:DeviceRegistry")[0..8]
pub const DEVICE_REGISTRY_DISC: [u8; 8] = [103, 245, 70, 187, 154, 60, 208, 216];

/// Core node-selection algorithm — shared by the standalone `select_nodes`
/// instruction and by `request_randomness_auto` (v7.3+) which inlines the
/// selection into the same TX that opens the round.
///
/// Requires:
///   - `channel.node_count` already set to the desired N.
///   - `channel.device_ids` / `channel.device_pubkeys` large enough to hold N.
///   - `slot_hashes_account` pointing at the `SlotHashes` sysvar.
///   - `remaining_accounts` containing ≥ N `DeviceRegistry` PDAs with
///     `is_active == true` (the fn skips inactive / wrong-discriminator ones).
///   - `round_id` to bind the selection to a specific round, so a replay of
///     the same slot_hash + channel seed on a different round still produces
///     a fresh selection.
///
/// On success writes the selected N devices into
/// `channel.device_ids[0..N]` and `channel.device_pubkeys[0..N]`.
pub fn run_node_selection<'info>(
    channel: &mut DiceChannel,
    channel_key: &Pubkey,
    slot_hashes_account: &AccountInfo<'info>,
    remaining_accounts: &[AccountInfo<'info>],
    round_id: u64,
) -> Result<()> {
    // Sysvar address guard — protect against a caller passing a lookalike.
    require!(
        slot_hashes_account.key() == slot_hashes::id(),
        DiceError::RoundNotComplete
    );

    let node_count = channel.node_count as usize;
    require!(
        remaining_accounts.len() >= node_count,
        DiceError::InsufficientNodes
    );

    // Read the most recent slot hash. SlotHashes layout:
    //   u64 length prefix, then entries of (u64 slot, [u8; 32] hash).
    // We just need the first entry's hash (most recent slot).
    let slot_hashes_data = slot_hashes_account.try_borrow_data()?;
    require!(slot_hashes_data.len() >= 48, DiceError::RoundNotComplete);
    let recent_slot_hash: &[u8] = &slot_hashes_data[16..48];

    // Deterministic seed binding the selection to (slot, channel, round, block)
    // so the same slot_hash on a different round still produces a fresh pick.
    let round_id_bytes = round_id.to_le_bytes();
    let clock = Clock::get()?;
    let block_height_bytes = clock.slot.to_le_bytes();

    let seed_hash = hashv(&[
        recent_slot_hash,
        channel_key.as_ref(),
        &round_id_bytes,
        &block_height_bytes,
    ]);
    let seed_bytes = seed_hash.to_bytes();

    // Filter remaining_accounts to validated + active DeviceRegistry PDAs.
    let mut candidates: Vec<([u8; 32], [u8; 33])> = Vec::with_capacity(remaining_accounts.len());
    for acc in remaining_accounts.iter() {
        let data = acc.try_borrow_data()?;
        if data.len() < 58 {
            continue;
        }
        if data[0..8] != DEVICE_REGISTRY_DISC {
            continue;
        }
        // is_active at offset 57
        if data[57] == 0 {
            continue;
        }
        let device_pubkey: [u8; 33] = data[8..41].try_into().unwrap();
        let device_id = hashv(&[&device_pubkey]).to_bytes();
        candidates.push((device_id, device_pubkey));
    }

    require!(
        candidates.len() >= node_count,
        DiceError::InsufficientNodes
    );

    // Fisher-Yates over candidate indices, seeded from the hash above.
    let mut indices: Vec<usize> = (0..candidates.len()).collect();
    for i in (1..indices.len()).rev() {
        let idx_hash = hashv(&[&seed_bytes, &(i as u64).to_le_bytes()]);
        let j = u64::from_le_bytes(idx_hash.to_bytes()[0..8].try_into().unwrap()) as usize % (i + 1);
        indices.swap(i, j);
    }

    // Write the top-N selection into the channel.
    for slot in 0..node_count {
        let candidate_idx = indices[slot];
        let (device_id, device_pubkey) = candidates[candidate_idx];
        channel.device_ids[slot] = device_id;
        channel.device_pubkeys[slot] = device_pubkey;
    }

    msg!(
        "run_node_selection: round_id={}, selected={}/{}",
        round_id,
        node_count,
        candidates.len()
    );
    Ok(())
}

#[derive(Accounts)]
pub struct SelectNodes<'info> {
    /// The protocol coordinator.
    pub coordinator: Signer<'info>,

    #[account(
        mut,
        seeds = [SEED_CHANNEL, channel.authority.as_ref(), &channel.channel_index.to_le_bytes()],
        bump,
    )]
    pub channel: Account<'info, DiceChannel>,

    /// SlotHashes sysvar — provides 512 recent slot hashes for unpredictable seed.
    /// CHECK: validated by address constraint.
    #[account(address = slot_hashes::id())]
    pub slot_hashes: AccountInfo<'info>,

    // remaining_accounts: all DeviceRegistry PDAs (read-only)
}

pub fn handler<'info>(
    ctx: Context<'_, '_, 'info, 'info, SelectNodes<'info>>,
    round_id: u64,
) -> Result<()> {
    // Validate coordinator is authorized for this channel.
    require!(
        ctx.accounts.coordinator.key() == ctx.accounts.channel.coordinator,
        DiceError::UnauthorizedCoordinator
    );

    // Validate round_id matches current round.
    require!(
        ctx.accounts.channel.round_id == round_id,
        DiceError::RoundAlreadyFinalized
    );

    // Must be in Pending status (request submitted, awaiting node selection).
    require!(
        ctx.accounts.channel.status == ChannelStatus::Pending,
        DiceError::RoundNotComplete
    );

    // Delegate the selection math to the shared helper so both this
    // standalone entry point and `request_randomness_auto` use exactly the
    // same algorithm.
    let channel_key = ctx.accounts.channel.key();
    let channel = &mut ctx.accounts.channel;
    run_node_selection(
        channel,
        &channel_key,
        &ctx.accounts.slot_hashes,
        ctx.remaining_accounts,
        round_id,
    )
}
