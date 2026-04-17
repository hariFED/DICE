// DICE Coordinator — entry point
// Wires together: config, DB, WebSocket server (mTLS or plain), REST API, metrics.

mod api;
mod config;
mod db;
mod feed_crank;
mod metrics;
mod node_session;
mod protocol;
pub mod queue;
mod selection;
mod solana_rpc;
mod solana_tx;
mod solana_watcher;
mod solana_ws;
mod state_machine;

#[cfg(test)]
mod integration_tests;
#[cfg(test)]
mod vrf_proof_tests;

use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use anyhow::{Context, Result};
use axum::Router;
use clap::Parser;
use tokio::{net::TcpListener, sync::mpsc};
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use solana_sdk::signer::Signer as _;

use crate::{
    api::routes::{build_router, AppState},
    config::Config,
    metrics::Metrics,
    node_session::{deregister, new_registry, register, update_heartbeat, NodeRegistry},
    protocol::messages::{DiceMessage, RoundResult},
    solana_tx::OnChainCtx,
    state_machine::RoundMap,
};

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Parse config from CLI flags / environment variables.
    let cfg = Config::parse();

    // 2. Initialise structured logging.
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    if cfg.simulation {
        info!("SIMULATION MODE — plain WebSocket, no DB, no Solana RPC");
    }

    info!(
        ws_port = cfg.ws_port,
        api_port = cfg.api_port,
        metrics_port = cfg.metrics_port,
        simulation = cfg.simulation,
        "DICE Coordinator starting"
    );

    // 3. Connect to PostgreSQL (skipped in simulation mode, also skipped
    //    if DICE_SKIP_DB=1 is set — useful for a stress run when the
    //    local DNS is temporarily refusing to resolve the DB hostname
    //    but we still want the on-chain dispatch loop running).
    let skip_db = std::env::var("DICE_SKIP_DB").unwrap_or_default() == "1";
    let pool: Option<sqlx::PgPool> = if cfg.simulation || skip_db {
        if skip_db {
            warn!("DICE_SKIP_DB=1 — running without round history persistence");
        }
        None
    } else {
        // Reject default/weak credentials in production
        if cfg.database_url.is_empty() || cfg.database_url.contains("dice:dice@") {
            anyhow::bail!(
                "DATABASE_URL must be set with non-default credentials in production mode. \
                 Use --simulation for local testing without a database."
            );
        }
        match sqlx::PgPool::connect(&cfg.database_url).await {
            Ok(p) => {
                run_migrations(&p).await?;
                Some(p)
            }
            Err(e) => {
                warn!(
                    error = %e,
                    "DATABASE_URL connect failed — continuing without DB persistence. \
                     Round history will NOT be written to Postgres. Set DICE_SKIP_DB=1 \
                     to suppress this retry path explicitly."
                );
                None
            }
        }
    };

    // 4. Initialise shared state.
    let registry: NodeRegistry = new_registry();
    let metrics = Metrics::new();
    let rounds: RoundMap = Arc::new(tokio::sync::Mutex::new(HashMap::new()));

    // 4b. Try to load coordinator keypair for on-chain transaction submission.
    let on_chain: Option<OnChainCtx> = if cfg.coordinator_keypair_path.exists() {
        match solana_rpc::load_keypair(&cfg.coordinator_keypair_path) {
            Ok(keypair) => {
                let program_id: solana_sdk::pubkey::Pubkey =
                    "78Qv6cyKkRZN2YngiLSSBCe2iyRc6jgtCs3incCaMRcv"
                        .parse()
                        .expect("parse program ID");

                // Parse treasury/reserve config. Default to Pubkey::default()
                // (all-zero) if unset — claim_rewards_v2 logic checks for this
                // sentinel and skips submission rather than paying into a burn
                // address.
                let treasury = cfg
                    .treasury
                    .as_deref()
                    .and_then(|s| s.parse::<solana_sdk::pubkey::Pubkey>().ok())
                    .unwrap_or_default();
                let reserve = cfg
                    .reserve
                    .as_deref()
                    .and_then(|s| s.parse::<solana_sdk::pubkey::Pubkey>().ok())
                    .unwrap_or_default();

                if treasury == solana_sdk::pubkey::Pubkey::default()
                    || reserve == solana_sdk::pubkey::Pubkey::default()
                {
                    warn!(
                        "treasury/reserve not configured — claim_rewards_v2 will NOT be \
                         submitted post-finalization. Set --treasury and --reserve (or \
                         DICE_TREASURY / DICE_RESERVE) to enable node payouts."
                    );
                }

                info!(
                    coordinator = %solana_sdk::signer::Signer::pubkey(&keypair),
                    program = %program_id,
                    rpc = %cfg.solana_rpc_url,
                    treasury = %treasury,
                    reserve = %reserve,
                    "on-chain transactions ENABLED"
                );
                Some(OnChainCtx {
                    rpc: Arc::new(solana_rpc::SolanaRpc::new(&cfg.solana_rpc_url)),
                    keypair: Arc::new(keypair),
                    program_id,
                    treasury,
                    reserve,
                })
            }
            Err(e) => {
                warn!(error = %e, "could not load coordinator keypair — on-chain txs DISABLED");
                None
            }
        }
    } else {
        info!("no coordinator keypair found — on-chain txs DISABLED (in-memory only)");
        None
    };

    // 4c. Round history buffer for dashboard.
    let round_history: api::routes::RoundHistory =
        Arc::new(tokio::sync::Mutex::new(Vec::with_capacity(100)));

    // 4d. Request queue for burst handling.
    let request_queue = queue::new_queue();

    // 5. Spawn Axum REST API server.
    let api_state = AppState {
        registry: registry.clone(),
        metrics: metrics.clone(),
        db: pool.clone(),
        rounds: rounds.clone(),
        round_history: round_history.clone(),
        request_queue: request_queue.clone(),
        rate_limiter: std::sync::Arc::new(api::auth::RateLimiter::new(cfg.rate_limit_rps)),
        on_chain: on_chain.clone(),
    };
    let api_handle = {
        let port = cfg.api_port;
        let api_key = cfg.api_key.clone();
        let router = build_router(api_state, api_key);
        tokio::spawn(async move {
            if let Err(e) = serve_axum(router, port).await {
                error!(?e, "REST API server error");
            }
        })
    };

    // 6. Spawn Prometheus metrics server.
    let metrics_handle = {
        let m = metrics.clone();
        let port = cfg.metrics_port;
        tokio::spawn(async move {
            if let Err(e) = serve_metrics(m, port).await {
                error!(?e, "metrics server error");
            }
        })
    };

    // 7. Spawn WebSocket server: plain WS or mTLS.
    //    - simulation + no --tls → plain WS
    //    - simulation + --tls   → mTLS (test certs with no DB)
    //    - production (no --simulation) → always mTLS
    let use_tls = !cfg.simulation || cfg.tls;
    let ws_handle = if !use_tls {
        let reg = registry.clone();
        let m = metrics.clone();
        let r = rounds.clone();
        let rh = round_history.clone();
        let rq = request_queue.clone();
        let port = cfg.ws_port;
        let db_opt = pool.clone();
        let oc = on_chain.clone();
        tokio::spawn(async move {
            if let Err(e) = serve_websocket_plain(reg, r, rh, rq, m, db_opt, oc, port).await {
                error!(?e, "plain WebSocket server error");
            }
        })
    } else {
        let reg = registry.clone();
        let m = metrics.clone();
        let r = rounds.clone();
        let rh = round_history.clone();
        let rq = request_queue.clone();
        let port = cfg.ws_port;
        let tls_cert = cfg.tls_cert_path.clone();
        let tls_key = cfg.tls_key_path.clone();
        let ca_cert = cfg.ca_cert_path.clone();
        let db_opt = pool.clone();
        let oc = on_chain.clone();
        tokio::spawn(async move {
            if let Err(e) = serve_websocket_mtls(reg, r, rh, rq, m, db_opt, oc, port, tls_cert, tls_key, ca_cert).await {
                error!(?e, "mTLS WebSocket server error");
            }
        })
    };

    // 8. Spawn round timeout watchdog (checks every 5 seconds).
    let timeout_handle = {
        let r = rounds.clone();
        let m = metrics.clone();
        let reg = registry.clone();
        tokio::spawn(async move {
            round_timeout_watchdog(r, m, reg).await;
        })
    };

    // 9. Spawn Solana watcher (production mode only).
    //    Uses HTTP polling via reqwest because tokio-tungstenite is built
    //    without a TLS feature in this workspace (rustls 0.21 pin conflict
    //    with rustls 0.22). Polls Pending DiceChannels every 3s.
    let watcher_handle = if !cfg.simulation {
        let rpc = std::sync::Arc::new(solana_rpc::SolanaRpc::new(&cfg.solana_rpc_url));
        let keypair = std::sync::Arc::new(
            solana_rpc::load_keypair(&cfg.coordinator_keypair_path)
                .expect("load coordinator keypair"),
        );
        let program_id: solana_sdk::pubkey::Pubkey = "78Qv6cyKkRZN2YngiLSSBCe2iyRc6jgtCs3incCaMRcv"
            .parse()
            .expect("parse program ID");
        let reg = registry.clone();
        let r = rounds.clone();
        let m = metrics.clone();
        let min = cfg.min_nodes as usize;
        let max = cfg.max_nodes as usize;
        let timeout = std::time::Duration::from_secs(cfg.commit_timeout_secs);
        Some(tokio::spawn(async move {
            solana_ws::run_dice_channel_poller(
                rpc, program_id, keypair, reg, r, m, min, max, timeout,
            )
            .await;
        }))
    } else {
        info!("Solana WebSocket subscriber disabled in simulation mode");
        None
    };

    // 9b. Spawn streaming-VRF feed crank (production mode only).
    let feed_crank_handle = if !cfg.simulation {
        let rpc = std::sync::Arc::new(solana_rpc::SolanaRpc::new(&cfg.solana_rpc_url));
        let keypair = std::sync::Arc::new(
            solana_rpc::load_keypair(&cfg.coordinator_keypair_path)
                .expect("load coordinator keypair"),
        );
        let program_id: solana_sdk::pubkey::Pubkey = "78Qv6cyKkRZN2YngiLSSBCe2iyRc6jgtCs3incCaMRcv"
            .parse()
            .expect("parse program ID");
        Some(tokio::spawn(async move {
            feed_crank::run_feed_crank(rpc, program_id, keypair).await;
        }))
    } else {
        info!("Feed crank disabled in simulation mode");
        None
    };

    // 10. Ready banner.
    println!("DICE Coordinator ready:");
    println!("  Dashboard : http://localhost:{}/", cfg.api_port);
    println!("  WebSocket : {}://localhost:{}/", if use_tls { "wss" } else { "ws" }, cfg.ws_port);
    println!("  Metrics   : http://localhost:{}/metrics", cfg.metrics_port);
    if cfg.simulation {
        println!("  Simulate  : curl -X POST http://localhost:{}/simulate", cfg.api_port);
    } else {
        println!("  Solana    : WebSocket subscriber on {}", cfg.solana_rpc_url);
    }

    // 11. Wait for any task to exit.
    tokio::select! {
        _ = api_handle      => warn!("REST API task exited"),
        _ = metrics_handle  => warn!("metrics task exited"),
        _ = ws_handle       => warn!("WebSocket task exited"),
        _ = timeout_handle  => warn!("timeout watchdog exited"),
        _ = async {
            if let Some(h) = watcher_handle { h.await } else { std::future::pending().await }
        } => warn!("Solana watcher exited"),
        _ = async {
            if let Some(h) = feed_crank_handle { h.await } else { std::future::pending().await }
        } => warn!("feed crank exited"),
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// REST API server
// ---------------------------------------------------------------------------

async fn serve_axum(router: Router, port: u16) -> Result<()> {
    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse()?;
    let listener = TcpListener::bind(addr).await?;
    info!(addr = %addr, "REST API listening");
    axum::serve(listener, router).await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Prometheus metrics server
// ---------------------------------------------------------------------------

async fn serve_metrics(metrics: Metrics, port: u16) -> Result<()> {
    use axum::{response::IntoResponse, routing::get};

    let router = Router::new().route(
        "/metrics",
        get(move || {
            let m = metrics.clone();
            async move {
                let body = m.render();
                (
                    axum::http::StatusCode::OK,
                    [(
                        axum::http::header::CONTENT_TYPE,
                        "text/plain; version=0.0.4; charset=utf-8",
                    )],
                    body,
                )
                    .into_response()
            }
        }),
    );

    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse()?;
    let listener = TcpListener::bind(addr).await?;
    info!(addr = %addr, "Prometheus metrics listening");
    axum::serve(listener, router).await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Plain WebSocket server (simulation mode — no TLS)
// ---------------------------------------------------------------------------

async fn serve_websocket_plain(
    registry: NodeRegistry,
    rounds: RoundMap,
    round_history: api::routes::RoundHistory,
    request_queue: queue::SharedQueue,
    metrics: Metrics,
    db: Option<sqlx::PgPool>,
    on_chain: Option<OnChainCtx>,
    port: u16,
) -> Result<()> {
    use tokio_tungstenite::accept_async;

    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse()?;
    let listener = TcpListener::bind(addr).await?;
    info!(addr = %addr, "Plain WebSocket server listening (simulation mode)");

    loop {
        let (tcp_stream, peer_addr) = listener.accept().await?;
        info!(%peer_addr, "incoming TCP connection (simulation)");

        let registry = registry.clone();
        let rounds = rounds.clone();
        let round_history = round_history.clone();
        let request_queue = request_queue.clone();
        let metrics = metrics.clone();
        let db = db.clone();
        let oc = on_chain.clone();

        tokio::spawn(async move {
            match accept_async(tcp_stream).await {
                Ok(ws) => {
                    handle_node_connection(ws, registry, rounds, round_history, request_queue, metrics, db, oc).await;
                }
                Err(e) => warn!(%peer_addr, ?e, "WebSocket upgrade failed"),
            }
        });
    }
}

// ---------------------------------------------------------------------------
// mTLS WebSocket server (production mode)
// ---------------------------------------------------------------------------

async fn serve_websocket_mtls(
    registry: NodeRegistry,
    rounds: RoundMap,
    round_history: api::routes::RoundHistory,
    request_queue: queue::SharedQueue,
    metrics: Metrics,
    db: Option<sqlx::PgPool>,
    on_chain: Option<OnChainCtx>,
    port: u16,
    tls_cert: std::path::PathBuf,
    tls_key: std::path::PathBuf,
    ca_cert: std::path::PathBuf,
) -> Result<()> {
    use rustls::{server::AllowAnyAuthenticatedClient, Certificate, PrivateKey,
                 RootCertStore, ServerConfig};
    use std::{fs, io::BufReader};
    use tokio_tungstenite::accept_async_with_config;

    let cert_file =
        fs::File::open(&tls_cert).with_context(|| format!("open TLS cert {:?}", tls_cert))?;
    let certs: Vec<Certificate> = rustls_pemfile::certs(&mut BufReader::new(cert_file))
        .context("parse TLS cert chain")?
        .into_iter()
        .map(Certificate)
        .collect();

    let key_file =
        fs::File::open(&tls_key).with_context(|| format!("open TLS key {:?}", tls_key))?;
    let key = PrivateKey(
        rustls_pemfile::pkcs8_private_keys(&mut BufReader::new(key_file))
            .context("parse TLS private key")?
            .into_iter()
            .next()
            .context("no private key found in key file")?,
    );

    let ca_file =
        fs::File::open(&ca_cert).with_context(|| format!("open CA cert {:?}", ca_cert))?;
    let mut ca_roots = RootCertStore::empty();
    for ca in rustls_pemfile::certs(&mut BufReader::new(ca_file))
        .context("parse CA certs")?
    {
        ca_roots.add(&Certificate(ca)).context("add CA cert to root store")?;
    }

    let client_verifier = AllowAnyAuthenticatedClient::new(ca_roots);
    let tls_config = ServerConfig::builder()
        .with_safe_defaults()
        .with_client_cert_verifier(Arc::new(client_verifier))
        .with_single_cert(certs, key)
        .context("build TLS server config")?;

    let tls_acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(tls_config));
    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse()?;
    let listener = TcpListener::bind(addr).await?;
    info!(addr = %addr, "mTLS WebSocket server listening");

    loop {
        let (tcp_stream, peer_addr) = listener.accept().await?;
        info!(%peer_addr, "incoming TCP connection");

        let tls_acceptor = tls_acceptor.clone();
        let registry = registry.clone();
        let rounds = rounds.clone();
        let round_history = round_history.clone();
        let request_queue = request_queue.clone();
        let metrics = metrics.clone();
        let db = db.clone();
        let oc = on_chain.clone();

        tokio::spawn(async move {
            match tls_acceptor.accept(tcp_stream).await {
                Ok(tls_stream) => {
                    info!(%peer_addr, "mTLS handshake success");
                    match accept_async_with_config(tls_stream, None).await {
                        Ok(ws) => {
                            handle_node_connection(ws, registry, rounds, round_history, request_queue, metrics, db, oc).await;
                        }
                        Err(e) => warn!(%peer_addr, ?e, "WebSocket upgrade failed"),
                    }
                }
                Err(e) => {
                    warn!(%peer_addr, ?e, "mTLS handshake failed");
                    metrics.mtls_handshake_failed_total.inc();
                }
            }
        });
    }
}

// ---------------------------------------------------------------------------
// Per-node connection handler
// ---------------------------------------------------------------------------

/// Drive a single node's WebSocket session until disconnection.
async fn handle_node_connection<S>(
    ws: tokio_tungstenite::WebSocketStream<S>,
    registry: NodeRegistry,
    rounds: RoundMap,
    round_history: api::routes::RoundHistory,
    request_queue: queue::SharedQueue,
    metrics: Metrics,
    db: Option<sqlx::PgPool>,
    on_chain: Option<OnChainCtx>,
) where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::Message;

    let (mut ws_sink, mut ws_stream) = ws.split();

    // Outbound channel (coordinator → node).
    let (tx, mut rx) = mpsc::channel::<Vec<u8>>(64);

    // Node identity is known after the first Heartbeat.
    let mut node_id_opt: Option<[u8; 33]> = None;

    // Write pump: forward from channel to WebSocket.
    let write_task = tokio::spawn(async move {
        while let Some(msg_bytes) = rx.recv().await {
            if ws_sink
                .send(Message::Binary(msg_bytes.into()))
                .await
                .is_err()
            {
                break;
            }
        }
    });

    // Read loop.
    while let Some(msg_result) = ws_stream.next().await {
        let raw = match msg_result {
            Ok(Message::Binary(b)) => b,
            Ok(Message::Close(_)) => break,
            Ok(_) => continue,
            Err(e) => {
                warn!(?e, "WebSocket read error");
                break;
            }
        };

        let msg = match DiceMessage::decode(&raw) {
            Ok(m) => m,
            Err(e) => {
                warn!(?e, "CBOR decode error");
                continue;
            }
        };

        match msg {
            // ---- Heartbeat -------------------------------------------------
            DiceMessage::Heartbeat(hb) => {
                if hb.node_id.len() != 33 {
                    warn!(len = hb.node_id.len(), "heartbeat node_id wrong length");
                    continue;
                }
                let mut id = [0u8; 33];
                id.copy_from_slice(&hb.node_id);

                if node_id_opt.is_none() {
                    register(&registry, id, tx.clone()).await;
                    metrics.nodes_connected.inc();
                    node_id_opt = Some(id);
                    info!(node = hex::encode(id), "node connected");
                }

                update_heartbeat(
                    &registry,
                    &id,
                    hb.latency_ms as u32,
                    hb.uptime_secs,
                    hb.jobs_completed,
                )
                .await;
            }

            // ---- Commit submission -----------------------------------------
            DiceMessage::CommitSubmission(cs) => {
                let ok = cs.node_id.len() == 33
                    && cs.request_id.len() == 32
                    && cs.commit_hash.len() == 32
                    && cs.signature.len() == 64;
                if !ok {
                    warn!("commit submission has wrong field sizes");
                    continue;
                }

                let mut node_id = [0u8; 33];
                let mut request_id = [0u8; 32];
                let mut commit_hash = [0u8; 32];
                let mut sig = [0u8; 64];
                node_id.copy_from_slice(&cs.node_id);
                request_id.copy_from_slice(&cs.request_id);
                commit_hash.copy_from_slice(&cs.commit_hash);
                sig.copy_from_slice(&cs.signature);

                let mut map = rounds.lock().await;
                if let Some(entry) = map.get_mut(&request_id) {
                    let entry_requester = entry.requester;
                    let entry_sequence = entry.sequence;
                    match entry.round.handle_commit(node_id, commit_hash, sig) {
                        Ok(()) => {
                            let now_in_reveal = entry.round.status_str() == "collecting_reveals";
                            let selected_for_reveal = entry.round.selected_nodes.clone();
                            info!(
                                request = hex::encode(request_id),
                                node = hex::encode(node_id),
                                status = entry.round.status_str(),
                                "commit accepted"
                            );
                            if let Some(ref pool) = db {
                                let _ = db::queries::record_commit(
                                    pool, entry.db_id, &node_id, &commit_hash,
                                )
                                .await;
                            }

                            // If all commits collected, broadcast "reveal" signal to all nodes.
                            if now_in_reveal {
                                let reveal_msg = DiceMessage::RoundResult(RoundResult {
                                    request_id: request_id.to_vec(),
                                    status: "reveal".to_string(),
                                    randomness: vec![0u8; 32], // 32 zero bytes (firmware expects exactly 32)
                                });
                                if let Ok(encoded) = reveal_msg.encode() {
                                    let reg = registry.read().await;
                                    for nid in &selected_for_reveal {
                                        if let Some(session) = reg.get(nid) {
                                            let _ = session.tx.try_send(encoded.clone());
                                        }
                                    }
                                    info!(
                                        request = hex::encode(request_id),
                                        "broadcast reveal signal to {} nodes",
                                        selected_for_reveal.len()
                                    );
                                }
                            }

                            // On-chain commit submission is now BUNDLED with reveal+finalize
                            // in a single TX after the round completes (see reveal handler below).
                            // This reduces latency from 3 TXs to 1 TX.
                        }
                        Err(e) => {
                            warn!(
                                request = hex::encode(request_id),
                                node = hex::encode(node_id),
                                error = %e,
                                "commit rejected"
                            );
                        }
                    }
                } else {
                    warn!(
                        request = hex::encode(request_id),
                        node = hex::encode(node_id),
                        "commit for unknown round"
                    );
                }
            }

            // ---- Reveal submission -----------------------------------------
            DiceMessage::RevealSubmission(rs) => {
                let ok = rs.node_id.len() == 33
                    && rs.request_id.len() == 32
                    && rs.entropy.len() == 32
                    && rs.signature.len() == 64;
                if !ok {
                    warn!("reveal submission has wrong field sizes");
                    continue;
                }

                let mut node_id = [0u8; 33];
                let mut request_id = [0u8; 32];
                let mut entropy = [0u8; 32];
                let mut sig = [0u8; 64];
                node_id.copy_from_slice(&rs.node_id);
                request_id.copy_from_slice(&rs.request_id);
                entropy.copy_from_slice(&rs.entropy);
                sig.copy_from_slice(&rs.signature);

                // Process reveal; collect finalization data if round completes.
                let finalized = {
                    let mut map = rounds.lock().await;
                    if let Some(entry) = map.get_mut(&request_id) {
                        let entry_requester = entry.requester;
                        let entry_sequence = entry.sequence;
                        let entry_channel_auth = entry.channel_authority;
                        let entry_channel_idx = entry.channel_index;
                        match entry.round.handle_reveal(node_id, entropy, sig) {
                            Ok(Some(randomness)) => {
                                let selected = entry.round.selected_nodes.clone();
                                let onchain_data = entry
                                    .round
                                    .finalized_onchain_data()
                                    .cloned()
                                    .unwrap_or_default();
                                let db_id = entry.db_id;
                                let duration = entry.started_at.elapsed();
                                info!(
                                    request = hex::encode(request_id),
                                    randomness = hex::encode(randomness),
                                    elapsed_ms = duration.as_millis(),
                                    "round finalized!"
                                );
                                metrics.rounds_total.inc();
                                metrics.round_duration_seconds.observe(duration.as_secs_f64());
                                Some((randomness, selected, db_id, entry_requester, entry_sequence, entry_channel_auth, entry_channel_idx, onchain_data))
                            }
                            Ok(None) => {
                                info!(
                                    request = hex::encode(request_id),
                                    node = hex::encode(node_id),
                                    "reveal accepted, waiting for more"
                                );
                                None
                            }
                            Err(e) => {
                                warn!(
                                    request = hex::encode(request_id),
                                    node = hex::encode(node_id),
                                    error = %e,
                                    "reveal rejected"
                                );
                                None
                            }
                        }
                    } else {
                        warn!(
                            request = hex::encode(request_id),
                            node = hex::encode(node_id),
                            "reveal for unknown round"
                        );
                        None
                    }
                };

                if let Some((randomness, selected_nodes, db_id, req_pubkey, req_seq, channel_auth, channel_idx, onchain_data)) = finalized {
                    // Persist to DB if available.
                    if let Some(ref pool) = db {
                        let _ = db::queries::record_reveal(pool, db_id, &node_id, &entropy).await;
                        let _ = db::queries::finalize_round(pool, db_id, &randomness).await;
                    }

                    // ON-CHAIN SUBMISSION SPLIT INTO 3 TXs (Solana TX size cap is
                    // 1232 bytes; bundling all of commit×N + reveal×N + finalize +
                    // claim_rewards_v2 with N=4 nodes overflows at ~2.1KB):
                    //   TX A: N × submit_commit_v2  → channel: Pending → CommitPhase
                    //   TX B: N × submit_reveal_v2  → channel: CommitPhase → RevealPhase
                    //   TX C: finalize_v2 + claim_rewards_v2 → Finalized + payouts
                    if let Some(ref ctx) = on_chain {
                        if req_pubkey != solana_sdk::pubkey::Pubkey::default() {
                            if let (Some(auth), Some(idx)) = (channel_auth, channel_idx) {
                                // Reorder onchain_data into selected_nodes order so
                                // the on-chain channel.device_pubkeys[] vector ends
                                // up in the SAME order we'll pass NodeVault accounts
                                // to claim_rewards_v2 — otherwise the per-vault
                                // ownership check fails (claim_rewards_v2.rs:146).
                                let ordered: Vec<&([u8;33],[u8;32],[u8;64],[u8;32],[u8;64])> = selected_nodes
                                    .iter()
                                    .filter_map(|n| onchain_data.iter().find(|(node, ..)| node == n))
                                    .collect();
                                if ordered.len() != onchain_data.len() {
                                    warn!(
                                        ordered = ordered.len(),
                                        onchain = onchain_data.len(),
                                        "onchain_data has nodes not in selected_nodes — payout will be misordered"
                                    );
                                }

                                // ── ONE TX: submit_round_v2 + claim ──────────
                                //
                                // v7.5 (single-shot): all device commits +
                                // reveals in one ix (`submit_round_v2`),
                                // followed by `claim_rewards_v2` in the same
                                // TX. No prior commits TX — atomically writes
                                // commits, reveals, computed randomness, and
                                // auto-Idle's the channel.
                                //
                                // Threat model: see submit_round_v2.rs — the
                                // coordinator can grind by silently dropping
                                // a TX it doesn't like, but bias resistance
                                // still holds for any honest contributor.
                                // Acceptable because the coordinator is
                                // operator-controlled + observable.
                                //
                                // Size budget for 4 nodes (legacy TX, no ALT):
                                //   submit_round_v2 (664 B data + 6 ovh)  = 670 B
                                //   claim_rewards_v2 (8 accts inline)     =  19 B
                                //   compute_budget (set_cu_price)          =  12 B
                                //   static keys (10 × 32)                  = 320 B
                                //   sig + header + blockhash               = 100 B
                                //   total                                  ≈1123 B (< 1232)
                                //
                                // For 5+ nodes we still fit (~1300 B); for 7+
                                // we'd need ALT. Production = 4 nodes.
                                let default_pk = solana_sdk::pubkey::Pubkey::default();
                                let payouts_enabled = ctx.treasury != default_pk
                                    && ctx.reserve != default_pk
                                    && !selected_nodes.is_empty();

                                let v75_contribs: Vec<solana_tx::V75RoundContribution> = ordered
                                    .iter()
                                    .copied()
                                    .map(|(node_pk, commit_hash, _commit_sig, entropy, reveal_sig)| {
                                        solana_tx::V75RoundContribution {
                                            device_pubkey: *node_pk,
                                            commit_hash: *commit_hash,
                                            entropy: *entropy,
                                            signature: *reveal_sig,
                                        }
                                    })
                                    .collect();

                                let mut instructions = Vec::with_capacity(2);
                                instructions.push(solana_tx::build_submit_round_v2_ix(
                                    &ctx.program_id,
                                    &ctx.keypair.pubkey(),
                                    &auth,
                                    idx,
                                    req_seq,
                                    &v75_contribs,
                                ));
                                if payouts_enabled {
                                    let claim_nodes: Vec<[u8;33]> = ordered.iter().map(|(n, ..)| *n).collect();
                                    instructions.push(solana_tx::build_claim_rewards_v2_ix(
                                        &ctx.program_id,
                                        &ctx.keypair.pubkey(),
                                        &auth,
                                        idx,
                                        &ctx.treasury,
                                        &ctx.reserve,
                                        &claim_nodes,
                                    ));
                                }

                                match ctx.rpc.sign_send_and_confirm(&ctx.keypair, instructions).await {
                                    Ok(s) => info!(
                                        sig = %s,
                                        payouts_enabled,
                                        num_nodes = selected_nodes.len(),
                                        "v7.5 submit_round_v2 + claim TX confirmed (single-shot)"
                                    ),
                                    Err(e) => {
                                        warn!(error = %e, "v2 reveals+finalize+claim TX failed");
                                        continue;
                                    }
                                }
                            } else {
                                // v1.0 legacy flow — BUNDLE commit + reveal + finalize in ONE TX
                                let mut instructions = Vec::new();

                                // For each node that participated, add commit + reveal instructions
                                // We have node_id and entropy from the current reveal,
                                // and commit_hash was verified during the round.
                                // For single-node rounds, this is straightforward:
                                let device_id = solana_tx::compute_device_id(&node_id);
                                let commit_hash_for_chain = {
                                    // SHA-256(entropy) = the commit hash
                                    use sha2::{Sha256, Digest};
                                    let hash: [u8; 32] = Sha256::digest(&entropy).into();
                                    hash
                                };

                                instructions.push(solana_tx::build_submit_commit_ix(
                                    &ctx.program_id,
                                    &ctx.keypair.pubkey(),
                                    &req_pubkey,
                                    req_seq,
                                    &node_id,
                                    &commit_hash_for_chain,
                                ));

                                instructions.push(solana_tx::build_submit_reveal_ix(
                                    &ctx.program_id,
                                    &ctx.keypair.pubkey(),
                                    &req_pubkey,
                                    req_seq,
                                    &node_id,
                                    &entropy,
                                    &sig,
                                ));

                                instructions.push(solana_tx::build_finalize_randomness_ix(
                                    &ctx.program_id,
                                    &ctx.keypair.pubkey(),
                                    &req_pubkey,
                                    req_seq,
                                ));

                                match ctx.rpc.sign_and_send(&ctx.keypair, instructions).await {
                                    Ok(s) => info!(sig = %s, "BUNDLED TX sent (commit+reveal+finalize)"),
                                    Err(e) => warn!(error = %e, "BUNDLED TX failed"),
                                }
                            }
                        }
                    }

                    // Broadcast RoundResult to all selected nodes.
                    let result_msg = DiceMessage::RoundResult(RoundResult {
                        request_id: request_id.to_vec(),
                        status: "finalized".to_string(),
                        randomness: randomness.to_vec(),
                    });
                    if let Ok(encoded) = result_msg.encode() {
                        let reg = registry.read().await;
                        for nid in &selected_nodes {
                            if let Some(session) = reg.get(nid) {
                                let _ = session.tx.try_send(encoded.clone());
                            }
                        }
                    }

                    // Save to round history for dashboard, then remove from active map.
                    let elapsed_ms = {
                        let map = rounds.lock().await;
                        map.get(&request_id)
                            .map(|e| e.started_at.elapsed().as_millis() as u64)
                            .unwrap_or(0)
                    };
                    {
                        use crate::api::routes::CompletedRound;
                        let mut hist = round_history.lock().await;
                        let ts = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs();
                        hist.push(CompletedRound {
                            request_id: hex::encode(request_id),
                            randomness: hex::encode(randomness),
                            node_count: selected_nodes.len(),
                            elapsed_ms,
                            timestamp: ts,
                            status: "finalized".to_string(),
                        });
                        let hlen = hist.len();
                        if hlen > 100 { hist.drain(..hlen - 100); }
                    }
                    {
                        let mut map = rounds.lock().await;
                        map.remove(&request_id);
                    }

                    // Drain queue: dispatch pending requests to nodes that just freed up.
                    for nid in &selected_nodes {
                        let queued = {
                            let mut q = request_queue.lock().await;
                            q.mark_completed(nid)
                        };
                        for qr in queued {
                            info!(
                                request = hex::encode(qr.request_id),
                                node = hex::encode(nid),
                                "dispatching queued request"
                            );
                            let qr_round = crate::state_machine::Round::new(
                                qr.request_id,
                                vec![*nid],
                                1,
                                std::time::Duration::from_secs(60),
                            );
                            {
                                let mut map = rounds.lock().await;
                                map.insert(qr.request_id, crate::state_machine::RoundEntry {
                                    round: qr_round,
                                    db_id: qr.db_id,
                                    started_at: std::time::Instant::now(),
                                    requester: qr.requester,
                                    sequence: qr.sequence,
                                    channel_authority: None,
                                    channel_index: None,
                                });
                            }
                            let job = DiceMessage::JobAssignment(protocol::messages::JobAssignment {
                                request_id: qr.request_id.to_vec(),
                                round_seq: qr.sequence,
                                deadline_ts: qr.deadline_ts,
                            });
                            if let Ok(encoded) = job.encode() {
                                let reg = registry.read().await;
                                if let Some(session) = reg.get(nid) {
                                    let _ = session.tx.try_send(encoded);
                                }
                            }
                        }
                    }
                }
            }

            // Device requests to bind a payout wallet. Handled by submitting
            // register_node_vault on-chain. The signature is verified by the
            // Anchor instruction itself — we just forward the bytes.
            DiceMessage::PayoutBindingRequest(p) => {
                if let Some(ref ctx) = on_chain {
                    // Validate lengths before building the TX so we bail
                    // early with a clear log line instead of failing in the
                    // builder.
                    if p.node_id.len() != 33 {
                        warn!(
                            len = p.node_id.len(),
                            "PayoutBindingRequest: node_id must be 33 bytes — dropping"
                        );
                        continue;
                    }
                    if p.payout_wallet.len() != 32 {
                        warn!(
                            len = p.payout_wallet.len(),
                            "PayoutBindingRequest: payout_wallet must be 32 bytes — dropping"
                        );
                        continue;
                    }
                    if p.nonce.len() != 32 {
                        warn!(
                            len = p.nonce.len(),
                            "PayoutBindingRequest: nonce must be 32 bytes — dropping"
                        );
                        continue;
                    }
                    if p.signature.len() != 64 {
                        warn!(
                            len = p.signature.len(),
                            "PayoutBindingRequest: signature must be 64 bytes — dropping"
                        );
                        continue;
                    }

                    let mut device_pubkey = [0u8; 33];
                    device_pubkey.copy_from_slice(&p.node_id);

                    let payout_wallet = solana_sdk::pubkey::Pubkey::new_from_array(
                        p.payout_wallet.as_slice().try_into().unwrap(),
                    );

                    let mut nonce = [0u8; 32];
                    nonce.copy_from_slice(&p.nonce);

                    let mut signature = [0u8; 64];
                    signature.copy_from_slice(&p.signature);

                    let ix = solana_tx::build_register_node_vault_ix(
                        &ctx.program_id,
                        &solana_sdk::signer::Signer::pubkey(ctx.keypair.as_ref()),
                        &device_pubkey,
                        &payout_wallet,
                        p.timestamp,
                        &nonce,
                        &signature,
                    );

                    match ctx.rpc.sign_and_send(&ctx.keypair, vec![ix]).await {
                        Ok(sig) => info!(
                            sig = %sig,
                            device = hex::encode(&device_pubkey[..4]),
                            wallet = %payout_wallet,
                            "register_node_vault TX sent"
                        ),
                        Err(e) => warn!(
                            error = %e,
                            device = hex::encode(&device_pubkey[..4]),
                            "register_node_vault TX failed"
                        ),
                    }
                } else {
                    warn!("PayoutBindingRequest received but on-chain submission is disabled");
                }
            }

            // Nodes should not send these.
            DiceMessage::JobAssignment(_) | DiceMessage::RoundResult(_) => {
                warn!("received unexpected outbound message type from node");
            }
        }
    }

    // Cleanup.
    write_task.abort();
    if let Some(id) = node_id_opt {
        deregister(&registry, &id).await;
        metrics.nodes_connected.dec();
        // Clear queue tracking for this node.
        {
            let mut q = request_queue.lock().await;
            q.node_disconnected(&id);
        }
        info!(node = hex::encode(id), "node disconnected");
    }
}

// ---------------------------------------------------------------------------
// Round timeout watchdog
// ---------------------------------------------------------------------------

/// Background task that periodically checks all active rounds for timeouts.
/// Timed-out rounds are marked as failed, a `RoundResult` with status
/// "failed" is broadcast to all selected nodes, and the round is removed.
async fn round_timeout_watchdog(
    rounds: RoundMap,
    metrics: Metrics,
    registry: NodeRegistry,
) {
    use std::time::Duration;

    let mut interval = tokio::time::interval(Duration::from_secs(5));

    loop {
        interval.tick().await;

        let timed_out_rounds: Vec<([u8; 32], Vec<[u8; 33]>)> = {
            let mut map = rounds.lock().await;
            let mut expired = Vec::new();
            for (request_id, entry) in map.iter_mut() {
                if entry.round.check_timeout() {
                    metrics.rounds_failed_total.inc();
                    let selected = entry.round.selected_nodes.clone();
                    expired.push((*request_id, selected));
                }
            }
            // Remove timed-out rounds to prevent unbounded memory growth
            for (request_id, _) in &expired {
                map.remove(request_id);
            }
            expired
        };

        // Broadcast failure results to nodes outside the lock.
        for (request_id, selected_nodes) in timed_out_rounds {
            warn!(
                request = hex::encode(request_id),
                "round timed out — removed from map, broadcasting failure"
            );
            let result_msg = DiceMessage::RoundResult(RoundResult {
                request_id: request_id.to_vec(),
                status: "failed".to_string(),
                randomness: vec![],
            });
            if let Ok(encoded) = result_msg.encode() {
                let reg = registry.read().await;
                for nid in &selected_nodes {
                    if let Some(session) = reg.get(nid) {
                        let _ = session.tx.try_send(encoded.clone());
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Database migration
// ---------------------------------------------------------------------------

async fn run_migrations(pool: &sqlx::PgPool) -> Result<()> {
    let statements = [
        r#"CREATE TABLE IF NOT EXISTS nodes (
            node_id        BYTEA PRIMARY KEY,
            registered_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            last_seen      TIMESTAMPTZ,
            latency_ms     INTEGER,
            uptime_secs    BIGINT,
            jobs_completed BIGINT DEFAULT 0,
            is_active      BOOLEAN DEFAULT TRUE
        )"#,
        r#"CREATE TABLE IF NOT EXISTS rounds (
            id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            request_id     BYTEA NOT NULL,
            status         TEXT NOT NULL,
            selected_nodes BYTEA[],
            randomness     BYTEA,
            created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            finalized_at   TIMESTAMPTZ
        )"#,
        r#"CREATE TABLE IF NOT EXISTS commits (
            round_id     UUID REFERENCES rounds(id),
            node_id      BYTEA NOT NULL,
            commit_hash  BYTEA NOT NULL,
            submitted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            PRIMARY KEY (round_id, node_id)
        )"#,
        r#"CREATE TABLE IF NOT EXISTS reveals (
            round_id     UUID REFERENCES rounds(id),
            node_id      BYTEA NOT NULL,
            entropy      BYTEA NOT NULL,
            submitted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            PRIMARY KEY (round_id, node_id)
        )"#,
        r#"CREATE TABLE IF NOT EXISTS audit_log (
            id          BIGSERIAL PRIMARY KEY,
            event_type  TEXT NOT NULL,
            payload     JSONB,
            created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"#,
    ];

    for stmt in &statements {
        sqlx::query(stmt)
            .execute(pool)
            .await
            .context("run schema migration statement")?;
    }

    info!("database schema applied");
    Ok(())
}
