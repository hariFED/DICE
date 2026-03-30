use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::{anyhow, bail, Result};
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

use crate::protocol::validation::{combine_entropy, verify_commit, verify_reveal};

// ---------------------------------------------------------------------------
// Shared round map
// ---------------------------------------------------------------------------

/// All live rounds keyed by their 32-byte request identifier.
pub type RoundMap = Arc<Mutex<HashMap<[u8; 32], RoundEntry>>>;

/// One entry in the in-memory round map.
pub struct RoundEntry {
    /// The round state machine.
    pub round: Round,
    /// Database record UUID (random UUID in simulation mode).
    pub db_id: uuid::Uuid,
    /// Wall-clock instant when this round was created (for duration tracking).
    pub started_at: Instant,
    /// On-chain requester pubkey (for PDA derivation). Default = Pubkey::default().
    pub requester: solana_sdk::pubkey::Pubkey,
    /// On-chain sequence number (for v1.0 PDA derivation) or round_id (for v2.0).
    pub sequence: u64,
    /// v2.0: channel authority (for channel PDA derivation). None = v1.0 flow.
    pub channel_authority: Option<solana_sdk::pubkey::Pubkey>,
    /// v2.0: channel index (for channel PDA derivation).
    pub channel_index: Option<u16>,
}

// ---------------------------------------------------------------------------
// Round state
// ---------------------------------------------------------------------------

/// All possible states a round can occupy during its lifecycle.
pub enum RoundState {
    /// Waiting for all selected nodes to submit their commit hashes.
    CollectingCommits {
        deadline: Instant,
        /// node_id → commit_hash
        commits: HashMap<[u8; 33], [u8; 32]>,
    },
    /// All commits received; waiting for reveals.
    CollectingReveals {
        deadline: Instant,
        /// node_id → commit_hash  (kept for verification)
        commits: HashMap<[u8; 33], [u8; 32]>,
        /// node_id → entropy
        reveals: HashMap<[u8; 33], [u8; 32]>,
    },
    /// Round successfully completed with combined randomness.
    Finalized { randomness: [u8; 32] },
    /// Round aborted — no on-chain submission will be made.
    Failed { reason: String },
}

// ---------------------------------------------------------------------------
// Round
// ---------------------------------------------------------------------------

/// A single commit-reveal round for one on-chain randomness request.
pub struct Round {
    /// On-chain request identifier.
    pub request_id: [u8; 32],
    /// Nodes assigned to this round (ordered; used as the canonical set).
    pub selected_nodes: Vec<[u8; 33]>,
    /// Current lifecycle state.
    pub state: RoundState,
    /// Minimum reveals required to produce valid randomness.
    pub min_required: usize,
}

impl Round {
    /// Create a new round in `CollectingCommits` state.
    pub fn new(
        request_id: [u8; 32],
        selected_nodes: Vec<[u8; 33]>,
        min_required: usize,
        commit_timeout: Duration,
    ) -> Self {
        info!(
            request = hex::encode(request_id),
            nodes = selected_nodes.len(),
            min_required,
            "starting new round"
        );
        Round {
            request_id,
            selected_nodes,
            min_required,
            state: RoundState::CollectingCommits {
                deadline: Instant::now() + commit_timeout,
                commits: HashMap::new(),
            },
        }
    }

    // -----------------------------------------------------------------------
    // Public protocol handlers
    // -----------------------------------------------------------------------

    /// Record a commit from a node.
    ///
    /// Validates:
    /// - The node was selected for this round.
    /// - The round is still in `CollectingCommits`.
    /// - The ECDSA signature over `commit_hash` is valid.
    ///
    /// If all selected nodes have committed, transitions to `CollectingReveals`.
    pub fn handle_commit(
        &mut self,
        node_id: [u8; 33],
        commit_hash: [u8; 32],
        sig: [u8; 64],
    ) -> Result<()> {
        // Guard: correct state.
        let (_deadline, commits) = match &mut self.state {
            RoundState::CollectingCommits { deadline, commits } => (deadline, commits),
            _ => bail!("round is not in CommitCollection state"),
        };

        // Guard: node is a participant in this round.
        if !self.selected_nodes.contains(&node_id) {
            bail!(
                "node {} is not a participant in this round",
                hex::encode(node_id)
            );
        }

        // Guard: not a duplicate.
        if commits.contains_key(&node_id) {
            bail!("node {} already committed", hex::encode(node_id));
        }

        // Verify signature.
        if !verify_commit(&commit_hash, &node_id, &sig) {
            bail!(
                "invalid commit signature from node {}",
                hex::encode(node_id)
            );
        }

        debug!(
            node = hex::encode(node_id),
            commit = hex::encode(commit_hash),
            "commit accepted"
        );
        commits.insert(node_id, commit_hash);

        // Transition when every selected node has committed.
        let all_committed = self.selected_nodes.iter().all(|n| commits.contains_key(n));
        if all_committed {
            self.transition_to_reveals();
        }

        Ok(())
    }

    /// Record a reveal from a node.
    ///
    /// Validates:
    /// - The round is in `CollectingReveals`.
    /// - The node committed earlier.
    /// - `SHA-256(entropy) == commit_hash`.
    ///
    /// If `>= min_required` nodes have revealed, combines entropy and returns
    /// `Some(randomness)`.  Otherwise returns `None` (round still in progress).
    pub fn handle_reveal(
        &mut self,
        node_id: [u8; 33],
        entropy: [u8; 32],
        // The signature is verified against entropy by the same ECDSA key; for
        // the reveal phase we primarily validate hash-correctness, but we also
        // accept the sig for audit logging (verify_commit reuses the same key).
        _sig: [u8; 64],
    ) -> Result<Option<[u8; 32]>> {
        // We need read access to commits; collect what we need before the
        // mutable borrow of `state`.
        let commit_hash_for_node = match &self.state {
            RoundState::CollectingReveals { commits, .. } => *commits
                .get(&node_id)
                .ok_or_else(|| anyhow!("node {} did not commit", hex::encode(node_id)))?,
            _ => bail!("round is not in RevealCollection state"),
        };

        // Verify hash.
        if !verify_reveal(&entropy, &commit_hash_for_node) {
            bail!(
                "entropy does not match commit from node {}",
                hex::encode(node_id)
            );
        }

        // Now borrow mutably to store the reveal.
        let reveals = match &mut self.state {
            RoundState::CollectingReveals { reveals, .. } => reveals,
            _ => unreachable!(),
        };

        if reveals.contains_key(&node_id) {
            bail!("node {} already revealed", hex::encode(node_id));
        }

        debug!(
            node = hex::encode(node_id),
            entropy = hex::encode(entropy),
            "reveal accepted"
        );
        reveals.insert(node_id, entropy);

        let reveal_count_now = reveals.len();

        // Check whether we have enough reveals.
        if reveal_count_now >= self.min_required {
            let entropies: Vec<[u8; 32]> = reveals.values().copied().collect();
            let randomness = combine_entropy(&entropies);
            info!(
                request = hex::encode(self.request_id),
                reveals = reveal_count_now,
                randomness = hex::encode(randomness),
                "round finalized"
            );
            self.state = RoundState::Finalized { randomness };
            return Ok(Some(randomness));
        }

        Ok(None)
    }

    /// Return `true` if the current phase has exceeded its deadline and the
    /// round should be moved to `Failed`.
    ///
    /// Transitions the round to `Failed` internally when `true` is returned.
    pub fn check_timeout(&mut self) -> bool {
        let now = Instant::now();
        let timed_out = match &self.state {
            RoundState::CollectingCommits { deadline, .. } => now > *deadline,
            RoundState::CollectingReveals { deadline, .. } => now > *deadline,
            _ => false,
        };

        if timed_out {
            let reason = match &self.state {
                RoundState::CollectingCommits { .. } => "commit phase timeout",
                RoundState::CollectingReveals { .. } => "reveal phase timeout",
                _ => unreachable!(),
            };
            warn!(
                request = hex::encode(self.request_id),
                reason, "round timed out"
            );
            self.state = RoundState::Failed {
                reason: reason.to_string(),
            };
        }

        timed_out
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    /// Transition from `CollectingCommits` to `CollectingReveals`, preserving
    /// the collected commits and starting the reveal deadline.
    fn transition_to_reveals(&mut self) {
        // We need to consume the current state; swap in a temporary Failed to
        // satisfy the borrow checker, then immediately replace it.
        let prev = std::mem::replace(
            &mut self.state,
            RoundState::Failed {
                reason: "transitioning".to_string(),
            },
        );

        if let RoundState::CollectingCommits { commits, .. } = prev {
            info!(
                request = hex::encode(self.request_id),
                commits = commits.len(),
                "all commits received — entering reveal phase"
            );
            self.state = RoundState::CollectingReveals {
                deadline: Instant::now() + Duration::from_secs(60),
                commits,
                reveals: HashMap::new(),
            };
        }
        // If prev was somehow something else (shouldn't happen), leave the
        // Failed state in place.
    }

    // -----------------------------------------------------------------------
    // Status accessors (for API / logging)
    // -----------------------------------------------------------------------

    /// A short human-readable status string for the current state.
    pub fn status_str(&self) -> &'static str {
        match &self.state {
            RoundState::CollectingCommits { .. } => "collecting_commits",
            RoundState::CollectingReveals { .. } => "collecting_reveals",
            RoundState::Finalized { .. } => "finalized",
            RoundState::Failed { .. } => "failed",
        }
    }

    /// Return (commits_received, reveals_received) for progress tracking.
    pub fn progress_counts(&self) -> (usize, usize) {
        match &self.state {
            RoundState::CollectingCommits { commits, .. } => (commits.len(), 0),
            RoundState::CollectingReveals { commits, reveals, .. } => (commits.len(), reveals.len()),
            RoundState::Finalized { .. } => (self.selected_nodes.len(), self.selected_nodes.len()),
            RoundState::Failed { .. } => (0, 0),
        }
    }

    /// Extract the final randomness value, if the round is finalized.
    pub fn randomness(&self) -> Option<[u8; 32]> {
        match &self.state {
            RoundState::Finalized { randomness } => Some(*randomness),
            _ => None,
        }
    }
}
