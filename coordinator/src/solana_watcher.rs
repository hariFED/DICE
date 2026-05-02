//! Solana on-chain watcher.
//!
//! Polls for new `RandomnessRequest` accounts in `Pending` status and
//! automatically starts commit-reveal rounds for them.

use std::{
    collections::HashSet,
    str::FromStr,
    sync::Arc,
    time::Duration,
};

use anyhow::{Context, Result};
use solana_sdk::{pubkey::Pubkey, signature::Keypair};
use tracing::{debug, error, info, warn};

use crate::{
    metrics::Metrics,
    node_session::NodeRegistry,
    protocol::messages::{DiceMessage, JobAssignment},
    selection::SelectionEngine,
    solana_rpc::SolanaRpc,
    state_machine::{Round, RoundEntry, RoundMap},
};

/// Anchor account discriminator for `RandomnessRequest`.
/// SHA-256("account:RandomnessRequest")[0..8]
const RANDOMNESS_REQUEST_DISC: [u8; 8] = {
    // SHA-256("account:RandomnessRequest")[0..8]
    // Used as a `memcmp` filter at offset 0 when querying getProgramAccounts.
    [244, 231, 228, 160, 148, 28, 17, 184]
};

/// Byte value of `RequestStatus::Pending` in borsh encoding.
/// The status field is at offset 8 (disc) + 32 (requester) + 8 (sequence) = 48.
const STATUS_OFFSET: usize = 48;
const STATUS_PENDING: u8 = 0;

/// Account data size for `RandomnessRequest` (339 bytes per the LEN constant).
const REQUEST_ACCOUNT_SIZE: usize = 339;

/// Poll interval for checking new on-chain requests.
const POLL_INTERVAL: Duration = Duration::from_secs(5);

/// Run the Solana watcher loop. Polls for `Pending` randomness requests
/// and dispatches rounds to connected nodes.
pub async fn run_solana_watcher(
    rpc: Arc<SolanaRpc>,
    program_id: Pubkey,
    coordinator_keypair: Arc<Keypair>,
    registry: NodeRegistry,
    rounds: RoundMap,
    metrics: Metrics,
    min_nodes: usize,
    max_nodes: usize,
    commit_timeout: Duration,
) {
    info!(
        program = %program_id,
        "Solana watcher starting — polling for Pending RandomnessRequests"
    );

    // Track request pubkeys we've already dispatched to avoid duplicates.
    let mut dispatched: HashSet<Pubkey> = HashSet::new();
    let mut recently_selected: HashSet<[u8; 33]> = HashSet::new();
    let mut interval = tokio::time::interval(POLL_INTERVAL);

    loop {
        interval.tick().await;

        match poll_pending_requests(&rpc, &program_id).await {
            Ok(pending) => {
                for (request_pubkey, request_data) in pending {
                    if dispatched.contains(&request_pubkey) {
                        continue;
                    }

                    info!(
                        request = %request_pubkey,
                        "new Pending RandomnessRequest detected"
                    );

                    match dispatch_round(
                        &request_pubkey,
                        &request_data,
                        &registry,
                        &rounds,
                        &metrics,
                        &recently_selected,
                        max_nodes,
                        min_nodes,
                        commit_timeout,
                    )
                    .await
                    {
                        Ok(selected) => {
                            dispatched.insert(request_pubkey);
                            for nid in &selected {
                                recently_selected.insert(*nid);
                            }
                            info!(
                                request = %request_pubkey,
                                nodes = selected.len(),
                                "round dispatched"
                            );
                        }
                        Err(e) => {
                            warn!(
                                request = %request_pubkey,
                                error = %e,
                                "failed to dispatch round"
                            );
                        }
                    }
                }
            }
            Err(e) => {
                debug!(error = %e, "poll for pending requests failed (will retry)");
            }
        }

        // Prune dispatched set periodically (keep last 1000).
        if dispatched.len() > 1000 {
            dispatched.clear();
        }
        // Reset recently_selected to allow node re-use over time.
        if recently_selected.len() > 50 {
            recently_selected.clear();
        }
    }
}

/// Query Solana for `RandomnessRequest` accounts in `Pending` status.
async fn poll_pending_requests(
    rpc: &SolanaRpc,
    program_id: &Pubkey,
) -> Result<Vec<(Pubkey, Vec<u8>)>> {
    // Filter: data size matches, discriminator matches, status == Pending.
    let accounts = rpc
        .get_program_accounts(
            program_id,
            Some(REQUEST_ACCOUNT_SIZE),
            &[
                (0, &RANDOMNESS_REQUEST_DISC),
                (STATUS_OFFSET, &[STATUS_PENDING]),
            ],
        )
        .await
        .context("getProgramAccounts for Pending requests")?;

    Ok(accounts)
}

/// Select nodes and dispatch a round for a pending on-chain request.
async fn dispatch_round(
    request_pubkey: &Pubkey,
    request_data: &[u8],
    registry: &NodeRegistry,
    rounds: &RoundMap,
    metrics: &Metrics,
    recently_selected: &HashSet<[u8; 33]>,
    max_nodes: usize,
    min_nodes: usize,
    commit_timeout: Duration,
) -> Result<Vec<[u8; 33]>> {
    // Parse the request_id from the account data.
    // Layout: [0..8] disc, [8..40] requester, [40..48] sequence
    // We'll use a hash of the account pubkey as the internal request_id.
    let mut request_id = [0u8; 32];
    request_id.copy_from_slice(request_pubkey.as_ref());

    // Select nodes.
    let selected = SelectionEngine::select_nodes(
        registry,
        recently_selected,
        max_nodes,
        min_nodes,
    )
    .await
    .ok_or_else(|| anyhow::anyhow!(
        "not enough active nodes (need {}, have fewer)",
        min_nodes
    ))?;

    // Parse sequence from account data for the JobAssignment.
    let sequence = if request_data.len() >= 48 {
        u64::from_le_bytes(request_data[40..48].try_into().unwrap_or([0; 8]))
    } else {
        0
    };

    let deadline_ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        + commit_timeout.as_secs();

    // Create round in memory.
    let round = Round::new(
        request_id,
        selected.clone(),
        min_nodes,
        commit_timeout,
    );
    let db_id = uuid::Uuid::new_v4();

    {
        let mut map = rounds.lock().await;
        map.insert(request_id, RoundEntry {
            round,
            db_id,
            started_at: std::time::Instant::now(),
            requester: solana_sdk::pubkey::Pubkey::default(),
            sequence,
            channel_authority: None,
            channel_index: None,
        });
    }

    metrics.rounds_total.inc();

    // Build and send JobAssignment to each selected node.
    let job = DiceMessage::JobAssignment(JobAssignment {
        request_id: request_id.to_vec(),
        round_seq: sequence,
        deadline_ts,
    });

    let encoded = job.encode().context("encode JobAssignment")?;

    {
        let reg = registry.read().await;
        for node_id in &selected {
            if let Some(session) = reg.get(node_id) {
                let _ = session.tx.try_send(encoded.clone());
            }
        }
    }

    Ok(selected)
}
