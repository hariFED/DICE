use anchor_lang::prelude::*;

/// The lifecycle status of a randomness request.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq)]
pub enum RequestStatus {
    Pending,
    CommitPhase,
    RevealPhase,
    Finalized,
    Failed,
}

impl RequestStatus {
    pub const LEN: usize = 1 + 1; // discriminant byte + largest variant (all unit variants = 1)
}

/// On-chain state for a single randomness request.
///
/// Seeds: [SEED_REQUEST, requester, sequence.to_le_bytes()]
/// Space: 8 + 32 + 8 + 2 + (4 + 7*33) + 1 + 1 + 8 + 8 + 8 = 8+32+8+2+235+1+1+8+8+8 = 311
#[account]
pub struct RandomnessRequest {
    /// The developer/user who requested randomness
    pub requester: Pubkey,
    /// Monotonically-increasing per-requester counter (provided by caller)
    pub sequence: u64,
    /// Current lifecycle status
    pub status: RequestStatus,
    /// Selected hardware nodes (up to MAX_NODES_SELECTED = 7) stored as compressed pubkeys
    pub selected_nodes: [[u8; 33]; 7],
    /// Number of nodes actually selected (0..=7)
    pub node_count: u8,
    /// Number of commit messages received so far
    pub commits_received: u8,
    /// Number of reveal messages received so far
    pub reveals_received: u8,
    /// Slot at which the request was created
    pub created_slot: u64,
    /// Last slot at which commits are accepted
    pub commit_deadline_slot: u64,
    /// Last slot at which reveals are accepted
    pub reveal_deadline_slot: u64,
    /// Program to CPI-invoke with the randomness after finalization.
    /// Set to `Pubkey::default()` if no callback is desired (developer polls instead).
    pub callback_program_id: Pubkey,
}

impl RandomnessRequest {
    // 8  discriminator
    // 32 requester
    // 8  sequence
    // 1  status
    // 7*33 = 231 selected_nodes
    // 1  node_count
    // 1  commits_received
    // 1  reveals_received
    // 8  created_slot
    // 8  commit_deadline_slot
    // 8  reveal_deadline_slot
    // 32 callback_program_id
    pub const LEN: usize = 8 + 32 + 8 + 1 + 231 + 1 + 1 + 1 + 8 + 8 + 8 + 32; // 339
}
