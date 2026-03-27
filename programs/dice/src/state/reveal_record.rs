use anchor_lang::prelude::*;

/// Records the revealed entropy and ECDSA signature from a hardware node.
///
/// Seeds: [SEED_REVEAL, requester, sequence_le, device_pubkey]
/// Space: 8 + 32 + 33 + 32 + 64 + 8 = 177
#[account]
pub struct RevealRecord {
    /// The randomness request this reveal belongs to
    pub request: Pubkey,
    /// Compressed secp256k1 public key of the submitting device
    pub device_pubkey: [u8; 33],
    /// The raw entropy bytes
    pub entropy: [u8; 32],
    /// ECDSA secp256k1 signature over keccak256(entropy)
    pub signature: [u8; 64],
    /// Slot at which this reveal was submitted
    pub submitted_slot: u64,
}

impl RevealRecord {
    pub const LEN: usize = 8 + 32 + 33 + 32 + 64 + 8; // 177
}
