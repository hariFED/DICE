use anchor_lang::solana_program::hash::hashv;

/// Compute a 32-byte PDA-safe device identifier from a 33-byte compressed secp256k1 pubkey.
/// `device_id = SHA-256(device_pubkey)`
pub fn device_id(device_pubkey: &[u8; 33]) -> [u8; 32] {
    hashv(&[device_pubkey]).to_bytes()
}

// These seed values MUST match sdk/dice-vrf/src/pda.rs exactly
pub const SEED_DEVICE:  &[u8] = b"device";
pub const SEED_REQUEST: &[u8] = b"request";
pub const SEED_COMMIT:  &[u8] = b"commit";
pub const SEED_REVEAL:  &[u8] = b"reveal";
pub const SEED_RESULT:  &[u8] = b"result";
pub const SEED_ESCROW:  &[u8] = b"escrow";
pub const SEED_CHANNEL: &[u8] = b"channel";

pub const REQUEST_FEE_LAMPORTS: u64 = 2_000_000; // 0.002 SOL
pub const NODE_REWARD_BPS:      u64 = 7_000;     // 70%
pub const TREASURY_REWARD_BPS:  u64 = 2_000;     // 20%
pub const RESERVE_REWARD_BPS:   u64 = 1_000;     // 10%
pub const MIN_NODES_REQUIRED:   u8  = 4;
pub const MAX_NODES_SELECTED:   u8  = 50;
pub const COMMIT_TIMEOUT_SLOTS: u64 = 150; // ~60 seconds
pub const REVEAL_TIMEOUT_SLOTS: u64 = 150;
