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
    /// Authorized coordinator — required signer on submit_commit, submit_reveal,
    /// finalize, select_nodes, deliver_callback. Set at init_channel.
    pub coordinator: Pubkey,
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
        + 32  // coordinator
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

#[cfg(test)]
mod tests {
    use super::*;
    use anchor_lang::prelude::Pubkey;

    /// Helper: build a DiceChannel with sensible defaults for the given max_nodes.
    fn make_channel(max_nodes: u8) -> DiceChannel {
        let n = max_nodes as usize;
        DiceChannel {
            authority: Pubkey::new_unique(),
            coordinator: Pubkey::new_unique(),
            channel_index: 0,
            max_nodes,
            status: ChannelStatus::Idle,
            round_id: 0,
            node_count: 0,
            commits_received: 0,
            reveals_received: 0,
            created_slot: 0,
            commit_deadline_slot: 0,
            reveal_deadline_slot: 0,
            balance: 0,
            callback_program_id: Pubkey::default(),
            randomness: [0u8; 32],
            device_ids: vec![[0u8; 32]; n],
            device_pubkeys: vec![[0u8; 33]; n],
            commit_hashes: vec![[0u8; 32]; n],
            entropies: vec![[0u8; 32]; n],
            signatures: vec![[0u8; 64]; n],
        }
    }

    // ── space() tests ────────────────────────────────────────────────────

    #[test]
    fn test_space_calculation_min_nodes() {
        // max_nodes = 4 (MIN_NODES_REQUIRED)
        // Fixed: 8 (disc) + 32 (authority) + 32 (coordinator) + 2 (channel_index) + 1 (max_nodes)
        //   + 1 (status) + 8 (round_id) + 1 (node_count) + 1 (commits_received)
        //   + 1 (reveals_received) + 8 (created_slot) + 8 (commit_deadline_slot)
        //   + 8 (reveal_deadline_slot) + 8 (balance) + 32 (callback_program_id)
        //   + 32 (randomness) = 183 bytes
        // Vec length prefixes: 5 * 4 = 20
        // Per-node data: 4 * (32 + 33 + 32 + 32 + 64) = 4 * 193 = 772
        // Total: 183 + 20 + 772 = 975
        let fixed = 183usize;
        let vec_prefix = 20usize;
        let space = DiceChannel::space(4);
        assert_eq!(space, fixed + vec_prefix + 4 * 193);
        assert_eq!(space, 975);
    }

    #[test]
    fn test_space_calculation_max_nodes() {
        // max_nodes = 50 (MAX_CHANNEL_NODES)
        // Total: 183 + 20 + 50 * 193 = 9853
        let space = DiceChannel::space(50);
        assert_eq!(space, 183 + 20 + 50 * 193);
        assert_eq!(space, 9853);
        // Must fit within Solana's 10 KB account init limit (10240 bytes)
        assert!(space <= 10_240, "space for 50 nodes must fit in 10 KB");
    }

    #[test]
    fn test_space_calculation_single_node() {
        // max_nodes = 1 — won't pass validation (min is 4) but space() should compute
        let space = DiceChannel::space(1);
        assert_eq!(space, 183 + 20 + 1 * 193);
        assert_eq!(space, 396);
    }

    #[test]
    fn test_space_zero_nodes() {
        // Edge case: 0 nodes. Only fixed overhead + vec prefixes, no per-node data.
        let space = DiceChannel::space(0);
        assert_eq!(space, 183 + 20);
        assert_eq!(space, 203);
    }

    #[test]
    fn test_space_monotonically_increases() {
        // Every additional node should add exactly 193 bytes.
        for n in 1..=49u8 {
            let prev = DiceChannel::space(n - 1);
            let curr = DiceChannel::space(n);
            assert_eq!(curr - prev, 193, "each node adds exactly 193 bytes");
        }
    }

    // ── reset_for_new_round() tests ──────────────────────────────────────

    #[test]
    fn test_reset_clears_all_round_data() {
        let mut ch = make_channel(4);

        // Simulate a round in progress with dirty data
        ch.status = ChannelStatus::RevealPhase;
        ch.round_id = 5;
        ch.node_count = 4;
        ch.commits_received = 3;
        ch.reveals_received = 2;
        ch.randomness = [0xAB; 32];
        ch.device_ids[0] = [0xFF; 32];
        ch.device_pubkeys[0] = [0xEE; 33];
        ch.commit_hashes[0] = [0xDD; 32];
        ch.entropies[0] = [0xCC; 32];
        ch.signatures[0] = [0xBB; 64];

        ch.reset_for_new_round(4);

        // Round counters should be cleared
        assert_eq!(ch.commits_received, 0);
        assert_eq!(ch.reveals_received, 0);
        assert_eq!(ch.randomness, [0u8; 32]);

        // Status should be Pending
        assert_eq!(ch.status, ChannelStatus::Pending);

        // All inline arrays should be zeroed
        for i in 0..4 {
            assert_eq!(ch.device_ids[i], [0u8; 32], "device_ids[{}] not zeroed", i);
            assert_eq!(ch.device_pubkeys[i], [0u8; 33], "device_pubkeys[{}] not zeroed", i);
            assert_eq!(ch.commit_hashes[i], [0u8; 32], "commit_hashes[{}] not zeroed", i);
            assert_eq!(ch.entropies[i], [0u8; 32], "entropies[{}] not zeroed", i);
            assert_eq!(ch.signatures[i], [0u8; 64], "signatures[{}] not zeroed", i);
        }
    }

    #[test]
    fn test_reset_increments_round_id() {
        let mut ch = make_channel(4);
        assert_eq!(ch.round_id, 0);

        ch.reset_for_new_round(4);
        assert_eq!(ch.round_id, 1);

        ch.reset_for_new_round(4);
        assert_eq!(ch.round_id, 2);

        ch.reset_for_new_round(4);
        assert_eq!(ch.round_id, 3);

        // Multiple resets should monotonically increase
        for expected in 4..=20u64 {
            ch.reset_for_new_round(4);
            assert_eq!(ch.round_id, expected);
        }
    }

    #[test]
    fn test_reset_preserves_identity() {
        let authority = Pubkey::new_unique();
        let callback = Pubkey::new_unique();
        let mut ch = make_channel(8);
        ch.authority = authority;
        ch.channel_index = 42;
        ch.max_nodes = 8;
        ch.balance = 10_000_000;
        ch.callback_program_id = callback;
        ch.created_slot = 12345;
        ch.commit_deadline_slot = 12500;
        ch.reveal_deadline_slot = 12650;

        ch.reset_for_new_round(6);

        // Identity fields must NOT change
        assert_eq!(ch.authority, authority, "authority must survive reset");
        assert_eq!(ch.channel_index, 42, "channel_index must survive reset");
        assert_eq!(ch.max_nodes, 8, "max_nodes must survive reset");
        assert_eq!(ch.balance, 10_000_000, "balance must survive reset");
        assert_eq!(ch.callback_program_id, callback, "callback_program_id must survive reset");

        // Timing fields are NOT reset by reset_for_new_round (they are set by the
        // instruction handler), so they should also survive.
        assert_eq!(ch.created_slot, 12345);
        assert_eq!(ch.commit_deadline_slot, 12500);
        assert_eq!(ch.reveal_deadline_slot, 12650);
    }

    #[test]
    fn test_reset_allocates_correct_vec_sizes() {
        // Vecs should always be sized to max_nodes, not node_count.
        let mut ch = make_channel(10);
        ch.reset_for_new_round(5); // requesting 5 but max is 10

        assert_eq!(ch.device_ids.len(), 10);
        assert_eq!(ch.device_pubkeys.len(), 10);
        assert_eq!(ch.commit_hashes.len(), 10);
        assert_eq!(ch.entropies.len(), 10);
        assert_eq!(ch.signatures.len(), 10);

        // node_count should be set to the requested value, not max
        assert_eq!(ch.node_count, 5);
    }

    #[test]
    fn test_reset_sets_node_count() {
        let mut ch = make_channel(20);
        ch.reset_for_new_round(7);
        assert_eq!(ch.node_count, 7);

        ch.reset_for_new_round(15);
        assert_eq!(ch.node_count, 15);
    }

    // ── ChannelStatus enum tests ─────────────────────────────────────────

    #[test]
    fn test_channel_status_enum_values() {
        use anchor_lang::AnchorSerialize;

        // All 6 variants should round-trip through borsh serialization.
        let variants = [
            ChannelStatus::Idle,
            ChannelStatus::Pending,
            ChannelStatus::CommitPhase,
            ChannelStatus::RevealPhase,
            ChannelStatus::Finalized,
            ChannelStatus::Failed,
        ];

        for (i, variant) in variants.iter().enumerate() {
            let mut buf = Vec::new();
            variant.serialize(&mut buf).expect("serialize should succeed");

            // Borsh encodes C-like enums as a single byte (u8 discriminant)
            assert_eq!(buf.len(), 1, "ChannelStatus should serialize to 1 byte");
            assert_eq!(buf[0], i as u8, "variant index mismatch for {:?}", variant);

            // Deserialize back
            let decoded = ChannelStatus::deserialize(&mut &buf[..])
                .expect("deserialize should succeed");
            assert_eq!(*variant, decoded, "round-trip failed for {:?}", variant);
        }
    }

    #[test]
    fn test_channel_status_equality() {
        assert_eq!(ChannelStatus::Idle, ChannelStatus::Idle);
        assert_ne!(ChannelStatus::Idle, ChannelStatus::Pending);
        assert_ne!(ChannelStatus::CommitPhase, ChannelStatus::RevealPhase);
        assert_ne!(ChannelStatus::Finalized, ChannelStatus::Failed);
    }

    #[test]
    fn test_channel_status_clone_and_copy() {
        let status = ChannelStatus::RevealPhase;
        let cloned = status.clone();
        let copied = status; // Copy
        assert_eq!(status, cloned);
        assert_eq!(status, copied);
    }
}
