// DICE Coordinator — entry point
// Wires together: config, DB, WebSocket server (mTLS or plain), REST API, metrics.

mod api;
mod config;
mod db;
mod metrics;
mod node_session;
mod protocol;
mod selection;
mod solana_rpc;
mod solana_tx;
mod solana_watcher;
mod solana_ws;
mod state_machine;

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

    // 3. Connect to PostgreSQL (skipped in simulation mode).
    let pool: Option<sqlx::PgPool> = if cfg.simulation {
        None
    } else {
        let p = sqlx::PgPool::connect(&cfg.database_url)
            .await
            .context("connect to PostgreSQL")?;
        run_migrations(&p).await?;
        Some(p)
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
                info!(
                    coordinator = %solana_sdk::signer::Signer::pubkey(&keypair),
                    program = %program_id,
                    rpc = %cfg.solana_rpc_url,
                    "on-chain transactions ENABLED"
                );
                Some(OnChainCtx {
                    rpc: Arc::new(solana_rpc::SolanaRpc::new(&cfg.solana_rpc_url)),
                    keypair: Arc::new(keypair),
                    program_id,
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

    // 5. Spawn Axum REST API server.
    let api_state = AppState {
        registry: registry.clone(),
        metrics: metrics.clone(),
        db: pool.clone(),
        rounds: rounds.clone(),
        on_chain: on_chain.clone(),
    };
    let api_handle = {
        let port = cfg.api_port;
        let router = build_router(api_state);
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

    // 7. Spawn WebSocket server: plain for simulation, mTLS for production.
    let ws_handle = if cfg.simulation {
        let reg = registry.clone();
        let m = metrics.clone();
        let r = rounds.clone();
        let port = cfg.ws_port;
        let db_opt = pool.clone();
        let oc = on_chain.clone();
        tokio::spawn(async move {
            if let Err(e) = serve_websocket_plain(reg, r, m, db_opt, oc, port).await {
                error!(?e, "plain WebSocket server error");
            }
        })
    } else {
        let reg = registry.clone();
        let m = metrics.clone();
        let r = rounds.clone();
        let port = cfg.ws_port;
        let tls_cert = cfg.tls_cert_path.clone();
        let tls_key = cfg.tls_key_path.clone();
        let ca_cert = cfg.ca_cert_path.clone();
        let db_opt = pool.clone();
        let oc = on_chain.clone();
        tokio::spawn(async move {
            if let Err(e) = serve_websocket_mtls(reg, r, m, db_opt, oc, port, tls_cert, tls_key, ca_cert).await {
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

    // 9. Spawn Solana WebSocket subscriber (production mode only — real-time log events).
    //    Replaces the old 5-second polling watcher with ~500ms latency via logsSubscribe.
    let watcher_handle = if !cfg.simulation {
        let rpc = std::sync::Arc::new(solana_rpc::SolanaRpc::new(&cfg.solana_rpc_url));
        let keypair = std::sync::Arc::new(
            solana_rpc::load_keypair(&cfg.coordinator_keypair_path)
                .expect("load coordinator keypair"),
        );
        let program_id: solana_sdk::pubkey::Pubkey = "78Qv6cyKkRZN2YngiLSSBCe2iyRc6jgtCs3incCaMRcv"
            .parse()
            .expect("parse program ID");
        let rpc_url = cfg.solana_rpc_url.clone();
        let reg = registry.clone();
        let r = rounds.clone();
        let m = metrics.clone();
        let min = cfg.min_nodes as usize;
        let max = cfg.max_nodes as usize;
        let timeout = std::time::Duration::from_secs(cfg.commit_timeout_secs);
        Some(tokio::spawn(async move {
            solana_ws::run_solana_ws_subscriber(
                rpc, rpc_url, program_id, keypair, reg, r, m, min, max, timeout,
            )
            .await;
        }))
    } else {
        info!("Solana WebSocket subscriber disabled in simulation mode");
        None
    };

    // 10. Ready banner.
    println!("DICE Coordinator ready:");
    println!("  Dashboard : http://localhost:{}/", cfg.api_port);
    println!("  WebSocket : {}://localhost:{}/", if cfg.simulation { "ws" } else { "wss" }, cfg.ws_port);
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
        let metrics = metrics.clone();
        let db = db.clone();
        let oc = on_chain.clone();

        tokio::spawn(async move {
            match accept_async(tcp_stream).await {
                Ok(ws) => {
                    handle_node_connection(ws, registry, rounds, metrics, db, oc).await;
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
        let metrics = metrics.clone();
        let db = db.clone();
        let oc = on_chain.clone();

        tokio::spawn(async move {
            match tls_acceptor.accept(tcp_stream).await {
                Ok(tls_stream) => {
                    info!(%peer_addr, "mTLS handshake success");
                    match accept_async_with_config(tls_stream, None).await {
                        Ok(ws) => {
                            handle_node_connection(ws, registry, rounds, metrics, db, oc).await;
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
                            // Submit commit on-chain (v2.0 channel or v1.0 legacy).
                            if let Some(ref ctx) = on_chain {
                                if entry_requester != solana_sdk::pubkey::Pubkey::default() {
                                    let ix = if let (Some(auth), Some(idx)) = (entry.channel_authority, entry.channel_index) {
                                        // v2.0 channel flow
                                        solana_tx::build_submit_commit_v2_ix(
                                            &ctx.program_id,
                                            &ctx.keypair.pubkey(),
                                            &auth,
                                            idx,
                                            entry_sequence,
                                            &node_id,
                                            &commit_hash,
                                        )
                                    } else {
                                        // v1.0 legacy flow
                                        solana_tx::build_submit_commit_ix(
                                            &ctx.program_id,
                                            &ctx.keypair.pubkey(),
                                            &entry_requester,
                                            entry_sequence,
                                            &node_id,
                                            &commit_hash,
                                        )
                                    };
                                    match ctx.rpc.sign_and_send(&ctx.keypair, vec![ix]).await {
                                        Ok(s) => info!(sig = %s, "submit_commit TX sent"),
                                        Err(e) => warn!(error = %e, "submit_commit TX failed"),
                                    }
                                }
                            }
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
                                Some((randomness, selected, db_id, entry_requester, entry_sequence, entry_channel_auth, entry_channel_idx))
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

                if let Some((randomness, selected_nodes, db_id, req_pubkey, req_seq, channel_auth, channel_idx)) = finalized {
                    // Persist to DB if available.
                    if let Some(ref pool) = db {
                        let _ = db::queries::record_reveal(pool, db_id, &node_id, &entropy).await;
                        let _ = db::queries::finalize_round(pool, db_id, &randomness).await;
                    }

                    // Submit finalize on-chain (v2.0 channel or v1.0 legacy).
                    if let Some(ref ctx) = on_chain {
                        if req_pubkey != solana_sdk::pubkey::Pubkey::default() {
                            if let (Some(auth), Some(idx)) = (channel_auth, channel_idx) {
                                // v2.0: finalize_v2 + deliver_callback
                                let ix_finalize = solana_tx::build_finalize_v2_ix(
                                    &ctx.program_id,
                                    &ctx.keypair.pubkey(),
                                    &auth,
                                    idx,
                                    req_seq,
                                );
                                match ctx.rpc.sign_and_send(&ctx.keypair, vec![ix_finalize]).await {
                                    Ok(s) => {
                                        info!(sig = %s, "finalize_v2 TX sent");
                                        // Follow up with deliver_callback
                                        let ix_callback = solana_tx::build_deliver_callback_ix(
                                            &ctx.program_id,
                                            &ctx.keypair.pubkey(),
                                            &auth,
                                            idx,
                                            req_seq,
                                            vec![],
                                        );
                                        match ctx.rpc.sign_and_send(&ctx.keypair, vec![ix_callback]).await {
                                            Ok(s2) => info!(sig = %s2, "deliver_callback TX sent"),
                                            Err(e) => warn!(error = %e, "deliver_callback TX failed (randomness still saved)"),
                                        }
                                    }
                                    Err(e) => warn!(error = %e, "finalize_v2 TX failed"),
                                }
                            } else {
                                // v1.0 legacy flow
                                let ix = solana_tx::build_finalize_randomness_ix(
                                    &ctx.program_id,
                                    &ctx.keypair.pubkey(),
                                    &req_pubkey,
                                    req_seq,
                                );
                                match ctx.rpc.sign_and_send(&ctx.keypair, vec![ix]).await {
                                    Ok(s) => info!(sig = %s, "finalize_randomness TX sent"),
                                    Err(e) => warn!(error = %e, "finalize_randomness TX failed"),
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

                    // Remove finalized round from map to prevent memory leak.
                    {
                        let mut map = rounds.lock().await;
                        map.remove(&request_id);
                    }
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
