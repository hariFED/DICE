use anchor_lang::prelude::*;

/// Final verifiable randomness output for a request.
///
/// Seeds: [SEED_RESULT, requester, sequence_le]
/// Space: 8 + 32 + 32 + (4 + 7*33) + 8 = 8+32+32+235+8 = 315
#[account]
pub struct RandomnessResult {
    /// The randomness request this result belongs to
    pub request: Pubkey,
    /// Final randomness: SHA-256(entropy1 || entropy2 || ... || entropyN)
    pub randomness: [u8; 32],
    /// Nodes whose entropy contributed to the final value
    pub contributing_nodes: [[u8; 33]; 7],
    /// Number of nodes that actually contributed (0..=7)
    pub contributing_count: u8,
    /// Slot at which the result was finalized
    pub finalized_slot: u64,
}

impl RandomnessResult {
    // 8  discriminator
    // 32 request
    // 32 randomness
    // 7*33 = 231 contributing_nodes (fixed array, max 7)
    // 1  contributing_count
    // 8  finalized_slot
    pub const LEN: usize = 8 + 32 + 32 + 231 + 1 + 8; // 312
}
