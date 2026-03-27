use std::sync::Arc;

use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::Keypair,
    system_program,
};

use sha2::{Digest, Sha256};

use crate::solana_rpc::SolanaRpc;

/// Compute the 32-byte device ID from a 33-byte compressed secp256k1 pubkey.
/// `device_id = SHA-256(device_pubkey)` — must match `programs/dice/src/constants.rs::device_id()`.
pub fn compute_device_id(device_pubkey: &[u8; 33]) -> [u8; 32] {
    Sha256::digest(device_pubkey).into()
}

// Seed constants — must match programs/dice/src/lib.rs exactly.
const SEED_REQUEST: &[u8] = b"request";
const SEED_COMMIT:  &[u8] = b"commit";
const SEED_REVEAL:  &[u8] = b"reveal";
const SEED_RESULT:  &[u8] = b"result";
const SEED_ESCROW:  &[u8] = b"escrow";

// Instruction discriminators — SHA-256("global:<snake_case_name>")[0..8]
// Must match target/idl/dice.json exactly.
const DISC_REQUEST_RANDOMNESS:  [u8; 8] = [213,   5, 173, 166,  37, 236,  31,  18];
const DISC_SUBMIT_COMMIT:       [u8; 8] = [213, 213, 149,  72, 230,  14,  23,  16];
const DISC_SUBMIT_REVEAL:       [u8; 8] = [255, 153,  68,  56, 227,  55,  19, 157];
const DISC_FINALIZE_RANDOMNESS: [u8; 8] = [ 29, 180, 158, 167,  45,  40,   8, 199];
const DISC_CLAIM_REWARDS:       [u8; 8] = [  4, 144, 132,  71, 116,  23, 151,  80];

/// Shared context for on-chain transaction submission.
/// `None` means on-chain txs are disabled (pure in-memory simulation).
#[derive(Clone)]
pub struct OnChainCtx {
    pub rpc: Arc<SolanaRpc>,
    pub keypair: Arc<Keypair>,
    pub program_id: Pubkey,
}

// ---------------------------------------------------------------------------
// PDA helpers
// ---------------------------------------------------------------------------

pub fn request_pda(program_id: &Pubkey, requester: &Pubkey, sequence: u64) -> Pubkey {
    Pubkey::find_program_address(
        &[SEED_REQUEST, requester.as_ref(), &sequence.to_le_bytes()],
        program_id,
    )
    .0
}

pub fn commit_pda(
    program_id: &Pubkey,
    requester: &Pubkey,
    sequence: u64,
    device_id: &[u8; 32],
) -> Pubkey {
    Pubkey::find_program_address(
        &[SEED_COMMIT, requester.as_ref(), &sequence.to_le_bytes(), device_id.as_ref()],
        program_id,
    )
    .0
}

pub fn reveal_pda(
    program_id: &Pubkey,
    requester: &Pubkey,
    sequence: u64,
    device_id: &[u8; 32],
) -> Pubkey {
    Pubkey::find_program_address(
        &[SEED_REVEAL, requester.as_ref(), &sequence.to_le_bytes(), device_id.as_ref()],
        program_id,
    )
    .0
}

pub fn result_pda(program_id: &Pubkey, requester: &Pubkey, sequence: u64) -> Pubkey {
    Pubkey::find_program_address(
        &[SEED_RESULT, requester.as_ref(), &sequence.to_le_bytes()],
        program_id,
    )
    .0
}

pub fn escrow_pda(program_id: &Pubkey, requester: &Pubkey, sequence: u64) -> Pubkey {
    Pubkey::find_program_address(
        &[SEED_ESCROW, requester.as_ref(), &sequence.to_le_bytes()],
        program_id,
    )
    .0
}

// ---------------------------------------------------------------------------
// Instruction builders
// ---------------------------------------------------------------------------

/// Build the `request_randomness` instruction.
///
/// Creates the `RandomnessRequest` and `EscrowAccount` PDAs on-chain.
/// The coordinator pays the 0.002 SOL fee and acts as the requester.
///
/// Instruction data: discriminator (8) + sequence (u64 LE, 8) + callback_program_id (32).
pub fn build_request_randomness_ix(
    program_id: &Pubkey,
    requester: &Pubkey,
    sequence: u64,
    callback_program_id: &Pubkey,
) -> Instruction {
    let request = request_pda(program_id, requester, sequence);
    let escrow  = escrow_pda(program_id, requester, sequence);

    let mut data = DISC_REQUEST_RANDOMNESS.to_vec();
    data.extend_from_slice(&sequence.to_le_bytes());
    data.extend_from_slice(callback_program_id.as_ref());

    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*requester, true),           // requester — signer, writable, payer
            AccountMeta::new(request, false),             // randomnessRequest — writable PDA (init)
            AccountMeta::new(escrow, false),              // escrow — writable PDA (init)
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data,
    }
}

/// Build the `submit_commit` instruction.
///
/// Instruction data: discriminator (8) + device_id (32) + device_pubkey (33) + commit_hash (32).
pub fn build_submit_commit_ix(
    program_id: &Pubkey,
    coordinator: &Pubkey,
    requester: &Pubkey,
    sequence: u64,
    device_pubkey: &[u8; 33],
    commit_hash: &[u8; 32],
) -> Instruction {
    let device_id = compute_device_id(device_pubkey);
    let request     = request_pda(program_id, requester, sequence);
    let commit_rec  = commit_pda(program_id, requester, sequence, &device_id);

    let mut data = DISC_SUBMIT_COMMIT.to_vec();
    data.extend_from_slice(&device_id);
    data.extend_from_slice(device_pubkey);
    data.extend_from_slice(commit_hash);

    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*coordinator, true),         // coordinator — signer, writable
            AccountMeta::new(request, false),             // randomnessRequest — writable PDA
            AccountMeta::new(commit_rec, false),          // commitRecord — writable PDA
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data,
    }
}

/// Build the `submit_reveal` instruction.
///
/// Instruction data: discriminator (8) + device_id (32) + device_pubkey (33) + entropy (32) + signature (64).
pub fn build_submit_reveal_ix(
    program_id: &Pubkey,
    coordinator: &Pubkey,
    requester: &Pubkey,
    sequence: u64,
    device_pubkey: &[u8; 33],
    entropy: &[u8; 32],
    signature: &[u8; 64],
) -> Instruction {
    let device_id = compute_device_id(device_pubkey);
    let request     = request_pda(program_id, requester, sequence);
    let commit_rec  = commit_pda(program_id, requester, sequence, &device_id);
    let reveal_rec  = reveal_pda(program_id, requester, sequence, &device_id);

    let mut data = DISC_SUBMIT_REVEAL.to_vec();
    data.extend_from_slice(&device_id);
    data.extend_from_slice(device_pubkey);
    data.extend_from_slice(entropy);
    data.extend_from_slice(signature);

    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*coordinator, true),         // coordinator — signer, writable
            AccountMeta::new(request, false),             // randomnessRequest — writable PDA
            AccountMeta::new(commit_rec, false),          // commitRecord — writable PDA
            AccountMeta::new(reveal_rec, false),          // revealRecord — writable PDA
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data,
    }
}

/// Build the `finalize_randomness` instruction.
///
/// Instruction data: discriminator (8) only (no args).
pub fn build_finalize_randomness_ix(
    program_id: &Pubkey,
    coordinator: &Pubkey,
    requester: &Pubkey,
    sequence: u64,
) -> Instruction {
    let request = request_pda(program_id, requester, sequence);
    let result  = result_pda(program_id, requester, sequence);

    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*coordinator, true),         // coordinator — signer, writable
            AccountMeta::new(request, false),             // randomnessRequest — writable PDA
            AccountMeta::new(result, false),              // randomnessResult — writable PDA
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data: DISC_FINALIZE_RANDOMNESS.to_vec(),
    }
}

/// Build a `claim_rewards` instruction for a single participating node.
///
/// Instruction data: discriminator (8) + device_pubkey (33).
pub fn build_claim_rewards_ix(
    program_id: &Pubkey,
    coordinator: &Pubkey,
    requester: &Pubkey,
    sequence: u64,
    device_pubkey: &[u8; 33],
    node_wallet: &Pubkey,
    treasury: &Pubkey,
    reserve: &Pubkey,
) -> Instruction {
    let request = request_pda(program_id, requester, sequence);
    let result  = result_pda(program_id, requester, sequence);
    let escrow  = escrow_pda(program_id, requester, sequence);

    let mut data = DISC_CLAIM_REWARDS.to_vec();
    data.extend_from_slice(device_pubkey);

    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new_readonly(*coordinator, true),  // coordinator — signer
            AccountMeta::new_readonly(request, false),      // randomnessRequest — PDA
            AccountMeta::new_readonly(result, false),       // randomnessResult — PDA
            AccountMeta::new(escrow, false),                // escrow — writable PDA
            AccountMeta::new(*node_wallet, false),          // nodeWallet — writable
            AccountMeta::new(*treasury, false),             // treasury — writable
            AccountMeta::new(*reserve, false),              // reserve — writable
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data,
    }
}
