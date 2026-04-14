use anchor_lang::prelude::*;

use crate::constants::FEED_HISTORY_SIZE;

/// Lifecycle status of a RandomnessFeed.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum FeedStatus {
    /// Feed is live and accepting publishes.
    Active,
    /// Feed was closed by its authority. No further publishes allowed.
    Closed,
}

/// One entry in a RandomnessFeed's history ring buffer.
///
/// Fixed-size struct — no Vec, no borsh length prefix. 48 bytes on the wire.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Default, Debug)]
pub struct FeedHistoryEntry {
    /// Randomness value that was current at `slot`.
    pub randomness: [u8; 32],
    /// Monotonic sequence number at the time of publish.
    pub sequence: u64,
    /// Solana slot at which this value was published.
    pub slot: u64,
}

/// A streaming randomness feed.
///
/// The coordinator publishes new values on a cadence (`publish_interval_slots`).
/// Subscriber dApps read this account as a passive input to their own instructions
/// — no callback, no per-request transaction. Every published value must come from
/// a real finalized DICE round on the `bound_channel`, so the hardware-backed audit
/// trail is preserved end-to-end.
///
/// Seeds: `["feed", authority, &feed_index.to_le_bytes()]`
#[account]
pub struct RandomnessFeed {
    // ── Identity ──────────────────────────────────────────────────────────
    /// Feed owner — paid rent, can close the feed, receives reclaimed SOL on close.
    pub authority: Pubkey,
    /// Authorized coordinator — only this key can call `publish_feed_value`.
    pub coordinator: Pubkey,
    /// Bound DiceChannel — every published value must originate from this channel.
    pub bound_channel: Pubkey,
    /// Index for multiple feeds per authority (0, 1, 2, ...).
    pub feed_index: u16,
    /// Human-readable feed name (ASCII/UTF-8, zero-padded).
    pub name: [u8; 32],
    /// Target cadence between publishes, in slots. Enforced as a rate limit.
    pub publish_interval_slots: u32,

    // ── Current state ─────────────────────────────────────────────────────
    /// Lifecycle status.
    pub status: FeedStatus,
    /// Most recently published randomness value.
    pub current_randomness: [u8; 32],
    /// Monotonic publish counter (0 = never published, 1 = first publish, ...).
    pub current_sequence: u64,
    /// Slot at which `current_randomness` was published.
    pub current_slot: u64,
    /// DiceChannel round_id that produced `current_randomness`.
    pub current_round_id: u64,

    // ── Previous value ────────────────────────────────────────────────────
    /// Value that was current immediately before the latest publish. Lets
    /// subscribers verify monotonic progression without walking the ring.
    pub prev_randomness: [u8; 32],
    /// Sequence number of the previous value.
    pub prev_sequence: u64,

    // ── History ring buffer ───────────────────────────────────────────────
    /// Index into `history[]` where the next entry will be written.
    pub history_head: u8,
    /// Ring buffer of the last FEED_HISTORY_SIZE published values. A subscriber
    /// reading a few slots late can find the value that was current when their
    /// transaction was built.
    pub history: [FeedHistoryEntry; FEED_HISTORY_SIZE],

    // ── Stats ─────────────────────────────────────────────────────────────
    /// Lifetime publish counter (equals `current_sequence` unless the feed was reset).
    pub total_published: u64,
    /// Slot at which the feed was created.
    pub created_slot: u64,
}

impl RandomnessFeed {
    /// Fixed on-wire size — no Vec fields, so this is a compile-time constant.
    ///
    /// 8 (discriminator)
    /// + 32 (authority) + 32 (coordinator) + 32 (bound_channel)
    /// + 2 (feed_index) + 32 (name) + 4 (publish_interval_slots)
    /// + 1 (status)
    /// + 32 (current_randomness) + 8 (current_sequence) + 8 (current_slot) + 8 (current_round_id)
    /// + 32 (prev_randomness) + 8 (prev_sequence)
    /// + 1 (history_head) + FEED_HISTORY_SIZE * 48 (history entries)
    /// + 8 (total_published) + 8 (created_slot)
    pub const fn space() -> usize {
        8  // discriminator
        + 32 // authority
        + 32 // coordinator
        + 32 // bound_channel
        + 2  // feed_index
        + 32 // name
        + 4  // publish_interval_slots
        + 1  // status
        + 32 // current_randomness
        + 8  // current_sequence
        + 8  // current_slot
        + 8  // current_round_id
        + 32 // prev_randomness
        + 8  // prev_sequence
        + 1  // history_head
        + FEED_HISTORY_SIZE * (32 + 8 + 8) // history entries
        + 8  // total_published
        + 8  // created_slot
    }

    /// Record a newly published value: advance current, snapshot prev, push history.
    ///
    /// Caller must have already validated rate limits, bound-channel match, and status.
    pub fn record_publish(&mut self, randomness: [u8; 32], round_id: u64, slot: u64) {
        // Snapshot current → prev
        self.prev_randomness = self.current_randomness;
        self.prev_sequence = self.current_sequence;

        // Advance current
        self.current_randomness = randomness;
        self.current_sequence = self.current_sequence.saturating_add(1);
        self.current_slot = slot;
        self.current_round_id = round_id;

        // Append to history ring
        let head = self.history_head as usize;
        self.history[head] = FeedHistoryEntry {
            randomness,
            sequence: self.current_sequence,
            slot,
        };
        self.history_head = ((head + 1) % FEED_HISTORY_SIZE) as u8;

        self.total_published = self.total_published.saturating_add(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_feed() -> RandomnessFeed {
        RandomnessFeed {
            authority: Pubkey::new_unique(),
            coordinator: Pubkey::new_unique(),
            bound_channel: Pubkey::new_unique(),
            feed_index: 0,
            name: [0u8; 32],
            publish_interval_slots: 10,
            status: FeedStatus::Active,
            current_randomness: [0u8; 32],
            current_sequence: 0,
            current_slot: 0,
            current_round_id: 0,
            prev_randomness: [0u8; 32],
            prev_sequence: 0,
            history_head: 0,
            history: [FeedHistoryEntry::default(); FEED_HISTORY_SIZE],
            total_published: 0,
            created_slot: 1000,
        }
    }

    #[test]
    fn space_is_constant_and_under_10kb() {
        let s = RandomnessFeed::space();
        // 8 (disc)
        // + 32 (authority) + 32 (coordinator) + 32 (bound_channel)     = 96
        // + 2 (feed_index) + 32 (name) + 4 (publish_interval_slots)    = 38
        // + 1 (status)                                                  = 1
        // + 32 (current_randomness) + 8 (current_sequence)
        //   + 8 (current_slot) + 8 (current_round_id)                   = 56
        // + 32 (prev_randomness) + 8 (prev_sequence)                    = 40
        // + 1 (history_head) + 16 * 48 (history entries: 32+8+8)        = 769
        // + 8 (total_published) + 8 (created_slot)                      = 16
        // Total: 8 + 96 + 38 + 1 + 56 + 40 + 769 + 16 = 1024
        assert_eq!(s, 1024);
        assert!(s <= 10_240, "feed must fit in Solana init limit");
    }

    #[test]
    fn first_publish_advances_sequence_to_one() {
        let mut feed = fresh_feed();
        let r = [0xAA; 32];
        feed.record_publish(r, 42, 5000);

        assert_eq!(feed.current_sequence, 1);
        assert_eq!(feed.current_randomness, r);
        assert_eq!(feed.current_round_id, 42);
        assert_eq!(feed.current_slot, 5000);
        assert_eq!(feed.total_published, 1);
        assert_eq!(feed.history_head, 1, "head should advance after write");
        assert_eq!(feed.history[0].randomness, r);
        assert_eq!(feed.history[0].sequence, 1);
        assert_eq!(feed.prev_randomness, [0u8; 32], "prev was pre-genesis");
        assert_eq!(feed.prev_sequence, 0);
    }

    #[test]
    fn second_publish_snapshots_prev() {
        let mut feed = fresh_feed();
        feed.record_publish([0x11; 32], 1, 100);
        feed.record_publish([0x22; 32], 2, 110);

        assert_eq!(feed.current_randomness, [0x22; 32]);
        assert_eq!(feed.current_sequence, 2);
        assert_eq!(feed.prev_randomness, [0x11; 32]);
        assert_eq!(feed.prev_sequence, 1);
        assert_eq!(feed.total_published, 2);
    }

    #[test]
    fn history_ring_wraps_cleanly() {
        let mut feed = fresh_feed();
        // Fill the ring plus a few more so it wraps
        for i in 0..(FEED_HISTORY_SIZE + 5) as u64 {
            let mut r = [0u8; 32];
            r[0] = i as u8;
            feed.record_publish(r, i, 1000 + i);
        }

        // After FEED_HISTORY_SIZE + 5 publishes:
        //   - current_sequence = FEED_HISTORY_SIZE + 5
        //   - history_head should have wrapped to 5
        assert_eq!(feed.current_sequence, (FEED_HISTORY_SIZE + 5) as u64);
        assert_eq!(feed.history_head, 5);

        // The most recently written slot (index 4) holds the latest value
        let latest = feed.history[4];
        assert_eq!(latest.sequence, (FEED_HISTORY_SIZE + 5) as u64);

        // The oldest surviving entry is at index 5 (sequence = 6)
        let oldest = feed.history[5];
        assert_eq!(oldest.sequence, 6);
    }

    #[test]
    fn status_round_trips_through_borsh() {
        for variant in [FeedStatus::Active, FeedStatus::Closed] {
            let mut buf = Vec::new();
            variant.serialize(&mut buf).unwrap();
            let decoded = FeedStatus::deserialize(&mut &buf[..]).unwrap();
            assert_eq!(variant, decoded);
        }
    }

    #[test]
    fn history_entry_default_is_zero() {
        let e = FeedHistoryEntry::default();
        assert_eq!(e.randomness, [0u8; 32]);
        assert_eq!(e.sequence, 0);
        assert_eq!(e.slot, 0);
    }
}
