use anchor_lang::prelude::*;
use solana_program::keccak;
use solana_program::secp256k1_recover::secp256k1_recover;

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
    let entropy_hash = solana_program::hash::hashv(&[&entropy]);
    require!(
        entropy_hash.to_bytes() == ctx.accounts.commit_record.commit_hash,
        DiceError::RevealMismatch
    );

    // Verify ECDSA secp256k1 signature.
    // The message is keccak256(entropy) following Ethereum signing convention.
    //
    // NOTE on recovery-id logic: `secp256k1_recover` almost always returns
    // `Ok` for BOTH recovery IDs — the two results are distinct public keys
    // and only one of them is the signer. The previous implementation did
    //   secp256k1_recover(.., 0, ..).or_else(|_| secp256k1_recover(.., 1, ..))
    // which bailed out on the first Ok and returned a valid-but-wrong
    // pubkey for half of legitimate reveals. We now try both recovery IDs
    // and only accept the result whose compressed form matches the stored
    // device pubkey.
    let msg_hash = keccak::hash(&entropy);
    let sig_bytes = &signature[..64];
    let mut sig_arr = [0u8; 64];
    sig_arr.copy_from_slice(sig_bytes);

    let mut matched = false;
    for rec_id in [0u8, 1u8] {
        if let Ok(recovered) = secp256k1_recover(msg_hash.as_ref(), rec_id, &sig_arr) {
            // secp256k1_recover returns an uncompressed pubkey (64 bytes,
            // x || y without the 0x04 prefix). Compress it and compare
            // against the stored 33-byte device pubkey.
            let raw = recovered.0;
            let y_parity = raw[63] & 1;
            let prefix = if y_parity == 0 { 0x02u8 } else { 0x03u8 };
            let mut compressed = [0u8; 33];
            compressed[0] = prefix;
            compressed[1..33].copy_from_slice(&raw[..32]);
            if compressed == device_pubkey {
                matched = true;
                break;
            }
        }
    }
    require!(matched, DiceError::InvalidSignature);

    // Record the reveal (release req reference to avoid borrow conflict)
    let _ = req;

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
