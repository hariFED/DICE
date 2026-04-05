//! Request queue for burst handling.
//!
//! When more randomness requests arrive than nodes can process concurrently,
//! they are queued here and dispatched as nodes finish rounds.
//!
//! Each node can handle up to `MAX_CONCURRENT_PER_NODE` rounds simultaneously
//! (matching the firmware's 16-slot job queue on the ESP32-S3).

use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
    time::{Duration, Instant},
};

use tokio::sync::Mutex;
use tracing::{info, warn};

/// Maximum concurrent rounds a single node can handle.
/// Matches `MAX_PENDING_JOBS` in the ESP32 firmware's commit_reveal.c.
pub const MAX_CONCURRENT_PER_NODE: usize = 12;

/// A pending request waiting to be dispatched to a node.
#[derive(Debug, Clone)]
pub struct QueuedRequest {
    /// Unique request identifier (32 bytes).
    pub request_id: [u8; 32],
    /// Monotonic sequence number for PDA derivation.
    pub sequence: u64,
    /// Deadline timestamp (unix seconds).
    pub deadline_ts: u64,
    /// When this request was queued.
    pub queued_at: Instant,
    /// On-chain requester pubkey.
    pub requester: solana_sdk::pubkey::Pubkey,
    /// Database UUID.
    pub db_id: uuid::Uuid,
}

/// Tracks per-node active round counts and the pending request queue.
pub struct RequestQueue {
    /// FIFO queue of requests waiting to be dispatched.
    pending: VecDeque<QueuedRequest>,
    /// Number of currently active (in-progress) rounds per node.
    active_rounds: HashMap<[u8; 33], usize>,
    /// Total requests dispatched (lifetime counter).
    pub total_dispatched: u64,
    /// Total requests queued (waiting, not yet dispatched).
    pub total_queued: u64,
    /// Total requests dropped (expired in queue).
    pub total_dropped: u64,
    /// Maximum time a request can wait in queue before being dropped.
    pub max_queue_wait: Duration,
}

pub type SharedQueue = Arc<Mutex<RequestQueue>>;

impl RequestQueue {
    pub fn new() -> Self {
        Self {
            pending: VecDeque::with_capacity(256),
            active_rounds: HashMap::new(),
            total_dispatched: 0,
            total_queued: 0,
            total_dropped: 0,
            max_queue_wait: Duration::from_secs(60),
        }
    }

    /// How many active rounds does this node have?
    pub fn node_active_count(&self, node_id: &[u8; 33]) -> usize {
        *self.active_rounds.get(node_id).unwrap_or(&0)
    }

    /// Can this node accept another round?
    pub fn node_has_capacity(&self, node_id: &[u8; 33]) -> bool {
        self.node_active_count(node_id) < MAX_CONCURRENT_PER_NODE
    }

    /// Record that a round was dispatched to a node.
    pub fn mark_dispatched(&mut self, node_id: &[u8; 33]) {
        *self.active_rounds.entry(*node_id).or_insert(0) += 1;
        self.total_dispatched += 1;
    }

    /// Record that a round completed (finalized or failed) for a node.
    /// Returns any queued requests that can now be dispatched.
    pub fn mark_completed(&mut self, node_id: &[u8; 33]) -> Vec<QueuedRequest> {
        if let Some(count) = self.active_rounds.get_mut(node_id) {
            *count = count.saturating_sub(1);
        }

        // Drain expired requests first
        self.drop_expired();

        // Collect requests that can now be dispatched (node has capacity)
        let mut ready = Vec::new();
        while self.node_has_capacity(node_id) {
            if let Some(req) = self.pending.pop_front() {
                *self.active_rounds.entry(*node_id).or_insert(0) += 1;
                self.total_dispatched += 1;
                ready.push(req);
            } else {
                break;
            }
        }

        ready
    }

    /// Add a request to the queue.
    pub fn enqueue(&mut self, request: QueuedRequest) {
        self.total_queued += 1;
        self.pending.push_back(request);
    }

    /// Number of requests waiting in the queue.
    pub fn queue_len(&self) -> usize {
        self.pending.len()
    }

    /// Drop requests that have been waiting too long.
    fn drop_expired(&mut self) {
        let now = Instant::now();
        let before = self.pending.len();
        self.pending.retain(|req| {
            now.duration_since(req.queued_at) < self.max_queue_wait
        });
        let dropped = before - self.pending.len();
        if dropped > 0 {
            self.total_dropped += dropped as u64;
            warn!(dropped, remaining = self.pending.len(), "dropped expired queued requests");
        }
    }

    /// Reset active count for a node (e.g., on disconnect — all its rounds are gone).
    pub fn node_disconnected(&mut self, node_id: &[u8; 33]) {
        self.active_rounds.remove(node_id);
    }
}

/// Create a new shared request queue.
pub fn new_queue() -> SharedQueue {
    Arc::new(Mutex::new(RequestQueue::new()))
}
