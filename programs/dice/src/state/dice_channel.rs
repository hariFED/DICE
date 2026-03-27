use anchor_lang::prelude::*;

/// Maximum nodes a channel can support (Solana 10 KB account init limit).
pub const MAX_CHANNEL_NODES: usize = 50;

/// Lifecycle status of a DiceChannel.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ChannelStatus {
    /// Ready for a new request.
    Idle,
    /// Request submitted, waiting for commits.
    Pending,
    /// At least one commit received.
    CommitPhase,
    /// All commits received, waiting for reveals.
    RevealPhase,
    /// Randomness computed and written. Awaiting callback delivery (if any).
    Finalized,
    /// Round failed (timeout or insufficient nodes).
    Failed,
}

/// A persistent, reusable PDA that holds everything for one randomness round at a time.
///
/// The developer creates this once with `init_channel` and reuses it for every request.
/// Commits, reveals, and the result are stored inline — no separate PDAs per node.
///
/// Seeds: `["channel", authority, &channel_index.to_le_bytes()]`
#[account]
pub struct DiceChannel {
    // ── Identity ──────────────────────────────────────────────────────────
    /// Channel owner — required signer on request_randomness, fund, withdraw, close.
    pub authority: Pubkey,
    /// Index for multiple channels per developer (0, 1, 2, ...).
    pub channel_index: u16,
    /// Maximum nodes this channel supports (set at init, changeable via resize).
    pub max_nodes: u8,

    // ── Round state ───────────────────────────────────────────────────────
    /// Current lifecycle status.
    pub status: ChannelStatus,
    /// Monotonically increasing counter — increments on every request_randomness.
    /// Prevents replay of commits/reveals from previous rounds.
    pub round_id: u64,
    /// Number of nodes requested for the current round.
    pub node_count: u8,
    /// Number of commits received so far.
    pub commits_received: u8,
    /// Number of reveals received so far.
    pub reveals_received: u8,

    // ── Timing ────────────────────────────────────────────────────────────
    /// Slot at which the current request was created.
    pub created_slot: u64,
    /// Last slot for commits.
    pub commit_deadline_slot: u64,
    /// Last slot for reveals.
    pub reveal_deadline_slot: u64,

    // ── Economics ──────────────────────────────────────────────────────────
    /// Prepaid protocol fee balance (lamports). Deducted on each request.
    pub balance: u64,

    // ── Callback ──────────────────────────────────────────────────────────
    /// Program to CPI-invoke with the result. Pubkey::default() = no callback.
    pub callback_program_id: Pubkey,

    // ── Result ────────────────────────────────────────────────────────────
    /// Final randomness from the last finalized round.
    pub randomness: [u8; 32],

    // ── Inline arrays (variable size based on max_nodes) ──────────────────
    // These are stored as Vec but sized at init. Anchor serializes Vec with a
    // 4-byte length prefix.
    //
    // Using Vec instead of fixed arrays because max_nodes varies per channel.

    /// Selected device IDs for the current round (SHA-256 of device pubkeys).
    pub device_ids: Vec<[u8; 32]>,
    /// Compressed secp256k1 public keys of selected devices.
    pub device_pubkeys: Vec<[u8; 33]>,
    /// Commit hashes: SHA-256(entropy) per node.
    pub commit_hashes: Vec<[u8; 32]>,
    /// Revealed entropy values per node.
    pub entropies: Vec<[u8; 32]>,
    /// ECDSA signatures over entropy per node.
    pub signatures: Vec<[u8; 64]>,
}

impl DiceChannel {
    /// Compute the account size for a given max_nodes.
    ///
    /// Fixed fields: 8 (disc) + 32 (authority) + 2 (channel_index) + 1 (max_nodes)
    ///   + 1 (status) + 8 (round_id) + 1 (node_count) + 1 (commits_received)
    ///   + 1 (reveals_received) + 8 (created_slot) + 8 (commit_deadline_slot)
    ///   + 8 (reveal_deadline_slot) + 8 (balance) + 32 (callback_program_id)
    ///   + 32 (randomness) = 143 bytes
    ///
    /// Per-node arrays (each has 4-byte Vec length prefix):
    ///   device_ids:     4 + N*32
    ///   device_pubkeys: 4 + N*33
    ///   commit_hashes:  4 + N*32
    ///   entropies:      4 + N*32
    ///   signatures:     4 + N*64
    ///   Total per-node: 20 + N*193
    pub fn space(max_nodes: u8) -> usize {
        let n = max_nodes as usize;
        8   // discriminator
        + 32  // authority
        + 2   // channel_index
        + 1   // max_nodes
        + 1   // status
        + 8   // round_id
        + 1   // node_count
        + 1   // commits_received
        + 1   // reveals_received
        + 8   // created_slot
        + 8   // commit_deadline_slot
        + 8   // reveal_deadline_slot
        + 8   // balance
        + 32  // callback_program_id
        + 32  // randomness
        + (4 + n * 32)  // device_ids
        + (4 + n * 33)  // device_pubkeys
        + (4 + n * 32)  // commit_hashes
        + (4 + n * 32)  // entropies
        + (4 + n * 64)  // signatures
    }

    /// Reset all round-specific data for a new request.
    pub fn reset_for_new_round(&mut self, node_count: u8) {
        self.status = ChannelStatus::Pending;
        self.round_id += 1;
        self.node_count = node_count;
        self.commits_received = 0;
        self.reveals_received = 0;
        self.randomness = [0u8; 32];

        let n = self.max_nodes as usize;
        // Zero all inline arrays
        self.device_ids = vec![[0u8; 32]; n];
        self.device_pubkeys = vec![[0u8; 33]; n];
        self.commit_hashes = vec![[0u8; 32]; n];
        self.entropies = vec![[0u8; 32]; n];
        self.signatures = vec![[0u8; 64]; n];
    }
}
