//! Solana WebSocket log subscriber for DiceChannel events.
//!
//! Replaces the 5-second polling watcher with real-time `logsSubscribe`.
//! Subscribes to all log messages from the DICE program and detects
//! `request_randomness_v2` calls by matching log patterns.
//! Latency: ~500ms (vs 0-5s with polling).

use std::{collections::HashSet, sync::Arc, time::Duration};

use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use solana_sdk::{pubkey::Pubkey, signature::Keypair};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info, warn};

use crate::{
    metrics::Metrics,
    node_session::NodeRegistry,
    protocol::messages::{DiceMessage, JobAssignment},
    selection::SelectionEngine,
    solana_rpc::SolanaRpc,
    state_machine::{Round, RoundEntry, RoundMap},
};

/// Anchor account discriminator for `DiceChannel`.
/// SHA-256("account:DiceChannel")[0..8]
const DICE_CHANNEL_DISC: [u8; 8] = [13, 92, 61, 143, 179, 94, 32, 52];

/// DiceChannel status field offset:
///   8 (disc) + 32 (authority) + 2 (channel_index) + 1 (max_nodes) = 43
const CHANNEL_STATUS_OFFSET: usize = 43;

/// ChannelStatus::Pending = enum variant index 1
const STATUS_PENDING: u8 = 1;

/// DiceChannel round_id offset: status_offset + 1 (status) = 44
const ROUND_ID_OFFSET: usize = 44;

/// DiceChannel node_count offset: round_id_offset + 8 (round_id) = 52
const NODE_COUNT_OFFSET: usize = 52;

/// DiceChannel authority offset: 8 (disc) = 8
const AUTHORITY_OFFSET: usize = 8;

/// DiceChannel channel_index offset: 8 + 32 = 40
const CHANNEL_INDEX_OFFSET: usize = 40;

/// Log message emitted by request_randomness_v2 that we match on.
const REQUEST_LOG_PREFIX: &str = "Program log: Randomness requested on channel";

/// Reconnect delay after WebSocket disconnection.
const RECONNECT_DELAY: Duration = Duration::from_secs(3);

/// Convert an HTTP(S) RPC URL to a WebSocket URL.
///
/// `https://api.devnet.solana.com` → `wss://api.devnet.solana.com`
/// `http://localhost:8899` → `ws://localhost:8899`
fn http_to_ws_url(http_url: &str) -> String {
    if let Some(rest) = http_url.strip_prefix("https://") {
        format!("wss://{}", rest)
    } else if let Some(rest) = http_url.strip_prefix("http://") {
        format!("ws://{}", rest)
    } else {
        // Already a ws:// or wss:// URL
        http_url.to_string()
    }
}

/// Run the Solana WebSocket log subscriber loop.
///
/// Subscribes to `logsSubscribe` for the DICE program. When a
/// `request_randomness_v2` log is detected, fetches the channel account
/// and dispatches a round to connected nodes.
///
/// Automatically reconnects on disconnection.
pub async fn run_solana_ws_subscriber(
    rpc: Arc<SolanaRpc>,
    rpc_url: String,
    program_id: Pubkey,
    coordinator_keypair: Arc<Keypair>,
    registry: NodeRegistry,
    rounds: RoundMap,
    metrics: Metrics,
    min_nodes: usize,
    max_nodes: usize,
    commit_timeout: Duration,
) {
    let ws_url = http_to_ws_url(&rpc_url);
    info!(
        program = %program_id,
        ws_url = %ws_url,
        "Solana WebSocket subscriber starting — listening for DiceChannel events"
    );

    // Track channels we've already dispatched for this round to avoid duplicates.
    let mut dispatched: HashSet<(Pubkey, u64)> = HashSet::new();

    loop {
        match run_subscription(
            &ws_url,
            &rpc,
            &program_id,
            &coordinator_keypair,
            &registry,
            &rounds,
            &metrics,
            &mut dispatched,
            min_nodes,
            max_nodes,
            commit_timeout,
        )
        .await
        {
            Ok(()) => {
                info!("WebSocket subscription ended normally, reconnecting...");
            }
            Err(e) => {
                warn!(error = %e, "WebSocket subscription error, reconnecting...");
            }
        }

        // Prune dispatched set to prevent unbounded growth.
        if dispatched.len() > 1000 {
            dispatched.clear();
        }

        tokio::time::sleep(RECONNECT_DELAY).await;
    }
}

/// Single subscription session. Returns when the WebSocket disconnects.
async fn run_subscription(
    ws_url: &str,
    rpc: &SolanaRpc,
    program_id: &Pubkey,
    coordinator_keypair: &Keypair,
    registry: &NodeRegistry,
    rounds: &RoundMap,
    metrics: &Metrics,
    dispatched: &mut HashSet<(Pubkey, u64)>,
    min_nodes: usize,
    max_nodes: usize,
    commit_timeout: Duration,
) -> Result<()> {
    let (ws_stream, _) = connect_async(ws_url)
        .await
        .context("WebSocket connect failed")?;

    info!("WebSocket connected to {}", ws_url);

    let (mut write, mut read) = ws_stream.split();

    // Subscribe to program logs.
    let subscribe_msg = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "logsSubscribe",
        "params": [
            { "mentions": [program_id.to_string()] },
            { "commitment": "confirmed" }
        ]
    });

    write
        .send(Message::Text(subscribe_msg.to_string().into()))
        .await
        .context("send logsSubscribe")?;

    info!(program = %program_id, "logsSubscribe sent, waiting for events...");

    while let Some(msg_result) = read.next().await {
        let msg = match msg_result {
            Ok(Message::Text(t)) => t,
            Ok(Message::Ping(p)) => {
                let _ = write.send(Message::Pong(p)).await;
                continue;
            }
            Ok(Message::Close(_)) => {
                info!("WebSocket closed by server");
                break;
            }
            Ok(_) => continue,
            Err(e) => {
                warn!(error = %e, "WebSocket read error");
                break;
            }
        };

        // Parse the JSON notification.
        let json: Value = match serde_json::from_str(&msg) {
            Ok(v) => v,
            Err(_) => continue,
        };

        // Skip subscription confirmation responses.
        if json.get("result").is_some() && json.get("method").is_none() {
            debug!("subscription confirmed: {}", json);
            continue;
        }

        // Process log notifications.
        let params = match json.get("params").and_then(|p| p.get("result")) {
            Some(r) => r,
            None => continue,
        };

        let logs = match params.get("value").and_then(|v| v.get("logs")).and_then(|l| l.as_array())
        {
            Some(l) => l,
            None => continue,
        };

        // Check if any log line matches our pattern.
        let has_request = logs.iter().any(|l| {
            l.as_str()
                .map(|s| s.contains(REQUEST_LOG_PREFIX))
                .unwrap_or(false)
        });

        if !has_request {
            continue;
        }

        info!("request_randomness_v2 detected via log subscription");

        // The log tells us a request was made, but we need the channel details.
        // Scan for Pending DiceChannel accounts.
        match find_pending_channels(rpc, program_id).await {
            Ok(channels) => {
                for (channel_pubkey, authority, channel_index, round_id, node_count) in channels {
                    let key = (channel_pubkey, round_id);
                    if dispatched.contains(&key) {
                        continue;
                    }

                    info!(
                        channel = %channel_pubkey,
                        round_id,
                        node_count,
                        "dispatching round for pending channel"
                    );

                    match dispatch_channel_round(
                        &channel_pubkey,
                        &authority,
                        channel_index,
                        round_id,
                        node_count as usize,
                        registry,
                        rounds,
                        metrics,
                        min_nodes.max(node_count as usize),
                        commit_timeout,
                    )
                    .await
                    {
                        Ok(()) => {
                            dispatched.insert(key);
                            info!(channel = %channel_pubkey, round_id, "round dispatched");
                        }
                        Err(e) => {
                            warn!(channel = %channel_pubkey, error = %e, "failed to dispatch round");
                        }
                    }
                }
            }
            Err(e) => {
                warn!(error = %e, "failed to scan for pending channels");
            }
        }
    }

    Ok(())
}

/// Query Solana for DiceChannel accounts in Pending status.
/// Returns Vec of (channel_pubkey, authority, channel_index, round_id, node_count).
async fn find_pending_channels(
    rpc: &SolanaRpc,
    program_id: &Pubkey,
) -> Result<Vec<(Pubkey, Pubkey, u16, u64, u8)>> {
    let accounts = rpc
        .get_program_accounts(
            program_id,
            None,
            &[
                (0, &DICE_CHANNEL_DISC),
                (CHANNEL_STATUS_OFFSET, &[STATUS_PENDING]),
            ],
        )
        .await
        .context("getProgramAccounts for Pending channels")?;

    let mut results = Vec::new();
    for (pubkey, data) in accounts {
        if data.len() < 53 {
            continue;
        }

        let authority = Pubkey::try_from(&data[AUTHORITY_OFFSET..AUTHORITY_OFFSET + 32])
            .unwrap_or_default();
        let channel_index =
            u16::from_le_bytes(data[CHANNEL_INDEX_OFFSET..CHANNEL_INDEX_OFFSET + 2].try_into().unwrap_or([0; 2]));
        let round_id =
            u64::from_le_bytes(data[ROUND_ID_OFFSET..ROUND_ID_OFFSET + 8].try_into().unwrap_or([0; 8]));
        let node_count = data[NODE_COUNT_OFFSET];

        results.push((pubkey, authority, channel_index, round_id, node_count));
    }

    Ok(results)
}

/// Select nodes and dispatch a round for a pending DiceChannel.
async fn dispatch_channel_round(
    channel_pubkey: &Pubkey,
    authority: &Pubkey,
    channel_index: u16,
    round_id: u64,
    node_count: usize,
    registry: &NodeRegistry,
    rounds: &RoundMap,
    metrics: &Metrics,
    min_nodes: usize,
    commit_timeout: Duration,
) -> Result<()> {
    let recently_selected = HashSet::new();
    let selected = SelectionEngine::select_nodes(
        registry,
        &recently_selected,
        node_count,
        min_nodes,
    )
    .await
    .ok_or_else(|| anyhow::anyhow!(
        "not enough active nodes (need {}, have fewer)",
        min_nodes
    ))?;

    // Use channel pubkey as the in-memory request_id.
    let mut request_id = [0u8; 32];
    request_id.copy_from_slice(channel_pubkey.as_ref());

    let deadline_ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        + commit_timeout.as_secs();

    let round = Round::new(request_id, selected.clone(), min_nodes, commit_timeout);
    let db_id = uuid::Uuid::new_v4();

    {
        let mut map = rounds.lock().await;
        map.insert(
            request_id,
            RoundEntry {
                round,
                db_id,
                started_at: std::time::Instant::now(),
                requester: *authority,
                sequence: round_id,
                channel_authority: Some(*authority),
                channel_index: Some(channel_index),
            },
        );
    }

    metrics.rounds_total.inc();

    // Build and send JobAssignment to each selected node.
    let job = DiceMessage::JobAssignment(JobAssignment {
        request_id: request_id.to_vec(),
        round_seq: round_id,
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

    Ok(())
}
