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
const SEED_CHANNEL: &[u8] = b"channel";
const SEED_REQUEST: &[u8] = b"request";
const SEED_COMMIT:  &[u8] = b"commit";
const SEED_REVEAL:  &[u8] = b"reveal";
const SEED_RESULT:  &[u8] = b"result";
const SEED_ESCROW:  &[u8] = b"escrow";

// Instruction discriminators — SHA-256("global:<snake_case_name>")[0..8]

// v1.0 (legacy)
const DISC_REQUEST_RANDOMNESS:  [u8; 8] = [213,   5, 173, 166,  37, 236,  31,  18];
const DISC_SUBMIT_COMMIT:       [u8; 8] = [213, 213, 149,  72, 230,  14,  23,  16];
const DISC_SUBMIT_REVEAL:       [u8; 8] = [255, 153,  68,  56, 227,  55,  19, 157];
const DISC_FINALIZE_RANDOMNESS: [u8; 8] = [ 29, 180, 158, 167,  45,  40,   8, 199];
const DISC_CLAIM_REWARDS:       [u8; 8] = [  4, 144, 132,  71, 116,  23, 151,  80];

// v2.0 (channel-based)
const DISC_INIT_CHANNEL:        [u8; 8] = [ 21, 200, 152,  39,  19, 178,  76,  47];
const DISC_FUND_CHANNEL:        [u8; 8] = [ 50,  67,   3,  71, 190, 169,  20, 207];
const DISC_REQUEST_RANDOMNESS_V2: [u8; 8] = [52, 181, 7, 93, 31, 238, 117, 70];
const DISC_SUBMIT_COMMIT_V2:    [u8; 8] = [ 21, 253, 125, 167, 234, 224, 182, 148];
const DISC_SUBMIT_REVEAL_V2:    [u8; 8] = [201,  26,  38, 149, 248, 104, 142,   7];
const DISC_FINALIZE_V2:         [u8; 8] = [101, 248,  68,  53,  48, 110, 126, 211];
const DISC_DELIVER_CALLBACK:    [u8; 8] = [ 75, 178,  82, 149, 119,  28,  57, 254];
const DISC_WITHDRAW_BALANCE:   [u8; 8] = [140,  79,  65,  53,  68,  73, 241, 211];
const DISC_CLOSE_CHANNEL:      [u8; 8] = [  0, 104,  36,   1,  66,   0, 103, 157];
const DISC_RESIZE_CHANNEL:     [u8; 8] = [ 44, 224, 158,  10, 139, 248,  77,  22];
const DISC_SELECT_NODES:       [u8; 8] = [ 45, 118, 200,  26, 149,  67, 185,   1];
const DISC_REQUEST_RANDOMNESS_AUTO: [u8; 8] = [139,  28, 100, 177, 156, 141, 117, 153];

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

// ---------------------------------------------------------------------------
// v2.0 Channel-based instruction builders
// ---------------------------------------------------------------------------

/// Derive the DiceChannel PDA.
pub fn channel_pda(program_id: &Pubkey, authority: &Pubkey, channel_index: u16) -> Pubkey {
    Pubkey::find_program_address(
        &[SEED_CHANNEL, authority.as_ref(), &channel_index.to_le_bytes()],
        program_id,
    )
    .0
}

/// Build `init_channel` instruction.
pub fn build_init_channel_ix(
    program_id: &Pubkey,
    authority: &Pubkey,
    channel_index: u16,
    max_nodes: u8,
    callback_program_id: &Pubkey,
) -> Instruction {
    let channel = channel_pda(program_id, authority, channel_index);
    let mut data = DISC_INIT_CHANNEL.to_vec();
    data.extend_from_slice(&channel_index.to_le_bytes());
    data.push(max_nodes);
    data.extend_from_slice(callback_program_id.as_ref());
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*authority, true),
            AccountMeta::new(channel, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data,
    }
}

/// Build `fund_channel` instruction.
pub fn build_fund_channel_ix(
    program_id: &Pubkey,
    authority: &Pubkey,
    channel_index: u16,
    amount: u64,
) -> Instruction {
    let channel = channel_pda(program_id, authority, channel_index);
    let mut data = DISC_FUND_CHANNEL.to_vec();
    data.extend_from_slice(&amount.to_le_bytes());
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*authority, true),
            AccountMeta::new(channel, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data,
    }
}

/// Build `request_randomness_v2` instruction.
pub fn build_request_randomness_v2_ix(
    program_id: &Pubkey,
    authority: &Pubkey,
    channel_index: u16,
    node_count: u8,
) -> Instruction {
    let channel = channel_pda(program_id, authority, channel_index);
    let mut data = DISC_REQUEST_RANDOMNESS_V2.to_vec();
    data.push(node_count);
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*authority, true),
            AccountMeta::new(channel, false),
        ],
        data,
    }
}

/// Build `submit_commit_v2` instruction.
pub fn build_submit_commit_v2_ix(
    program_id: &Pubkey,
    coordinator: &Pubkey,
    authority: &Pubkey,
    channel_index: u16,
    round_id: u64,
    device_pubkey: &[u8; 33],
    commit_hash: &[u8; 32],
) -> Instruction {
    let device_id = compute_device_id(device_pubkey);
    let channel = channel_pda(program_id, authority, channel_index);
    let mut data = DISC_SUBMIT_COMMIT_V2.to_vec();
    data.extend_from_slice(&round_id.to_le_bytes());
    data.extend_from_slice(&device_id);
    data.extend_from_slice(device_pubkey);
    data.extend_from_slice(commit_hash);
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new_readonly(*coordinator, true),
            AccountMeta::new(channel, false),
        ],
        data,
    }
}

/// Build `submit_reveal_v2` instruction.
pub fn build_submit_reveal_v2_ix(
    program_id: &Pubkey,
    coordinator: &Pubkey,
    authority: &Pubkey,
    channel_index: u16,
    round_id: u64,
    device_pubkey: &[u8; 33],
    entropy: &[u8; 32],
    signature: &[u8; 64],
) -> Instruction {
    let device_id = compute_device_id(device_pubkey);
    let channel = channel_pda(program_id, authority, channel_index);
    let mut data = DISC_SUBMIT_REVEAL_V2.to_vec();
    data.extend_from_slice(&round_id.to_le_bytes());
    data.extend_from_slice(&device_id);
    data.extend_from_slice(device_pubkey);
    data.extend_from_slice(entropy);
    data.extend_from_slice(signature);
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new_readonly(*coordinator, true),
            AccountMeta::new(channel, false),
        ],
        data,
    }
}

/// Build `finalize_v2` instruction.
pub fn build_finalize_v2_ix(
    program_id: &Pubkey,
    coordinator: &Pubkey,
    authority: &Pubkey,
    channel_index: u16,
    round_id: u64,
) -> Instruction {
    let channel = channel_pda(program_id, authority, channel_index);
    let mut data = DISC_FINALIZE_V2.to_vec();
    data.extend_from_slice(&round_id.to_le_bytes());
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new_readonly(*coordinator, true),
            AccountMeta::new(channel, false),
        ],
        data,
    }
}

/// Build `deliver_callback` instruction.
///
/// `remaining_accounts` should contain the callback program as the first entry,
/// followed by any additional accounts the callback needs.
pub fn build_deliver_callback_ix(
    program_id: &Pubkey,
    coordinator: &Pubkey,
    authority: &Pubkey,
    channel_index: u16,
    round_id: u64,
    remaining_accounts: Vec<AccountMeta>,
) -> Instruction {
    let channel = channel_pda(program_id, authority, channel_index);
    let mut data = DISC_DELIVER_CALLBACK.to_vec();
    data.extend_from_slice(&round_id.to_le_bytes());

    let mut accounts = vec![
        AccountMeta::new_readonly(*coordinator, true),
        AccountMeta::new(channel, false),
    ];
    accounts.extend(remaining_accounts);

    Instruction {
        program_id: *program_id,
        accounts,
        data,
    }
}

/// Build `withdraw_balance` instruction.
pub fn build_withdraw_balance_ix(
    program_id: &Pubkey,
    authority: &Pubkey,
    channel_index: u16,
    amount: u64,
) -> Instruction {
    let channel = channel_pda(program_id, authority, channel_index);
    let mut data = DISC_WITHDRAW_BALANCE.to_vec();
    data.extend_from_slice(&amount.to_le_bytes());
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*authority, true),
            AccountMeta::new(channel, false),
        ],
        data,
    }
}

/// Build `close_channel` instruction.
pub fn build_close_channel_ix(
    program_id: &Pubkey,
    authority: &Pubkey,
    channel_index: u16,
) -> Instruction {
    let channel = channel_pda(program_id, authority, channel_index);
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*authority, true),
            AccountMeta::new(channel, false),
        ],
        data: DISC_CLOSE_CHANNEL.to_vec(),
    }
}

/// Build `resize_channel` instruction.
pub fn build_resize_channel_ix(
    program_id: &Pubkey,
    authority: &Pubkey,
    channel_index: u16,
    new_max_nodes: u8,
) -> Instruction {
    let channel = channel_pda(program_id, authority, channel_index);
    let mut data = DISC_RESIZE_CHANNEL.to_vec();
    data.push(new_max_nodes);
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*authority, true),
            AccountMeta::new(channel, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data,
    }
}

/// Build `select_nodes` instruction.
///
/// `device_registry_accounts` are passed as remaining accounts for on-chain selection.
pub fn build_select_nodes_ix(
    program_id: &Pubkey,
    coordinator: &Pubkey,
    authority: &Pubkey,
    channel_index: u16,
    round_id: u64,
    device_registry_accounts: Vec<Pubkey>,
) -> Instruction {
    let channel = channel_pda(program_id, authority, channel_index);
    let mut data = DISC_SELECT_NODES.to_vec();
    data.extend_from_slice(&round_id.to_le_bytes());

    let mut accounts = vec![
        AccountMeta::new_readonly(*coordinator, true),
        AccountMeta::new(channel, false),
    ];
    // Pass all DeviceRegistry PDAs as remaining_accounts (read-only)
    for registry in &device_registry_accounts {
        accounts.push(AccountMeta::new_readonly(*registry, false));
    }

    Instruction {
        program_id: *program_id,
        accounts,
        data,
    }
}

/// Build `request_randomness_auto` instruction.
///
/// Zero-friction: auto-creates channel if needed, auto-funds from authority wallet.
pub fn build_request_randomness_auto_ix(
    program_id: &Pubkey,
    authority: &Pubkey,
    channel_index: u16,
    max_nodes: u8,
    node_count: u8,
    callback_program_id: &Pubkey,
) -> Instruction {
    let channel = channel_pda(program_id, authority, channel_index);
    let mut data = DISC_REQUEST_RANDOMNESS_AUTO.to_vec();
    data.extend_from_slice(&channel_index.to_le_bytes());
    data.push(max_nodes);
    data.push(node_count);
    data.extend_from_slice(callback_program_id.as_ref());
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*authority, true),
            AccountMeta::new(channel, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};
    use std::collections::HashSet;

    /// A deterministic test program ID.
    fn test_program_id() -> Pubkey {
        Pubkey::new_from_array([1u8; 32])
    }

    /// A deterministic test pubkey.
    fn test_pubkey_a() -> Pubkey {
        Pubkey::new_from_array([0xAA; 32])
    }

    fn test_pubkey_b() -> Pubkey {
        Pubkey::new_from_array([0xBB; 32])
    }

    // -----------------------------------------------------------------------
    // compute_device_id
    // -----------------------------------------------------------------------

    #[test]
    fn test_compute_device_id_deterministic() {
        let pk = [0x02u8; 33]; // dummy compressed pubkey
        let id1 = compute_device_id(&pk);
        let id2 = compute_device_id(&pk);
        assert_eq!(id1, id2, "compute_device_id must be deterministic");

        // Verify it matches raw SHA-256
        let expected: [u8; 32] = Sha256::digest(pk).into();
        assert_eq!(id1, expected, "should equal SHA-256(pubkey)");
    }

    #[test]
    fn test_compute_device_id_differs() {
        let pk_a = [0x02u8; 33];
        let mut pk_b = [0x02u8; 33];
        pk_b[32] = 0x03; // flip last byte

        let id_a = compute_device_id(&pk_a);
        let id_b = compute_device_id(&pk_b);
        assert_ne!(
            id_a, id_b,
            "different pubkeys must produce different device IDs"
        );
    }

    // -----------------------------------------------------------------------
    // PDA helpers
    // -----------------------------------------------------------------------

    #[test]
    fn test_request_pda_deterministic() {
        let pid = test_program_id();
        let req = test_pubkey_a();
        let seq = 42u64;

        let pda1 = request_pda(&pid, &req, seq);
        let pda2 = request_pda(&pid, &req, seq);
        assert_eq!(pda1, pda2, "request_pda must be deterministic");

        // Different sequence => different PDA
        let pda3 = request_pda(&pid, &req, seq + 1);
        assert_ne!(pda1, pda3, "different sequence should yield different PDA");
    }

    #[test]
    fn test_channel_pda_deterministic() {
        let pid = test_program_id();
        let auth = test_pubkey_a();
        let idx = 7u16;

        let pda1 = channel_pda(&pid, &auth, idx);
        let pda2 = channel_pda(&pid, &auth, idx);
        assert_eq!(pda1, pda2, "channel_pda must be deterministic");
    }

    #[test]
    fn test_channel_pda_differs_by_index() {
        let pid = test_program_id();
        let auth = test_pubkey_a();

        let pda0 = channel_pda(&pid, &auth, 0);
        let pda1 = channel_pda(&pid, &auth, 1);
        let pda2 = channel_pda(&pid, &auth, 2);

        assert_ne!(pda0, pda1, "different index should yield different PDA");
        assert_ne!(pda1, pda2, "different index should yield different PDA");
        assert_ne!(pda0, pda2, "different index should yield different PDA");
    }

    #[test]
    fn test_channel_pda_differs_by_authority() {
        let pid = test_program_id();
        let auth_a = test_pubkey_a();
        let auth_b = test_pubkey_b();
        let idx = 5u16;

        let pda_a = channel_pda(&pid, &auth_a, idx);
        let pda_b = channel_pda(&pid, &auth_b, idx);
        assert_ne!(
            pda_a, pda_b,
            "different authority should yield different channel PDA"
        );
    }

    // -----------------------------------------------------------------------
    // Instruction data layout tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_build_init_channel_ix_data_layout() {
        let pid = test_program_id();
        let auth = test_pubkey_a();
        let channel_index: u16 = 42;
        let max_nodes: u8 = 7;
        let callback = test_pubkey_b();

        let ix = build_init_channel_ix(&pid, &auth, channel_index, max_nodes, &callback);

        // Total data = 8 (disc) + 2 (channel_index) + 1 (max_nodes) + 32 (callback) = 43
        assert_eq!(ix.data.len(), 43, "init_channel data should be 43 bytes");

        // Discriminator
        assert_eq!(&ix.data[0..8], &DISC_INIT_CHANNEL, "discriminator mismatch");

        // channel_index at offset 8..10 (u16 LE)
        let ci = u16::from_le_bytes([ix.data[8], ix.data[9]]);
        assert_eq!(ci, channel_index, "channel_index in data should match");

        // max_nodes at offset 10
        assert_eq!(ix.data[10], max_nodes, "max_nodes in data should match");

        // callback_program_id at offset 11..43
        assert_eq!(
            &ix.data[11..43],
            callback.as_ref(),
            "callback_program_id in data should match"
        );

        // Verify accounts
        assert_eq!(ix.program_id, pid);
        assert_eq!(ix.accounts.len(), 3);
        assert_eq!(ix.accounts[0].pubkey, auth);
        assert!(ix.accounts[0].is_signer);
    }

    #[test]
    fn test_build_request_randomness_v2_ix_data_layout() {
        let pid = test_program_id();
        let auth = test_pubkey_a();
        let channel_index: u16 = 3;
        let node_count: u8 = 5;

        let ix = build_request_randomness_v2_ix(&pid, &auth, channel_index, node_count);

        // Total data = 8 (disc) + 1 (node_count) = 9
        assert_eq!(ix.data.len(), 9, "request_randomness_v2 data should be 9 bytes");

        assert_eq!(
            &ix.data[0..8],
            &DISC_REQUEST_RANDOMNESS_V2,
            "discriminator mismatch"
        );

        assert_eq!(ix.data[8], node_count, "node_count in data should match");

        // Verify accounts: authority (signer) + channel PDA
        assert_eq!(ix.accounts.len(), 2);
        assert!(ix.accounts[0].is_signer);
        let expected_channel = channel_pda(&pid, &auth, channel_index);
        assert_eq!(ix.accounts[1].pubkey, expected_channel);
    }

    #[test]
    fn test_build_submit_commit_v2_ix_data_layout() {
        let pid = test_program_id();
        let coordinator = test_pubkey_a();
        let authority = test_pubkey_b();
        let channel_index: u16 = 1;
        let round_id: u64 = 999;
        let device_pubkey = [0x03u8; 33];
        let commit_hash = [0xFFu8; 32];

        let ix = build_submit_commit_v2_ix(
            &pid,
            &coordinator,
            &authority,
            channel_index,
            round_id,
            &device_pubkey,
            &commit_hash,
        );

        // Total data = 8 (disc) + 8 (round_id) + 32 (device_id) + 33 (device_pubkey) + 32 (commit_hash) = 113
        assert_eq!(
            ix.data.len(),
            113,
            "submit_commit_v2 data should be 113 bytes"
        );

        // Discriminator
        assert_eq!(&ix.data[0..8], &DISC_SUBMIT_COMMIT_V2, "discriminator mismatch");

        // round_id at offset 8..16 (u64 LE)
        let rid = u64::from_le_bytes(ix.data[8..16].try_into().unwrap());
        assert_eq!(rid, round_id, "round_id in data should match");

        // device_id at offset 16..48 (SHA-256 of device_pubkey)
        let expected_device_id = compute_device_id(&device_pubkey);
        assert_eq!(
            &ix.data[16..48],
            &expected_device_id,
            "device_id in data should match SHA-256(device_pubkey)"
        );

        // device_pubkey at offset 48..81
        assert_eq!(
            &ix.data[48..81],
            &device_pubkey,
            "device_pubkey in data should match"
        );

        // commit_hash at offset 81..113
        assert_eq!(
            &ix.data[81..113],
            &commit_hash,
            "commit_hash in data should match"
        );

        // Verify accounts
        assert_eq!(ix.accounts.len(), 2);
        assert_eq!(ix.accounts[0].pubkey, coordinator);
        assert!(ix.accounts[0].is_signer);
        let expected_channel = channel_pda(&pid, &authority, channel_index);
        assert_eq!(ix.accounts[1].pubkey, expected_channel);
    }

    #[test]
    fn test_build_finalize_v2_ix_data_layout() {
        let pid = test_program_id();
        let coordinator = test_pubkey_a();
        let authority = test_pubkey_b();
        let channel_index: u16 = 10;
        let round_id: u64 = 12345;

        let ix = build_finalize_v2_ix(&pid, &coordinator, &authority, channel_index, round_id);

        // Total data = 8 (disc) + 8 (round_id) = 16
        assert_eq!(ix.data.len(), 16, "finalize_v2 data should be 16 bytes");

        assert_eq!(&ix.data[0..8], &DISC_FINALIZE_V2, "discriminator mismatch");

        let rid = u64::from_le_bytes(ix.data[8..16].try_into().unwrap());
        assert_eq!(rid, round_id, "round_id in data should match");

        // Verify accounts
        assert_eq!(ix.accounts.len(), 2);
        assert!(ix.accounts[0].is_signer);
    }

    #[test]
    fn test_build_request_randomness_auto_ix_data_layout() {
        let pid = test_program_id();
        let auth = test_pubkey_a();
        let channel_index: u16 = 0;
        let max_nodes: u8 = 5;
        let node_count: u8 = 3;
        let callback = test_pubkey_b();

        let ix = build_request_randomness_auto_ix(
            &pid,
            &auth,
            channel_index,
            max_nodes,
            node_count,
            &callback,
        );

        // Total data = 8 (disc) + 2 (channel_index) + 1 (max_nodes) + 1 (node_count) + 32 (callback) = 44
        assert_eq!(
            ix.data.len(),
            44,
            "request_randomness_auto data should be 44 bytes"
        );

        assert_eq!(
            &ix.data[0..8],
            &DISC_REQUEST_RANDOMNESS_AUTO,
            "discriminator mismatch"
        );

        // channel_index at offset 8..10
        let ci = u16::from_le_bytes([ix.data[8], ix.data[9]]);
        assert_eq!(ci, channel_index, "channel_index in data should match");

        // max_nodes at offset 10
        assert_eq!(ix.data[10], max_nodes, "max_nodes in data should match");

        // node_count at offset 11
        assert_eq!(ix.data[11], node_count, "node_count in data should match");

        // callback at offset 12..44
        assert_eq!(
            &ix.data[12..44],
            callback.as_ref(),
            "callback_program_id in data should match"
        );

        // Verify accounts: authority (signer) + channel + system_program
        assert_eq!(ix.accounts.len(), 3);
        assert!(ix.accounts[0].is_signer);
        assert_eq!(ix.accounts[2].pubkey, system_program::id());
    }

    #[test]
    fn test_all_v2_discriminators_unique() {
        let discs: Vec<(&str, [u8; 8])> = vec![
            ("DISC_INIT_CHANNEL", DISC_INIT_CHANNEL),
            ("DISC_FUND_CHANNEL", DISC_FUND_CHANNEL),
            ("DISC_REQUEST_RANDOMNESS_V2", DISC_REQUEST_RANDOMNESS_V2),
            ("DISC_SUBMIT_COMMIT_V2", DISC_SUBMIT_COMMIT_V2),
            ("DISC_SUBMIT_REVEAL_V2", DISC_SUBMIT_REVEAL_V2),
            ("DISC_FINALIZE_V2", DISC_FINALIZE_V2),
            ("DISC_DELIVER_CALLBACK", DISC_DELIVER_CALLBACK),
            ("DISC_WITHDRAW_BALANCE", DISC_WITHDRAW_BALANCE),
            ("DISC_CLOSE_CHANNEL", DISC_CLOSE_CHANNEL),
            ("DISC_RESIZE_CHANNEL", DISC_RESIZE_CHANNEL),
            ("DISC_SELECT_NODES", DISC_SELECT_NODES),
            ("DISC_REQUEST_RANDOMNESS_AUTO", DISC_REQUEST_RANDOMNESS_AUTO),
            // Also include v1.0 discriminators to make sure no collisions across versions
            ("DISC_REQUEST_RANDOMNESS", DISC_REQUEST_RANDOMNESS),
            ("DISC_SUBMIT_COMMIT", DISC_SUBMIT_COMMIT),
            ("DISC_SUBMIT_REVEAL", DISC_SUBMIT_REVEAL),
            ("DISC_FINALIZE_RANDOMNESS", DISC_FINALIZE_RANDOMNESS),
            ("DISC_CLAIM_REWARDS", DISC_CLAIM_REWARDS),
        ];

        let mut seen = HashSet::new();
        for (name, disc) in &discs {
            assert!(
                seen.insert(*disc),
                "discriminator collision detected: {} has the same value as a previously seen discriminator",
                name
            );
        }

        // Sanity: we checked all 17 discriminators
        assert_eq!(seen.len(), 17, "should have checked 17 unique discriminators");
    }
}
