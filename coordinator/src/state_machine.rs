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
        /// node_id → (commit_hash, ecdsa_signature)
        commits: HashMap<[u8; 33], ([u8; 32], [u8; 64])>,
    },
    /// All commits received; waiting for reveals.
    CollectingReveals {
        deadline: Instant,
        /// node_id → (commit_hash, ecdsa_signature)
        commits: HashMap<[u8; 33], ([u8; 32], [u8; 64])>,
        /// node_id → (entropy, ecdsa_signature)
        reveals: HashMap<[u8; 33], ([u8; 32], [u8; 64])>,
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
        commits.insert(node_id, (commit_hash, sig));

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
        sig: [u8; 64],
    ) -> Result<Option<[u8; 32]>> {
        // We need read access to commits; collect what we need before the
        // mutable borrow of `state`.
        let (commit_hash_for_node, _commit_sig) = match &self.state {
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
        reveals.insert(node_id, (entropy, sig));

        let reveal_count_now = reveals.len();

        // Check whether we have enough reveals.
        if reveal_count_now >= self.min_required {
            let entropies: Vec<[u8; 32]> = reveals.values().map(|(e, _)| *e).collect();
            let randomness = combine_entropy(&entropies);
            info!(
                request = hex::encode(self.request_id),
                reveals = reveal_count_now,
                randomness = hex::encode(randomness),
                "round finalized"
            );
            // Note: onchain_data is collected by the caller via collect_onchain_data()
            // BEFORE this state transition, if needed.
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

    /// Extract all commit+reveal data needed for bundled on-chain TX.
    /// Returns Vec of (node_id, commit_hash, commit_sig, entropy, reveal_sig).
    /// Only valid to call right BEFORE transitioning to Finalized.
    pub fn collect_onchain_data(&self) -> Vec<([u8; 33], [u8; 32], [u8; 64], [u8; 32], [u8; 64])> {
        match &self.state {
            RoundState::CollectingReveals { commits, reveals, .. } => {
                reveals.iter().filter_map(|(node_id, (entropy, reveal_sig))| {
                    commits.get(node_id).map(|(commit_hash, commit_sig)| {
                        (*node_id, *commit_hash, *commit_sig, *entropy, *reveal_sig)
                    })
                }).collect()
            }
            _ => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use k256::ecdsa::{signature::Signer, Signature, SigningKey};
    use sha2::{Digest, Sha256};
    use std::time::Duration;

    /// Helper: generate a new random ECDSA signing key and return
    /// (signing_key, compressed_pubkey_33_bytes).
    fn gen_node() -> (SigningKey, [u8; 33]) {
        let sk = SigningKey::random(&mut rand::thread_rng());
        let pk = sk.verifying_key().to_encoded_point(true);
        let mut pubkey = [0u8; 33];
        pubkey.copy_from_slice(pk.as_bytes());
        (sk, pubkey)
    }

    /// Helper: produce a valid commit (commit_hash, entropy, signature).
    /// commit_hash = SHA-256(entropy), signature = ECDSA(signing_key, commit_hash).
    fn make_commit(sk: &SigningKey) -> ([u8; 32], [u8; 32], [u8; 64]) {
        let mut entropy = [0u8; 32];
        rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut entropy);
        let commit_hash: [u8; 32] = Sha256::digest(entropy).into();
        let sig: Signature = sk.sign(&commit_hash);
        let mut sig_bytes = [0u8; 64];
        sig_bytes.copy_from_slice(&sig.to_bytes());
        (commit_hash, entropy, sig_bytes)
    }

    fn make_round(nodes: &[[u8; 33]], min_required: usize, timeout: Duration) -> Round {
        Round::new([0xAA; 32], nodes.to_vec(), min_required, timeout)
    }

    // -------------------------------------------------------------------
    // Basic construction
    // -------------------------------------------------------------------

    #[test]
    fn test_new_round_starts_in_collecting_commits() {
        let (_, pk) = gen_node();
        let round = make_round(&[pk], 1, Duration::from_secs(60));
        assert!(
            matches!(round.state, RoundState::CollectingCommits { .. }),
            "new round should start in CollectingCommits state"
        );
    }

    // -------------------------------------------------------------------
    // Progress counts
    // -------------------------------------------------------------------

    #[test]
    fn test_progress_counts_initial() {
        let (_, pk1) = gen_node();
        let (_, pk2) = gen_node();
        let round = make_round(&[pk1, pk2], 2, Duration::from_secs(60));
        let (commits, reveals) = round.progress_counts();
        assert_eq!(commits, 0);
        assert_eq!(reveals, 0);
    }

    #[test]
    fn test_progress_counts_after_commits() {
        let (sk1, pk1) = gen_node();
        let (sk2, pk2) = gen_node();
        let mut round = make_round(&[pk1, pk2], 2, Duration::from_secs(60));

        let (ch1, _, sig1) = make_commit(&sk1);
        round.handle_commit(pk1, ch1, sig1).unwrap();

        let (commits, reveals) = round.progress_counts();
        assert_eq!(commits, 1, "should have 1 commit after one node committed");
        assert_eq!(reveals, 0, "reveals should still be 0 in commit phase");

        // Commit second node — transitions to reveals
        let (ch2, _, sig2) = make_commit(&sk2);
        round.handle_commit(pk2, ch2, sig2).unwrap();

        let (commits2, reveals2) = round.progress_counts();
        assert_eq!(commits2, 2, "should have 2 commits in reveal phase");
        assert_eq!(reveals2, 0, "no reveals yet");
    }

    #[test]
    fn test_progress_counts_after_finalization() {
        let (sk1, pk1) = gen_node();
        let (sk2, pk2) = gen_node();
        let mut round = make_round(&[pk1, pk2], 2, Duration::from_secs(60));

        let (ch1, ent1, sig1) = make_commit(&sk1);
        let (ch2, ent2, sig2) = make_commit(&sk2);
        round.handle_commit(pk1, ch1, sig1).unwrap();
        round.handle_commit(pk2, ch2, sig2).unwrap();

        round.handle_reveal(pk1, ent1, [0u8; 64]).unwrap();
        round.handle_reveal(pk2, ent2, [0u8; 64]).unwrap();

        let (commits, reveals) = round.progress_counts();
        assert_eq!(commits, 2, "finalized round shows full commit count");
        assert_eq!(reveals, 2, "finalized round shows full reveal count");
    }

    // -------------------------------------------------------------------
    // Status string
    // -------------------------------------------------------------------

    #[test]
    fn test_status_str_all_states() {
        let (sk1, pk1) = gen_node();
        let (sk2, pk2) = gen_node();

        // CollectingCommits
        let mut round = make_round(&[pk1, pk2], 2, Duration::from_secs(60));
        assert_eq!(round.status_str(), "collecting_commits");

        // CollectingReveals
        let (ch1, _, sig1) = make_commit(&sk1);
        let (ch2, _, sig2) = make_commit(&sk2);
        round.handle_commit(pk1, ch1, sig1).unwrap();
        round.handle_commit(pk2, ch2, sig2).unwrap();
        assert_eq!(round.status_str(), "collecting_reveals");

        // Finalized — construct manually
        let finalized = Round {
            request_id: [0; 32],
            selected_nodes: vec![pk1],
            min_required: 1,
            state: RoundState::Finalized {
                randomness: [0xBB; 32],
            },
        };
        assert_eq!(finalized.status_str(), "finalized");

        // Failed
        let failed = Round {
            request_id: [0; 32],
            selected_nodes: vec![pk1],
            min_required: 1,
            state: RoundState::Failed {
                reason: "test".into(),
            },
        };
        assert_eq!(failed.status_str(), "failed");
    }

    // -------------------------------------------------------------------
    // handle_commit edge cases
    // -------------------------------------------------------------------

    #[test]
    fn test_handle_commit_rejects_non_participant() {
        let (_, pk1) = gen_node();
        let (sk_outsider, pk_outsider) = gen_node();
        let mut round = make_round(&[pk1], 1, Duration::from_secs(60));

        let (ch, _, sig) = make_commit(&sk_outsider);
        let err = round.handle_commit(pk_outsider, ch, sig);
        assert!(err.is_err(), "should reject non-participant");
        assert!(
            err.unwrap_err().to_string().contains("not a participant"),
            "error should mention 'not a participant'"
        );
    }

    #[test]
    fn test_handle_commit_rejects_duplicate() {
        let (sk1, pk1) = gen_node();
        let (_, pk2) = gen_node();
        let mut round = make_round(&[pk1, pk2], 2, Duration::from_secs(60));

        let (ch1, _, sig1) = make_commit(&sk1);
        round.handle_commit(pk1, ch1, sig1).unwrap();

        // Try to commit again with the same node
        let (ch1b, _, sig1b) = make_commit(&sk1);
        let err = round.handle_commit(pk1, ch1b, sig1b);
        assert!(err.is_err(), "should reject duplicate commit");
        assert!(
            err.unwrap_err().to_string().contains("already committed"),
            "error should mention 'already committed'"
        );
    }

    #[test]
    fn test_handle_commit_transitions_to_reveals_when_all_committed() {
        let (sk1, pk1) = gen_node();
        let (sk2, pk2) = gen_node();
        let (sk3, pk3) = gen_node();
        let mut round = make_round(&[pk1, pk2, pk3], 2, Duration::from_secs(60));

        let (ch1, _, sig1) = make_commit(&sk1);
        let (ch2, _, sig2) = make_commit(&sk2);
        let (ch3, _, sig3) = make_commit(&sk3);

        round.handle_commit(pk1, ch1, sig1).unwrap();
        assert!(
            matches!(round.state, RoundState::CollectingCommits { .. }),
            "should stay in CollectingCommits after 1/3"
        );

        round.handle_commit(pk2, ch2, sig2).unwrap();
        assert!(
            matches!(round.state, RoundState::CollectingCommits { .. }),
            "should stay in CollectingCommits after 2/3"
        );

        round.handle_commit(pk3, ch3, sig3).unwrap();
        assert!(
            matches!(round.state, RoundState::CollectingReveals { .. }),
            "should transition to CollectingReveals after 3/3"
        );
    }

    // -------------------------------------------------------------------
    // handle_reveal edge cases
    // -------------------------------------------------------------------

    #[test]
    fn test_handle_reveal_rejects_wrong_state() {
        let (_, pk1) = gen_node();
        let mut round = make_round(&[pk1], 1, Duration::from_secs(60));

        // Round is in CollectingCommits — reveal should fail
        let entropy = [0x42u8; 32];
        let err = round.handle_reveal(pk1, entropy, [0u8; 64]);
        assert!(err.is_err(), "should reject reveal in CollectingCommits state");
        assert!(
            err.unwrap_err()
                .to_string()
                .contains("not in RevealCollection state"),
            "error should mention wrong state"
        );
    }

    // -------------------------------------------------------------------
    // Randomness accessor
    // -------------------------------------------------------------------

    #[test]
    fn test_randomness_none_before_finalization() {
        let (_, pk1) = gen_node();
        let round = make_round(&[pk1], 1, Duration::from_secs(60));
        assert!(
            round.randomness().is_none(),
            "randomness should be None before finalization"
        );
    }

    #[test]
    fn test_randomness_some_after_finalization() {
        let (sk1, pk1) = gen_node();
        let mut round = make_round(&[pk1], 1, Duration::from_secs(60));

        let (ch1, ent1, sig1) = make_commit(&sk1);
        round.handle_commit(pk1, ch1, sig1).unwrap();

        let result = round.handle_reveal(pk1, ent1, [0u8; 64]).unwrap();
        assert!(result.is_some(), "reveal should finalize with min_required=1");

        let randomness = round.randomness();
        assert!(randomness.is_some(), "randomness should be Some after finalization");
        assert_eq!(
            randomness.unwrap(),
            result.unwrap(),
            "randomness accessor should return same value as handle_reveal"
        );
    }

    // -------------------------------------------------------------------
    // Timeout
    // -------------------------------------------------------------------

    #[test]
    fn test_check_timeout_triggers_after_deadline() {
        let (_, pk1) = gen_node();
        // Create round with zero timeout — already expired
        let mut round = make_round(&[pk1], 1, Duration::from_millis(0));

        // Spin briefly to ensure Instant::now() is past the deadline
        std::thread::sleep(Duration::from_millis(5));

        let timed_out = round.check_timeout();
        assert!(timed_out, "should detect timeout after deadline has passed");
        assert!(
            matches!(round.state, RoundState::Failed { .. }),
            "state should be Failed after timeout"
        );
        assert_eq!(round.status_str(), "failed");

        // A non-timed-out round should return false
        let mut fresh = make_round(&[pk1], 1, Duration::from_secs(3600));
        assert!(
            !fresh.check_timeout(),
            "should not timeout with a long deadline"
        );
    }
}
