use std::sync::Arc;
use std::time::Duration;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    middleware,
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde_json::json;
use uuid::Uuid;

use super::auth::{AuthState, RateLimiter};

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

/// A completed round kept in history.
#[derive(Clone, serde::Serialize)]
pub struct CompletedRound {
    pub request_id: String,
    pub randomness: String,
    pub node_count: usize,
    pub elapsed_ms: u64,
    pub timestamp: u64,
    pub status: String,
}

/// Ring buffer of recently completed rounds (in-memory, no DB needed).
pub type RoundHistory = std::sync::Arc<tokio::sync::Mutex<Vec<CompletedRound>>>;

#[derive(Clone)]
pub struct AppState {
    pub registry: NodeRegistry,
    pub metrics: Metrics,
    pub db: Option<sqlx::PgPool>,
    pub rounds: RoundMap,
    pub round_history: RoundHistory,
    pub request_queue: crate::queue::SharedQueue,
    pub rate_limiter: Arc<RateLimiter>,
    /// If set, transactions are submitted to Solana devnet/mainnet.
    pub on_chain: Option<OnChainCtx>,
}

// ---------------------------------------------------------------------------
// Router builder
// ---------------------------------------------------------------------------

pub fn build_router(state: AppState, api_key: Option<String>) -> Router {
    let auth_state = AuthState { api_key };

    // Public routes — no auth required (monitoring)
    let public = Router::new()
        .route("/health", get(health))
        .route("/metrics", get(metrics_handler))
        .with_state(state.clone());

    // Protected routes — require API key (if configured)
    let protected = Router::new()
        .route("/", get(dashboard))
        .route("/nodes", get(list_nodes))
        .route("/rounds", get(list_rounds))
        .route("/rounds/:id", get(get_round_handler))
        .route("/queue", get(queue_status))
        .route("/simulate", post(simulate))
        .layer(middleware::from_fn_with_state(
            auth_state,
            super::auth::require_api_key,
        ))
        .with_state(state);

    public.merge(protected)
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
    .subtitle { color: #555; font-size: 0.85rem; margin-bottom: 0.5rem; }
    .version-badge { display: inline-block; padding: 0.2rem 0.6rem; border-radius: 4px; font-size: 0.7rem; font-weight: bold; margin-bottom: 1.5rem; background: #002211; color: #00ff88; border: 1px solid #004422; }
    .grid { display: grid; grid-template-columns: 1fr 1fr; gap: 1.5rem; margin-bottom: 1.5rem; }
    .grid-3 { display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 1.5rem; margin-bottom: 1.5rem; }
    .grid-4 { display: grid; grid-template-columns: 1fr 1fr 1fr 1fr; gap: 1.5rem; margin-bottom: 1.5rem; }
    .card { border: 1px solid #1e1e1e; background: #111; padding: 1.25rem; border-radius: 6px; }
    .card h2 { color: #aaa; font-size: 0.8rem; text-transform: uppercase; letter-spacing: 0.1em; margin-bottom: 1rem; }
    .card.full { grid-column: 1 / -1; }
    .stat { font-size: 2.5rem; color: #00ff88; font-weight: bold; }
    .stat-sm { font-size: 1.8rem; color: #00ff88; font-weight: bold; }
    .stat-label { color: #555; font-size: 0.75rem; margin-top: 0.25rem; }
    .stat-accent { color: #ffaa00; }
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
    .pill-idle { background: #1a1a1a; color: #666; border: 1px solid #333; }
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
    .section-title { color: #555; font-size: 0.7rem; text-transform: uppercase; letter-spacing: 0.15em; margin-bottom: 0.75rem; margin-top: 1.5rem; padding-bottom: 0.5rem; border-bottom: 1px solid #1a1a1a; }
    /* Pipeline visualization */
    .pipeline { display: flex; align-items: center; gap: 0; margin: 1rem 0; }
    .pipeline-step { flex: 1; text-align: center; padding: 0.6rem 0.25rem; font-size: 0.65rem; text-transform: uppercase; letter-spacing: 0.05em; border: 1px solid #1e1e1e; position: relative; }
    .pipeline-step.active { background: #002211; color: #00ff88; border-color: #004422; }
    .pipeline-step.done { background: #001a0e; color: #007744; border-color: #003322; }
    .pipeline-step.waiting { background: #111; color: #333; }
    .pipeline-step.failed { background: #1a0008; color: #ff4444; border-color: #330011; }
    .pipeline-arrow { color: #333; font-size: 0.8rem; padding: 0 0.15rem; flex-shrink: 0; }
    .pipeline-arrow.active { color: #00ff88; }
    /* Progress bar */
    .progress-wrap { background: #1a1a1a; border-radius: 4px; height: 6px; margin-top: 0.5rem; overflow: hidden; }
    .progress-bar { height: 100%; background: #00ff88; border-radius: 4px; transition: width 0.3s; }
    .progress-bar.warn { background: #ffaa00; }
    /* Cost ticker */
    .cost-saved { color: #00ff88; font-size: 0.75rem; margin-top: 0.5rem; }
  </style>
</head>
<body>
  <h1>DICE Coordinator</h1>
  <p class="subtitle">Distributed Infrastructure for Cryptographic Entropy</p>
  <div class="version-badge">v2.0 CHANNEL DESIGN</div>
  <div class="ticker" id="ticker">connecting...</div>

  <!-- Top stats row -->
  <div class="grid-4">
    <div class="card">
      <h2>Nodes Online</h2>
      <div class="stat-sm" id="stat-nodes">--</div>
      <div class="stat-label">connected via WebSocket</div>
    </div>
    <div class="card">
      <h2>Rounds Completed</h2>
      <div class="stat-sm" id="stat-rounds">--</div>
      <div class="stat-label">finalized this session</div>
    </div>
    <div class="card">
      <h2>Success Rate</h2>
      <div class="stat-sm" id="stat-success">--</div>
      <div class="stat-label">finalized / total</div>
    </div>
    <div class="card">
      <h2>SOL Saved (v2)</h2>
      <div class="stat-sm stat-accent" id="stat-saved">--</div>
      <div class="stat-label">vs v1.0 per-PDA model</div>
    </div>
  </div>

  <!-- Round lifecycle pipeline -->
  <div class="section-title">Round Lifecycle Pipeline</div>
  <div class="card" style="margin-bottom:1.5rem">
    <div class="pipeline" id="pipeline">
      <div class="pipeline-step waiting" id="pipe-idle">Idle</div>
      <div class="pipeline-arrow">&rarr;</div>
      <div class="pipeline-step waiting" id="pipe-request">Request</div>
      <div class="pipeline-arrow">&rarr;</div>
      <div class="pipeline-step waiting" id="pipe-select">Select Nodes</div>
      <div class="pipeline-arrow">&rarr;</div>
      <div class="pipeline-step waiting" id="pipe-commit">Commits</div>
      <div class="pipeline-arrow">&rarr;</div>
      <div class="pipeline-step waiting" id="pipe-reveal">Reveals</div>
      <div class="pipeline-arrow">&rarr;</div>
      <div class="pipeline-step waiting" id="pipe-finalize">Finalize</div>
      <div class="pipeline-arrow">&rarr;</div>
      <div class="pipeline-step waiting" id="pipe-callback">Callback</div>
    </div>
    <div class="grid" style="margin-top:1rem;margin-bottom:0">
      <div>
        <span style="color:#555;font-size:0.7rem">COMMITS</span>
        <div style="display:flex;align-items:center;gap:0.5rem">
          <span style="font-size:1rem;color:#00ff88" id="commit-count">0/0</span>
          <div class="progress-wrap" style="flex:1"><div class="progress-bar" id="commit-bar" style="width:0%"></div></div>
        </div>
      </div>
      <div>
        <span style="color:#555;font-size:0.7rem">REVEALS</span>
        <div style="display:flex;align-items:center;gap:0.5rem">
          <span style="font-size:1rem;color:#00ff88" id="reveal-count">0/0</span>
          <div class="progress-wrap" style="flex:1"><div class="progress-bar" id="reveal-bar" style="width:0%"></div></div>
        </div>
      </div>
    </div>
  </div>

  <!-- Connected nodes -->
  <div class="section-title">Connected Nodes</div>
  <div class="card" style="margin-bottom:1.5rem">
    <div id="nodes-body"><p class="empty">No nodes connected yet -- start mock-firmware-node</p></div>
  </div>

  <!-- Simulate -->
  <div class="section-title">Simulation</div>
  <div class="card" style="margin-bottom:1.5rem">
    <h2>Trigger Round</h2>
    <p style="color:#666;font-size:0.8rem;margin-bottom:0.5rem">
      Dispatches a JobAssignment to connected nodes. Uses v2.0 channel design -- no new PDAs created per round.
    </p>
    <button id="sim-btn" onclick="simulate()">POST /simulate</button>
    <div id="sim-result"></div>
  </div>

  <!-- Recent rounds -->
  <div class="section-title">Recent Rounds</div>
  <div class="card">
    <div id="rounds-body"><p class="empty">No rounds yet -- click Simulate Round above</p></div>
  </div>

  <script>
    const COST_V1_PER_ROUND = 0.036;
    const COST_V2_PER_ROUND = 0.002;

    function pillClass(status) {
      if (status === 'finalized') return 'pill-ok';
      if (status === 'failed') return 'pill-fail';
      if (status === 'collecting_commits' || status === 'collecting_reveals') return 'pill-pending';
      return 'pill-warn';
    }

    function updatePipeline(latestRound) {
      const steps = ['pipe-idle','pipe-request','pipe-select','pipe-commit','pipe-reveal','pipe-finalize','pipe-callback'];
      const arrows = document.querySelectorAll('.pipeline-arrow');
      steps.forEach(id => { document.getElementById(id).className = 'pipeline-step waiting'; });
      arrows.forEach(a => a.className = 'pipeline-arrow');

      if (!latestRound) {
        document.getElementById('pipe-idle').className = 'pipeline-step active';
        return;
      }

      const s = latestRound.status;
      const stageMap = {
        'collecting_commits': 3,
        'collecting_reveals': 4,
        'finalized': 6,
        'failed': -1
      };
      const stage = stageMap[s] ?? 1;

      if (stage === -1) {
        steps.forEach(id => { document.getElementById(id).className = 'pipeline-step failed'; });
        return;
      }

      for (let i = 0; i < steps.length; i++) {
        const el = document.getElementById(steps[i]);
        if (i < stage) { el.className = 'pipeline-step done'; }
        else if (i === stage) { el.className = 'pipeline-step active'; }
      }
      for (let i = 0; i < arrows.length; i++) {
        if (i < stage) arrows[i].className = 'pipeline-arrow active';
      }

      // Update commit/reveal counts
      const nc = latestRound.node_count || 0;
      const cc = latestRound.commits_received || 0;
      const rc = latestRound.reveals_received || 0;
      document.getElementById('commit-count').textContent = cc + '/' + nc;
      document.getElementById('reveal-count').textContent = rc + '/' + nc;
      document.getElementById('commit-bar').style.width = nc ? (cc/nc*100)+'%' : '0%';
      document.getElementById('reveal-bar').style.width = nc ? (rc/nc*100)+'%' : '0%';
    }

    async function refresh() {
      try {
        const [nr, rr] = await Promise.all([fetch('/nodes'), fetch('/rounds')]);
        const nd = await nr.json();
        const rd = await rr.json();
        const nodes = nd.nodes || [];
        const rounds = rd.rounds || [];

        document.getElementById('stat-nodes').textContent = nodes.length;
        const finalized = rounds.filter(r => r.status === 'finalized').length;
        const total = rounds.filter(r => r.status === 'finalized' || r.status === 'failed').length;
        document.getElementById('stat-rounds').textContent = finalized;
        document.getElementById('stat-success').textContent = total > 0 ? Math.round(finalized/total*100) + '%' : '--';

        // Cost savings vs v1.0
        const saved = finalized * (COST_V1_PER_ROUND - COST_V2_PER_ROUND);
        document.getElementById('stat-saved').textContent = saved > 0 ? saved.toFixed(3) + ' SOL' : '--';

        // Pipeline
        const latest = rounds.length > 0 ? rounds[0] : null;
        updatePipeline(latest);

        // Nodes table
        if (nodes.length === 0) {
          document.getElementById('nodes-body').innerHTML = '<p class="empty">No nodes connected yet -- start mock-firmware-node</p>';
        } else {
          document.getElementById('nodes-body').innerHTML =
            '<table><thead><tr><th>Node ID</th><th>Latency</th><th>Uptime</th><th>Jobs</th><th>Connected</th></tr></thead><tbody>' +
            nodes.map(n =>
              `<tr>
                <td class="mono">${n.node_id.substring(0,20)}...</td>
                <td>${n.latency_ms} ms</td>
                <td>${n.uptime_secs} s</td>
                <td>${n.jobs_completed}</td>
                <td>${n.connected_secs} s</td>
              </tr>`
            ).join('') + '</tbody></table>';
        }

        // Rounds table
        if (rounds.length === 0) {
          document.getElementById('rounds-body').innerHTML = '<p class="empty">No rounds yet</p>';
        } else {
          document.getElementById('rounds-body').innerHTML =
            '<table><thead><tr><th>Request ID</th><th>Status</th><th>Nodes</th><th>Time</th><th>Randomness</th></tr></thead><tbody>' +
            rounds.map(r => {
              const elapsed = r.elapsed_ms < 1000 ? r.elapsed_ms+'ms' : (r.elapsed_ms/1000).toFixed(1)+'s';
              return `<tr>
                <td class="mono">${r.request_id.substring(0,16)}...</td>
                <td><span class="pill ${pillClass(r.status)}">${r.status}</span></td>
                <td>${r.node_count}</td>
                <td>${elapsed}</td>
                <td class="mono">${r.randomness ? r.randomness.substring(0,20)+'...' : '--'}</td>
              </tr>`;
            }).join('') + '</tbody></table>';
        }

        document.getElementById('ticker').textContent = 'LIVE  ' + new Date().toLocaleTimeString();
        document.getElementById('ticker').className = 'ticker live';
      } catch(e) {
        document.getElementById('ticker').textContent = 'OFFLINE';
        document.getElementById('ticker').className = 'ticker';
      }
    }

    async function simulate() {
      const btn = document.getElementById('sim-btn');
      btn.disabled = true;
      btn.textContent = 'dispatching...';
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
        btn.textContent = 'POST /simulate';
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
    // Active rounds (in progress)
    let map = state.rounds.lock().await;
    let mut items: Vec<serde_json::Value> = map
        .values()
        .map(|entry| {
            let (commits_received, reveals_received) = entry.round.progress_counts();
            json!({
                "request_id": hex::encode(entry.round.request_id),
                "status": entry.round.status_str(),
                "node_count": entry.round.selected_nodes.len(),
                "commits_received": commits_received,
                "reveals_received": reveals_received,
                "randomness": entry.round.randomness().map(hex::encode),
                "elapsed_ms": entry.started_at.elapsed().as_millis() as u64,
            })
        })
        .collect();
    drop(map);

    // Completed rounds from history (most recent first)
    let history = state.round_history.lock().await;
    for cr in history.iter().rev().take(50) {
        items.push(json!({
            "request_id": cr.request_id,
            "status": cr.status,
            "node_count": cr.node_count,
            "commits_received": cr.node_count,
            "reveals_received": cr.node_count,
            "randomness": cr.randomness,
            "elapsed_ms": cr.elapsed_ms,
            "timestamp": cr.timestamp,
        }));
    }
    drop(history);

    items.truncate(50);
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

/// `POST /simulate` — trigger a randomness round.
///
/// If a node has capacity (< 12 active rounds), dispatches immediately.
/// Otherwise, queues the request and dispatches when a node finishes a round.
async fn simulate(State(state): State<AppState>) -> Response {
    use rand::RngCore;

    // Rate limiting
    if !state.rate_limiter.try_acquire() {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({ "error": "rate limited", "retry_after_ms": 1000 })),
        )
            .into_response();
    }

    let active_nodes = get_active_nodes(&state.registry).await;
    if active_nodes.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "no nodes connected" })),
        )
            .into_response();
    }

    // Generate unique sequence.
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let base_time: u64 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    let seq_num = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let sequence: u64 = base_time.wrapping_mul(1000).wrapping_add(seq_num);

    let deadline_ts = sequence + 60;
    let db_id = Uuid::new_v4();
    let requester = state
        .on_chain
        .as_ref()
        .map(|ctx| solana_sdk::signer::Signer::pubkey(ctx.keypair.as_ref()))
        .unwrap_or_default();

    // Create on-chain request FIRST — this creates the RandomnessRequest + Escrow PDAs.
    // The request_id is the PDA address so on-chain submit_commit/finalize find the right account.
    let mut request_id = [0u8; 32];
    let mut on_chain_sequence: Option<u64> = None;

    if let Some(ref ctx) = state.on_chain {
        use solana_sdk::signer::Signer;
        let ix = crate::solana_tx::build_request_randomness_ix(
            &ctx.program_id,
            &ctx.keypair.pubkey(),
            sequence,
            &solana_sdk::pubkey::Pubkey::default(), // no callback in simulate
        );
        match ctx.rpc.sign_and_send(&ctx.keypair, vec![ix]).await {
            Ok(sig) => {
                tracing::info!(signature = %sig, sequence, "request_randomness TX sent");

                // Wait for confirmation before proceeding — the PDA must exist on-chain
                // before submit_commit can reference it
                let mut confirmed = false;
                for attempt in 0..15 {
                    tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
                    match ctx.rpc.confirm_transaction(&sig).await {
                        Ok(true) => {
                            tracing::info!(signature = %sig, attempts = attempt + 1, "request_randomness CONFIRMED on-chain");
                            confirmed = true;
                            break;
                        }
                        Ok(false) => continue,
                        Err(e) => {
                            tracing::warn!(error = %e, "confirmation check failed");
                            continue;
                        }
                    }
                }

                if !confirmed {
                    tracing::error!("request_randomness TX not confirmed after 15s — aborting");
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({ "error": "request_randomness TX not confirmed on-chain within 15s" })),
                    ).into_response();
                }

                // Use the request PDA as request_id so submit_commit/finalize find it
                let pda = crate::solana_tx::request_pda(&ctx.program_id, &ctx.keypair.pubkey(), sequence);
                request_id.copy_from_slice(pda.as_ref());
                on_chain_sequence = Some(sequence);
            }
            Err(e) => {
                tracing::error!(error = %e, "request_randomness TX FAILED — aborting round");
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": format!("on-chain request_randomness failed: {}", e),
                        "hint": "coordinator may not have enough SOL or program PDAs may conflict"
                    })),
                ).into_response();
            }
        }
    } else {
        // No on-chain context — use random request_id (local-only mode)
        rand::thread_rng().fill_bytes(&mut request_id);
    }

    // Check if any node has capacity to dispatch immediately.
    let mut queue = state.request_queue.lock().await;
    let queue_depth = queue.queue_len();

    // Find a node with capacity.
    let dispatch_node = active_nodes.iter().find(|nid| queue.node_has_capacity(nid));

    if let Some(&node_id) = dispatch_node {
        // Dispatch immediately.
        queue.mark_dispatched(&node_id);
        drop(queue);

        let dispatched = dispatch_round(
            &state, request_id, sequence, deadline_ts, db_id, requester, &[node_id],
        ).await;

        Json(json!({
            "request_id": hex::encode(request_id),
            "round_id": db_id.to_string(),
            "status": "dispatched",
            "dispatched": dispatched,
            "queued": 0,
            "queue_depth": queue_depth,
            "sequence": sequence,
        })).into_response()
    } else {
        // All nodes are at capacity — queue the request.
        queue.enqueue(crate::queue::QueuedRequest {
            request_id,
            sequence,
            deadline_ts,
            queued_at: std::time::Instant::now(),
            requester,
            db_id,
        });
        let new_depth = queue.queue_len();
        drop(queue);

        tracing::info!(
            request = hex::encode(request_id),
            queue_depth = new_depth,
            "request queued — all nodes at capacity"
        );

        Json(json!({
            "request_id": hex::encode(request_id),
            "round_id": db_id.to_string(),
            "status": "queued",
            "dispatched": 0,
            "queue_depth": new_depth,
            "sequence": sequence,
        })).into_response()
    }
}

/// Helper: create a round, encode JobAssignment, dispatch to nodes.
async fn dispatch_round(
    state: &AppState,
    request_id: [u8; 32],
    sequence: u64,
    deadline_ts: u64,
    db_id: Uuid,
    requester: solana_sdk::pubkey::Pubkey,
    selected: &[[u8; 33]],
) -> usize {
    let min_required = selected.len().min(4);

    let round = Round::new(
        request_id,
        selected.to_vec(),
        min_required,
        Duration::from_secs(60),
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
                channel_authority: None,
                channel_index: None,
            },
        );
    }

    if let Some(ref pool) = state.db {
        let node_vecs: Vec<Vec<u8>> = selected.iter().map(|n| n.to_vec()).collect();
        let _ = crate::db::queries::create_round(pool, &request_id, &node_vecs).await;
    }

    let job = DiceMessage::JobAssignment(JobAssignment {
        request_id: request_id.to_vec(),
        round_seq: sequence,
        deadline_ts,
    });

    let encoded = match job.encode() {
        Ok(b) => b,
        Err(_) => return 0,
    };

    let mut dispatched = 0usize;
    let reg = state.registry.read().await;
    for node_id in selected {
        if let Some(session) = reg.get(node_id) {
            if session.tx.try_send(encoded.clone()).is_ok() {
                dispatched += 1;
            }
        }
    }

    dispatched
}

/// `GET /metrics` — Prometheus text exposition.
/// `GET /queue` — queue status and per-node active round counts.
async fn queue_status(State(state): State<AppState>) -> Response {
    let q = state.request_queue.lock().await;
    let active_nodes = get_active_nodes(&state.registry).await;

    let node_loads: Vec<serde_json::Value> = active_nodes.iter().map(|nid| {
        json!({
            "node_id": hex::encode(nid),
            "active_rounds": q.node_active_count(nid),
            "capacity": crate::queue::MAX_CONCURRENT_PER_NODE,
            "has_capacity": q.node_has_capacity(nid),
        })
    }).collect();

    Json(json!({
        "pending": q.queue_len(),
        "total_dispatched": q.total_dispatched,
        "total_queued": q.total_queued,
        "total_dropped": q.total_dropped,
        "nodes": node_loads,
    })).into_response()
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
