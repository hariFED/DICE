use anchor_lang::prelude::*;

/// Records the commit (SHA-256 hash of entropy) submitted by a hardware node.
///
/// Seeds: [SEED_COMMIT, requester, sequence_le, device_pubkey]
/// Space: 8 + 32 + 33 + 32 + 8 = 113
#[account]
pub struct CommitRecord {
    /// The randomness request this commit belongs to
    pub request: Pubkey,
    /// Compressed secp256k1 public key of the submitting device
    pub device_pubkey: [u8; 33],
    /// SHA-256(entropy) — the commitment
    pub commit_hash: [u8; 32],
    /// Slot at which this commit was submitted
    pub submitted_slot: u64,
}

impl CommitRecord {
    pub const LEN: usize = 8 + 32 + 33 + 32 + 8; // 113
}
