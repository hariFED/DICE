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

/// DiceChannel layout offsets (must match programs/dice/src/state/dice_channel.rs):
///   8 (disc) + 32 (authority) + 32 (coordinator) + 2 (channel_index) + 1 (max_nodes)
///   + 1 (status) + 8 (round_id) + 1 (node_count) + ...
const AUTHORITY_OFFSET: usize = 8;
const COORDINATOR_OFFSET: usize = 40; // 8 + 32
const CHANNEL_INDEX_OFFSET: usize = 72; // 8 + 32 + 32
// max_nodes at 74
const CHANNEL_STATUS_OFFSET: usize = 75; // 8 + 32 + 32 + 2 + 1
const ROUND_ID_OFFSET: usize = 76; // status + 1
const NODE_COUNT_OFFSET: usize = 84; // round_id + 8

/// ChannelStatus::Pending = enum variant index 1
const STATUS_PENDING: u8 = 1;

/// Log message emitted by request_randomness_v2 that we match on.
const REQUEST_LOG_PREFIX: &str = "Program log: Randomness requested on channel";

/// Reconnect delay after WebSocket disconnection.
const RECONNECT_DELAY: Duration = Duration::from_secs(3);

/// Polling interval for the DiceChannel poller.
/// 3 s was the stable baseline that survived the 985/1000 A4 run. 800 ms
/// and 1.5 s both hung under public-devnet RPC backpressure (L2-A). Helius
/// RPC is well-provisioned enough that we can pull this back to 1 s without
/// the hang — each round now sees up to 1 s of "waiting for coord to notice"
/// latency, down from 3 s.
const DICE_CHANNEL_POLL_INTERVAL: Duration = Duration::from_secs(1);

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

/// HTTP-polling fallback for the WebSocket subscriber.
///
/// `tokio-tungstenite` 0.21 is built in this workspace WITHOUT a TLS
/// feature (see workspace `Cargo.toml` — adding one would force rustls 0.22
/// which conflicts with our pinned 0.21). That makes outgoing wss:// connects
/// fail, so the WebSocket subscriber loop just spams reconnect errors against
/// `wss://api.devnet.solana.com`. This poller uses the existing reqwest-based
/// JSON-RPC client (already wired for HTTPS) to scan for Pending DiceChannels
/// every few seconds and dispatch rounds — same behavior as the WS path,
/// just slightly higher latency.
pub async fn run_dice_channel_poller(
    rpc: Arc<SolanaRpc>,
    program_id: Pubkey,
    _coordinator_keypair: Arc<Keypair>,
    registry: NodeRegistry,
    rounds: RoundMap,
    metrics: Metrics,
    min_nodes: usize,
    _max_nodes: usize,
    commit_timeout: Duration,
) {
    info!(
        program = %program_id,
        "DiceChannel poller starting — scanning every 3 s for Pending channels"
    );

    let mut dispatched: HashSet<(Pubkey, u64)> = HashSet::new();
    let mut interval = tokio::time::interval(DICE_CHANNEL_POLL_INTERVAL);

    loop {
        interval.tick().await;

        match find_pending_channels(&rpc, &program_id).await {
            Ok(channels) => {
                for (channel_pubkey, authority, channel_index, round_id, node_count, preselected) in channels {
                    let key = (channel_pubkey, round_id);
                    if dispatched.contains(&key) {
                        continue;
                    }

                    info!(
                        channel = %channel_pubkey,
                        round_id,
                        node_count,
                        on_chain_selected = preselected.is_some(),
                        "dispatching round for pending channel"
                    );

                    match dispatch_channel_round(
                        &channel_pubkey,
                        &authority,
                        channel_index,
                        round_id,
                        node_count as usize,
                        &registry,
                        &rounds,
                        &metrics,
                        min_nodes.max(node_count as usize),
                        commit_timeout,
                        preselected.as_deref(),
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
                debug!(error = %e, "poll for pending channels failed (will retry)");
            }
        }

        if dispatched.len() > 1000 {
            dispatched.clear();
        }
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

        // Short delay to allow RPC state propagation before querying accounts.
        // Solana RPC nodes may have a slight lag between committing a log and
        // the account state becoming visible at "confirmed" commitment.
        tokio::time::sleep(Duration::from_millis(500)).await;

        // The log tells us a request was made, but we need the channel details.
        // Scan for Pending DiceChannel accounts. Retry once on empty result.
        match find_pending_channels(rpc, program_id).await {
            Ok(channels) => {
                for (channel_pubkey, authority, channel_index, round_id, node_count, preselected) in channels {
                    let key = (channel_pubkey, round_id);
                    if dispatched.contains(&key) {
                        continue;
                    }

                    info!(
                        channel = %channel_pubkey,
                        round_id,
                        node_count,
                        on_chain_selected = preselected.is_some(),
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
                        preselected.as_deref(),
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
) -> Result<Vec<(Pubkey, Pubkey, u16, u64, u8, Option<Vec<[u8; 33]>>)>> {
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
        if data.len() < 85 { // need at least through node_count offset
            continue;
        }

        let authority = Pubkey::try_from(&data[AUTHORITY_OFFSET..AUTHORITY_OFFSET + 32])
            .unwrap_or_default();
        let channel_index =
            u16::from_le_bytes(data[CHANNEL_INDEX_OFFSET..CHANNEL_INDEX_OFFSET + 2].try_into().unwrap_or([0; 2]));
        let round_id =
            u64::from_le_bytes(data[ROUND_ID_OFFSET..ROUND_ID_OFFSET + 8].try_into().unwrap_or([0; 8]));
        let node_count = data[NODE_COUNT_OFFSET];
        let max_nodes = data[74]; // offset 74 = max_nodes (see state layout)

        // Extract channel.device_pubkeys[0..node_count] if the program ran
        // v7.3 on-chain selection and pre-populated them during
        // `request_randomness_auto`. Layout after the fixed 183-byte header:
        //   device_ids      : 4 (Vec len) + max_nodes * 32
        //   device_pubkeys  : 4 (Vec len) + max_nodes * 33   ← we want this
        // If the Vec is all-zero-keys we treat it as "no on-chain selection"
        // and fall back to the coord's off-chain SelectionEngine below.
        let preselected = if max_nodes > 0 && node_count > 0 {
            let pubkeys_start =
                183 + 4 + (max_nodes as usize) * 32 + 4; // skip device_ids, into device_pubkeys
            let want = node_count as usize;
            if data.len() >= pubkeys_start + want * 33 {
                let mut picks = Vec::with_capacity(want);
                let mut any_nonzero = false;
                for i in 0..want {
                    let off = pubkeys_start + i * 33;
                    let mut key = [0u8; 33];
                    key.copy_from_slice(&data[off..off + 33]);
                    if key != [0u8; 33] {
                        any_nonzero = true;
                    }
                    picks.push(key);
                }
                if any_nonzero { Some(picks) } else { None }
            } else {
                None
            }
        } else {
            None
        };

        results.push((pubkey, authority, channel_index, round_id, node_count, preselected));
    }

    Ok(results)
}

/// Select nodes and dispatch a round for a pending DiceChannel.
///
/// If `preselected` is `Some`, those exact device pubkeys are used (this is
/// the v7.3 on-chain-selection path — the program already wrote picks into
/// `channel.device_pubkeys` and we just honor them). Otherwise the
/// coordinator's off-chain `SelectionEngine` picks based on measured latency.
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
    preselected: Option<&[[u8; 33]]>,
) -> Result<()> {
    let selected: Vec<[u8; 33]> = if let Some(picks) = preselected {
        // Honor the on-chain selection. We still need the nodes to be live
        // (connected via mTLS) to dispatch to them — filter against the
        // registry. If any selected node is offline, we bail: with on-chain
        // selection the coord isn't allowed to swap in a different device.
        let reg = registry.read().await;
        let mut verified = Vec::with_capacity(picks.len());
        for pk in picks.iter().take(node_count) {
            if reg.contains_key(pk) {
                verified.push(*pk);
            } else {
                return Err(anyhow::anyhow!(
                    "on-chain-selected node {} not connected",
                    hex::encode(&pk[..6])
                ));
            }
        }
        drop(reg);
        verified
    } else {
        let recently_selected = HashSet::new();
        SelectionEngine::select_nodes(
            registry,
            &recently_selected,
            node_count,
            min_nodes,
        )
        .await
        .ok_or_else(|| anyhow::anyhow!(
            "not enough active nodes (need {}, have fewer)",
            min_nodes
        ))?
    };

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
