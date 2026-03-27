use std::collections::HashSet;

use rand::seq::SliceRandom;
use tracing::debug;

use crate::node_session::NodeRegistry;

/// Candidate pool size — we pre-filter by latency before random selection.
const LATENCY_CANDIDATE_POOL: usize = 10;

/// Engine responsible for selecting a fair, low-latency subset of nodes for
/// each randomness round.
pub struct SelectionEngine;

impl SelectionEngine {
    /// Select up to `target` nodes from the active registry for a new round.
    ///
    /// # Algorithm
    /// 1. Read all sessions whose last heartbeat arrived within 45 s.
    /// 2. Exclude any node present in `recently_selected` (rotation fairness).
    /// 3. If fewer than `min` eligible nodes remain, return `None`.
    /// 4. Sort the eligible set by `latency_ms` ascending.
    /// 5. Retain the top [`LATENCY_CANDIDATE_POOL`] candidates.
    /// 6. Randomly shuffle that pool and take the first `target` entries.
    /// 7. Return `Some(selected)`.
    pub async fn select_nodes(
        registry: &NodeRegistry,
        recently_selected: &HashSet<[u8; 33]>,
        target: usize,
        min: usize,
    ) -> Option<Vec<[u8; 33]>> {
        // Snapshot active sessions (needs read lock briefly).
        let active: Vec<(u32, [u8; 33])> = {
            let map = registry.read().await;
            map.values()
                .filter(|s| {
                    s.last_heartbeat.elapsed().as_secs() <= 45
                        && !recently_selected.contains(&s.node_id)
                })
                .map(|s| (s.latency_ms, s.node_id))
                .collect()
        };

        debug!(
            eligible = active.len(),
            recently_excluded = recently_selected.len(),
            "selection: eligible nodes"
        );

        if active.len() < min {
            return None;
        }

        // Sort by latency ascending and keep the best LATENCY_CANDIDATE_POOL.
        let mut sorted = active;
        sorted.sort_by_key(|(lat, _)| *lat);
        sorted.truncate(LATENCY_CANDIDATE_POOL);

        // Shuffle so we don't always pick the same low-latency nodes.
        let mut rng = rand::thread_rng();
        sorted.shuffle(&mut rng);

        // Take up to `target`.
        let selected: Vec<[u8; 33]> = sorted.into_iter().take(target).map(|(_, id)| id).collect();

        Some(selected)
    }
}
