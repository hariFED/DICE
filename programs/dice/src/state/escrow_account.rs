use anchor_lang::prelude::*;

/// Holds the 0.002 SOL fee until rewards are distributed.
///
/// Seeds: [SEED_ESCROW, requester, sequence_le]
/// Space: 8 + 32 + 8 + 8 + 1 = 57
#[account]
pub struct EscrowAccount {
    /// The developer who paid the request fee
    pub requester: Pubkey,
    /// Sequence number matching the associated randomness request
    pub sequence: u64,
    /// Lamports held in escrow
    pub amount: u64,
    /// Whether the escrow has been fully claimed/distributed
    pub is_claimed: bool,
}

impl EscrowAccount {
    pub const LEN: usize = 8 + 32 + 8 + 8 + 1; // 57
}
