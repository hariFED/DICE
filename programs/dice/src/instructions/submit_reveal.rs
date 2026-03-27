use anchor_lang::prelude::*;
use anchor_lang::solana_program::keccak;
use anchor_lang::solana_program::secp256k1_recover::secp256k1_recover;

use crate::constants::{REVEAL_TIMEOUT_SLOTS, SEED_COMMIT, SEED_REQUEST, SEED_REVEAL};
use crate::error::DiceError;
use crate::state::{CommitRecord, RandomnessRequest, RequestStatus, RevealRecord};

#[derive(Accounts)]
#[instruction(device_id: [u8; 32], device_pubkey: [u8; 33], entropy: [u8; 32], signature: [u8; 64])]
pub struct SubmitReveal<'info> {
    /// The protocol coordinator
    #[account(mut)]
    pub coordinator: Signer<'info>,

    #[account(
        mut,
        seeds = [
            SEED_REQUEST,
            randomness_request.requester.as_ref(),
            &randomness_request.sequence.to_le_bytes(),
        ],
        bump,
    )]
    pub randomness_request: Account<'info, RandomnessRequest>,

    #[account(
        mut,
        seeds = [
            SEED_COMMIT,
            randomness_request.requester.as_ref(),
            &randomness_request.sequence.to_le_bytes(),
            &device_id,
        ],
        bump,
        constraint = commit_record.device_pubkey == device_pubkey,
    )]
    pub commit_record: Account<'info, CommitRecord>,

    #[account(
        init,
        payer = coordinator,
        space = RevealRecord::LEN,
        seeds = [
            SEED_REVEAL,
            randomness_request.requester.as_ref(),
            &randomness_request.sequence.to_le_bytes(),
            &device_id,
        ],
        bump,
    )]
    pub reveal_record: Account<'info, RevealRecord>,

    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<SubmitReveal>,
    device_id: [u8; 32],
    device_pubkey: [u8; 33],
    entropy: [u8; 32],
    signature: [u8; 64],
) -> Result<()> {
    require!(device_id == crate::constants::device_id(&device_pubkey), DiceError::InvalidDeviceId);

    let clock = Clock::get()?;
    let request_key = ctx.accounts.randomness_request.key();
    let req = &mut ctx.accounts.randomness_request;

    // Verify the round is accepting reveals
    require!(
        req.status == RequestStatus::CommitPhase || req.status == RequestStatus::RevealPhase,
        DiceError::RoundAlreadyFinalized
    );

    // Set reveal deadline on first reveal (transition from CommitPhase)
    if req.status == RequestStatus::CommitPhase {
        req.reveal_deadline_slot = clock.slot + REVEAL_TIMEOUT_SLOTS;
        req.status = RequestStatus::RevealPhase;
    }

    // Verify the reveal deadline has not passed
    require!(
        clock.slot <= req.reveal_deadline_slot,
        DiceError::RoundTimedOut
    );

    // Verify the device is one of the selected nodes
    let mut is_selected = false;
    for i in 0..req.node_count as usize {
        if req.selected_nodes[i] == device_pubkey {
            is_selected = true;
            break;
        }
    }
    require!(is_selected, DiceError::UnauthorizedNode);

    // Verify hash(entropy) matches the stored commit
    let entropy_hash = anchor_lang::solana_program::hash::hashv(&[&entropy]);
    require!(
        entropy_hash.to_bytes() == ctx.accounts.commit_record.commit_hash,
        DiceError::RevealMismatch
    );

    // Verify ECDSA secp256k1 signature
    // The message is keccak256(entropy) following Ethereum signing convention
    let msg_hash = keccak::hash(&entropy);
    // signature[0..64] is [r (32 bytes) || s (32 bytes)], recovery_id = 0 or 1
    // We try both recovery ids and check which produces the expected pubkey
    let sig_bytes = &signature[..64];
    let mut sig_arr = [0u8; 64];
    sig_arr.copy_from_slice(sig_bytes);

    let recovered = secp256k1_recover(msg_hash.as_ref(), 0, &sig_arr)
        .or_else(|_| secp256k1_recover(msg_hash.as_ref(), 1, &sig_arr))
        .map_err(|_| DiceError::InvalidSignature)?;

    // secp256k1_recover returns an uncompressed pubkey (64 bytes, x||y without 0x04 prefix).
    // We need to compare against the stored compressed pubkey (33 bytes).
    // Compress: prefix is 0x02 if y is even, 0x03 if y is odd.
    let raw = recovered.0; // [u8; 64]
    let y_parity = raw[63] & 1;
    let prefix = if y_parity == 0 { 0x02u8 } else { 0x03u8 };
    let mut compressed = [0u8; 33];
    compressed[0] = prefix;
    compressed[1..33].copy_from_slice(&raw[..32]);

    require!(compressed == device_pubkey, DiceError::InvalidSignature);

    // Record the reveal (drop req borrow first to avoid conflict)
    drop(req);

    let reveal = &mut ctx.accounts.reveal_record;
    reveal.request = request_key;
    reveal.device_pubkey = device_pubkey;
    reveal.entropy = entropy;
    reveal.signature = signature;
    reveal.submitted_slot = clock.slot;

    ctx.accounts.randomness_request.reveals_received += 1;

    msg!(
        "Reveal accepted from device {:?} (total reveals: {})",
        &device_pubkey[..4],
        ctx.accounts.randomness_request.reveals_received,
    );

    Ok(())
}
