use anchor_lang::prelude::*;
use anchor_lang::solana_program::hash::hashv;
use anchor_lang::solana_program::sysvar::slot_hashes;

use crate::constants::SEED_CHANNEL;
use crate::error::DiceError;
use crate::state::{ChannelStatus, DiceChannel};

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
    let channel = &ctx.accounts.channel;

    // Validate round_id matches
    require!(
        channel.round_id == round_id,
        DiceError::RoundAlreadyFinalized
    );

    // Must be in Pending status (request submitted, awaiting node selection)
    require!(
        channel.status == ChannelStatus::Pending,
        DiceError::RoundNotComplete
    );

    let node_count = channel.node_count as usize;
    let remaining = ctx.remaining_accounts;

    // Need at least node_count device registries to select from
    require!(
        remaining.len() >= node_count,
        DiceError::InsufficientNodes
    );

    // Read the most recent slot hash from the SlotHashes sysvar.
    // SlotHashes layout: 8-byte length prefix, then entries of (u64 slot, [u8; 32] hash).
    // We just need the first entry's hash (most recent).
    let slot_hashes_data = ctx.accounts.slot_hashes.try_borrow_data()?;
    require!(slot_hashes_data.len() >= 48, DiceError::RoundNotComplete);

    // First 8 bytes = number of entries (u64 LE)
    // Next 8 bytes = first entry's slot (u64 LE)
    // Next 32 bytes = first entry's hash
    let recent_slot_hash: &[u8] = &slot_hashes_data[16..48];

    // Compute deterministic seed: SHA-256(slot_hash || channel_key || round_id || block_height)
    let channel_key = channel.key();
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

    // Filter to active registries and collect their device info
    let mut candidates: Vec<([u8; 32], [u8; 33])> = Vec::with_capacity(remaining.len());
    for acc in remaining.iter() {
        let data = acc.try_borrow_data()?;
        // DeviceRegistry layout: 8 (disc) + 33 (device_pubkey) + 8 (registered_at) + 8 (jobs_completed) + 1 (is_active)
        if data.len() < 58 {
            continue;
        }
        let is_active = data[57] != 0;
        if !is_active {
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

    // Fisher-Yates shuffle using seed bytes as entropy source.
    // We generate additional entropy by hashing seed + index for each step.
    let mut indices: Vec<usize> = (0..candidates.len()).collect();
    for i in (1..indices.len()).rev() {
        // Generate a pseudo-random index using the seed
        let idx_hash = hashv(&[&seed_bytes, &(i as u64).to_le_bytes()]);
        let j = u64::from_le_bytes(idx_hash.to_bytes()[0..8].try_into().unwrap()) as usize % (i + 1);
        indices.swap(i, j);
    }

    // Select the first `node_count` from the shuffled indices
    let channel = &mut ctx.accounts.channel;
    for slot in 0..node_count {
        let candidate_idx = indices[slot];
        let (device_id, device_pubkey) = candidates[candidate_idx];
        channel.device_ids[slot] = device_id;
        channel.device_pubkeys[slot] = device_pubkey;
    }

    msg!(
        "Nodes selected on-chain: round_id={}, selected={}/{}",
        round_id,
        node_count,
        candidates.len()
    );
    Ok(())
}
