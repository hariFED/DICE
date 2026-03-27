use std::time::Duration;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde_json::json;
use uuid::Uuid;

use crate::{
    db::queries::get_round,
    metrics::Metrics,
    node_session::{get_active_node_infos, get_active_nodes, NodeRegistry},
    protocol::messages::{DiceMessage, JobAssignment},
    solana_tx::OnChainCtx,
    state_machine::{Round, RoundEntry, RoundMap},
};

// ---------------------------------------------------------------------------
// Shared application state
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct AppState {
    pub registry: NodeRegistry,
    pub metrics: Metrics,
    pub db: Option<sqlx::PgPool>,
    pub rounds: RoundMap,
    /// If set, transactions are submitted to Solana devnet/mainnet.
    pub on_chain: Option<OnChainCtx>,
}

// ---------------------------------------------------------------------------
// Router builder
// ---------------------------------------------------------------------------

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/", get(dashboard))
        .route("/health", get(health))
        .route("/nodes", get(list_nodes))
        .route("/rounds", get(list_rounds))
        .route("/rounds/:id", get(get_round_handler))
        .route("/simulate", post(simulate))
        .route("/metrics", get(metrics_handler))
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Dashboard
// ---------------------------------------------------------------------------

const DASHBOARD_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <title>DICE Coordinator Dashboard</title>
  <style>
    * { box-sizing: border-box; margin: 0; padding: 0; }
    body { font-family: 'Courier New', monospace; background: #0d0d0d; color: #e0e0e0; padding: 2rem; }
    h1 { color: #00ff88; font-size: 1.6rem; margin-bottom: 0.25rem; }
    .subtitle { color: #555; font-size: 0.85rem; margin-bottom: 2rem; }
    .grid { display: grid; grid-template-columns: 1fr 1fr; gap: 1.5rem; margin-bottom: 1.5rem; }
    .card { border: 1px solid #1e1e1e; background: #111; padding: 1.25rem; border-radius: 6px; }
    .card h2 { color: #aaa; font-size: 0.8rem; text-transform: uppercase; letter-spacing: 0.1em; margin-bottom: 1rem; }
    .card.full { grid-column: 1 / -1; }
    .stat { font-size: 2.5rem; color: #00ff88; font-weight: bold; }
    .stat-label { color: #555; font-size: 0.75rem; margin-top: 0.25rem; }
    table { width: 100%; border-collapse: collapse; font-size: 0.8rem; }
    th { color: #555; text-align: left; padding: 0.4rem 0.5rem; border-bottom: 1px solid #1e1e1e; font-weight: normal; }
    td { padding: 0.4rem 0.5rem; border-bottom: 1px solid #1a1a1a; }
    .ok   { color: #00ff88; }
    .warn { color: #ffaa00; }
    .fail { color: #ff4444; }
    .mono { font-size: 0.7rem; color: #888; word-break: break-all; }
    .pill { display: inline-block; padding: 0.15rem 0.5rem; border-radius: 99px; font-size: 0.7rem; }
    .pill-ok   { background: #003322; color: #00ff88; border: 1px solid #005533; }
    .pill-warn { background: #332200; color: #ffaa00; border: 1px solid #553300; }
    .pill-fail { background: #330011; color: #ff4444; border: 1px solid #550022; }
    .pill-pending { background: #1a1a2e; color: #8888ff; border: 1px solid #2a2a5e; }
    button {
      background: #00ff88; color: #000; border: none; padding: 0.5rem 1.25rem;
      cursor: pointer; font-family: monospace; font-size: 0.9rem; border-radius: 4px;
      font-weight: bold; margin-top: 0.5rem;
    }
    button:hover { background: #00cc66; }
    button:active { background: #009944; }
    button:disabled { background: #333; color: #666; cursor: not-allowed; }
    #sim-result { margin-top: 0.75rem; font-size: 0.75rem; color: #888; }
    #sim-result pre { background: #1a1a1a; padding: 0.75rem; border-radius: 4px; overflow-x: auto; color: #00ff88; }
    .ticker { position: fixed; top: 1rem; right: 1.5rem; font-size: 0.7rem; color: #333; }
    .ticker.live { color: #00ff44; }
    .empty { color: #333; font-style: italic; padding: 0.5rem 0; font-size: 0.8rem; }
  </style>
</head>
<body>
  <h1>⚡ DICE Coordinator</h1>
  <p class="subtitle">Distributed Infrastructure for Cryptographic Entropy — Simulation Dashboard</p>
  <div class="ticker" id="ticker">○ connecting...</div>

  <div class="grid">
    <div class="card">
      <h2>Nodes Online</h2>
      <div class="stat" id="stat-nodes">—</div>
      <div class="stat-label">connected via WebSocket</div>
    </div>
    <div class="card">
      <h2>Rounds Completed</h2>
      <div class="stat" id="stat-rounds">—</div>
      <div class="stat-label">in this session</div>
    </div>
  </div>

  <div class="card" style="margin-bottom:1.5rem">
    <h2>Connected Nodes</h2>
    <div id="nodes-body"><p class="empty">No nodes connected yet</p></div>
  </div>

  <div class="card" style="margin-bottom:1.5rem">
    <h2>Simulate Round</h2>
    <p style="color:#666;font-size:0.8rem;margin-bottom:0.5rem">
      Dispatches a JobAssignment to all connected nodes and runs the commit-reveal protocol.
    </p>
    <button id="sim-btn" onclick="simulate()">▶ POST /simulate</button>
    <div id="sim-result"></div>
  </div>

  <div class="card">
    <h2>Recent Rounds</h2>
    <div id="rounds-body"><p class="empty">No rounds yet — click Simulate Round above</p></div>
  </div>

  <script>
    let roundCount = 0;

    function pillClass(status) {
      if (status === 'finalized') return 'pill-ok';
      if (status === 'failed') return 'pill-fail';
      if (status === 'collecting_commits' || status === 'collecting_reveals') return 'pill-pending';
      return 'pill-warn';
    }

    async function refresh() {
      try {
        const [nr, rr] = await Promise.all([fetch('/nodes'), fetch('/rounds')]);
        const nd = await nr.json();
        const rd = await rr.json();
        const nodes = nd.nodes || [];
        const rounds = rd.rounds || [];

        document.getElementById('stat-nodes').textContent = nodes.length;
        const done = rounds.filter(r => r.status === 'finalized' || r.status === 'failed').length;
        document.getElementById('stat-rounds').textContent = done;

        if (nodes.length === 0) {
          document.getElementById('nodes-body').innerHTML = '<p class="empty">No nodes connected yet — start mock-firmware-node</p>';
        } else {
          document.getElementById('nodes-body').innerHTML =
            '<table><thead><tr><th>Node ID</th><th>Latency</th><th>Uptime</th><th>Jobs</th><th>Connected</th></tr></thead><tbody>' +
            nodes.map(n =>
              `<tr>
                <td class="mono">${n.node_id.substring(0,20)}…</td>
                <td>${n.latency_ms} ms</td>
                <td>${n.uptime_secs} s</td>
                <td>${n.jobs_completed}</td>
                <td>${n.connected_secs} s</td>
              </tr>`
            ).join('') + '</tbody></table>';
        }

        if (rounds.length === 0) {
          document.getElementById('rounds-body').innerHTML = '<p class="empty">No rounds yet</p>';
        } else {
          document.getElementById('rounds-body').innerHTML =
            '<table><thead><tr><th>Request ID</th><th>Status</th><th>Nodes</th><th>Randomness</th></tr></thead><tbody>' +
            rounds.map(r =>
              `<tr>
                <td class="mono">${r.request_id.substring(0,20)}…</td>
                <td><span class="pill ${pillClass(r.status)}">${r.status}</span></td>
                <td>${r.node_count}</td>
                <td class="mono">${r.randomness ? r.randomness.substring(0,24)+'…' : '—'}</td>
              </tr>`
            ).join('') + '</tbody></table>';
        }

        document.getElementById('ticker').textContent = '● live  ' + new Date().toLocaleTimeString();
        document.getElementById('ticker').className = 'ticker live';
      } catch(e) {
        document.getElementById('ticker').textContent = '○ offline';
        document.getElementById('ticker').className = 'ticker';
      }
    }

    async function simulate() {
      const btn = document.getElementById('sim-btn');
      btn.disabled = true;
      btn.textContent = '⏳ dispatching...';
      document.getElementById('sim-result').innerHTML = '';
      try {
        const resp = await fetch('/simulate', { method: 'POST' });
        const data = await resp.json();
        if (resp.ok) {
          document.getElementById('sim-result').innerHTML =
            '<pre>' + JSON.stringify(data, null, 2) + '</pre>';
        } else {
          document.getElementById('sim-result').innerHTML =
            `<span style="color:#ff4444">Error: ${data.error || JSON.stringify(data)}</span>`;
        }
        setTimeout(refresh, 300);
      } catch(e) {
        document.getElementById('sim-result').innerHTML =
          `<span style="color:#ff4444">Network error: ${e.message}</span>`;
      } finally {
        btn.disabled = false;
        btn.textContent = '▶ POST /simulate';
      }
    }

    refresh();
    setInterval(refresh, 2000);
  </script>
</body>
</html>"#;

async fn dashboard() -> Html<&'static str> {
    Html(DASHBOARD_HTML)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn health() -> Response {
    (StatusCode::OK, Json(json!({ "status": "ok" }))).into_response()
}

async fn list_nodes(State(state): State<AppState>) -> Response {
    let nodes = get_active_node_infos(&state.registry).await;
    Json(json!({ "nodes": nodes, "count": nodes.len() })).into_response()
}

/// `GET /rounds` — list in-memory rounds (most recent first, capped at 50).
async fn list_rounds(State(state): State<AppState>) -> Response {
    let map = state.rounds.lock().await;
    let mut items: Vec<serde_json::Value> = map
        .values()
        .map(|entry| {
            json!({
                "request_id": hex::encode(entry.round.request_id),
                "status": entry.round.status_str(),
                "node_count": entry.round.selected_nodes.len(),
                "randomness": entry.round.randomness().map(hex::encode),
                "elapsed_ms": entry.started_at.elapsed().as_millis() as u64,
            })
        })
        .collect();
    // Most recent entries first (by elapsed time ascending = most recent last inserted).
    items.sort_by_key(|v| v["elapsed_ms"].as_u64().unwrap_or(0));
    items.truncate(50);
    drop(map);
    Json(json!({ "rounds": items })).into_response()
}

/// `GET /rounds/:id` — fetch a round by UUID from DB (or 404 if no DB).
async fn get_round_handler(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    let round_id = match id.parse::<Uuid>() {
        Ok(u) => u,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "invalid UUID" })),
            )
                .into_response();
        }
    };

    let pool = match &state.db {
        Some(p) => p,
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({ "error": "database not available in simulation mode" })),
            )
                .into_response();
        }
    };

    match get_round(pool, round_id).await {
        Ok(Some(row)) => Json(json!({
            "id": row.id,
            "request_id": hex::encode(&row.request_id),
            "status": row.status,
            "randomness": row.randomness.as_ref().map(hex::encode),
            "created_at": row.created_at,
            "finalized_at": row.finalized_at,
        }))
        .into_response(),

        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "round not found" })),
        )
            .into_response(),

        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

/// `POST /simulate` — trigger a simulated randomness round.
///
/// Selects all currently connected nodes (up to 7), optionally creates
/// the `RandomnessRequest` on-chain (if `on_chain` is configured), then
/// dispatches `JobAssignment` messages to each selected node.
async fn simulate(State(state): State<AppState>) -> Response {
    use rand::RngCore;
    use solana_sdk::signer::Signer;

    let active_nodes = get_active_nodes(&state.registry).await;
    if active_nodes.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "no nodes connected — start mock-firmware-node first" })),
        )
            .into_response();
    }

    // Select up to 7 nodes.
    let count = active_nodes.len().min(7);
    let selected: Vec<[u8; 33]> = active_nodes[..count].to_vec();
    let min_required = count.min(4);

    // Sequence: use a monotonic counter based on current unix time.
    let sequence: u64 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let deadline_ts = sequence + 30;
    let db_id = Uuid::new_v4();

    // Determine requester pubkey (coordinator's key if on-chain, else default).
    let requester = state
        .on_chain
        .as_ref()
        .map(|ctx| ctx.keypair.pubkey())
        .unwrap_or_default();

    // If on-chain context is available, submit `request_randomness` to Solana first.
    let mut tx_signature: Option<String> = None;
    if let Some(ref ctx) = state.on_chain {
        let ix = crate::solana_tx::build_request_randomness_ix(
            &ctx.program_id,
            &ctx.keypair.pubkey(),
            sequence,
            &solana_sdk::pubkey::Pubkey::default(), // no callback in simulation
        );
        match ctx.rpc.sign_and_send(&ctx.keypair, vec![ix]).await {
            Ok(sig) => {
                tracing::info!(signature = %sig, sequence, "request_randomness TX sent to devnet");
                tx_signature = Some(sig.to_string());
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": format!("on-chain request_randomness failed: {}", e) })),
                )
                    .into_response();
            }
        }
    }

    // Use the request PDA pubkey as the request_id for the in-memory round.
    let request_pda = crate::solana_tx::request_pda(
        &state.on_chain.as_ref().map(|c| c.program_id).unwrap_or_default(),
        &requester,
        sequence,
    );
    let mut request_id = [0u8; 32];
    if requester != solana_sdk::pubkey::Pubkey::default() {
        request_id.copy_from_slice(request_pda.as_ref());
    } else {
        rand::thread_rng().fill_bytes(&mut request_id);
    }

    // Build the Round and insert into the round map.
    let round = Round::new(
        request_id,
        selected.clone(),
        min_required,
        Duration::from_secs(30),
    );

    {
        let mut map = state.rounds.lock().await;
        map.insert(
            request_id,
            RoundEntry {
                round,
                db_id,
                started_at: std::time::Instant::now(),
                requester,
                sequence,
            },
        );
    }

    // Create DB record if pool is available.
    if let Some(ref pool) = state.db {
        let node_vecs: Vec<Vec<u8>> = selected.iter().map(|n| n.to_vec()).collect();
        let _ = crate::db::queries::create_round(pool, &request_id, &node_vecs).await;
    }

    // Build and encode the JobAssignment.
    let job = DiceMessage::JobAssignment(JobAssignment {
        request_id: request_id.to_vec(),
        round_seq: sequence,
        deadline_ts,
    });

    let encoded = match job.encode() {
        Ok(b) => b,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("CBOR encode failed: {}", e) })),
            )
                .into_response();
        }
    };

    // Dispatch to each selected node.
    let mut dispatched = 0usize;
    {
        let reg = state.registry.read().await;
        for node_id in &selected {
            if let Some(session) = reg.get(node_id) {
                if session.tx.try_send(encoded.clone()).is_ok() {
                    dispatched += 1;
                }
            }
        }
    }

    let mut resp = json!({
        "request_id": hex::encode(request_id),
        "round_id": db_id.to_string(),
        "selected_nodes": selected.iter().map(hex::encode).collect::<Vec<_>>(),
        "min_required": min_required,
        "dispatched": dispatched,
        "sequence": sequence,
    });

    if let Some(sig) = tx_signature {
        resp["tx_signature"] = json!(sig);
        resp["explorer"] = json!(format!(
            "https://explorer.solana.com/tx/{}?cluster=devnet",
            sig
        ));
    }

    Json(resp).into_response()
}

/// `GET /metrics` — Prometheus text exposition.
async fn metrics_handler(State(state): State<AppState>) -> Response {
    let body = state.metrics.render();
    (
        StatusCode::OK,
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        body,
    )
        .into_response()
}
