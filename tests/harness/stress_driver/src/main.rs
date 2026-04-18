//! stress-driver — v2 channel load tester.
//!
//! One binary that runs N sequential `request_randomness_auto` rounds
//! on a single DiceChannel and records latency, success rate, and any
//! round that wedged or timed out. Used for categories A, J of
//! `test_v7.md`.
//!
//! No dApp involvement — this just exercises the DICE v2 channel path
//! so we can measure coordinator throughput under real devnet hardware.
//!
//! Usage:
//!   stress-driver --channel-index 100 --rounds 10
//!   stress-driver --channel-index 103 --rounds 1000 --node-count 4 --prefund-sol 2.2
//!
//! Channel is reusable: a second run with the same --channel-index will
//! keep going from wherever the previous run left off (including an Idle
//! state left by our deliver_callback no-op transition).

use anyhow::{anyhow, Context, Result};
use base64::Engine;
use clap::Parser;
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use solana_sdk::{
    hash::Hash,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
    system_program,
    transaction::Transaction,
};
use std::str::FromStr;
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::watch;
use tokio_tungstenite::{connect_async, tungstenite::Message};

const DICE_PROGRAM_ID: &str = "FMwPuCjkfZXN2MuNJQiUzZC3hnxHcD8mrTuntsqA84XD";
const REQUEST_FEE_LAMPORTS: u64 = 2_000_000;

#[derive(Parser)]
#[command(about = "v2 channel load tester")]
struct Args {
    #[arg(long, default_value = "https://api.devnet.solana.com")]
    rpc_url: String,

    #[arg(long, default_value = "coordinator-keypair.json")]
    keypair: std::path::PathBuf,

    /// DiceChannel index for this run. Reused across runs.
    #[arg(long)]
    channel_index: u16,

    /// Number of rounds to run.
    #[arg(long, default_value_t = 10)]
    rounds: u32,

    /// Node count requested per round (4..=max_nodes).
    #[arg(long, default_value_t = 4)]
    node_count: u8,

    /// Max nodes slot for the channel. Only used if channel doesn't exist.
    #[arg(long, default_value_t = 7)]
    max_nodes: u8,

    /// Pre-fund the channel with this much SOL before the loop. Skip if 0.
    #[arg(long, default_value_t = 0.0)]
    prefund_sol: f64,

    /// Per-round timeout (seconds) waiting for Finalized.
    #[arg(long, default_value_t = 60)]
    round_timeout_secs: u64,

    /// Sleep between rounds (milliseconds).
    #[arg(long, default_value_t = 0)]
    sleep_between_ms: u64,

    /// Write JSON results to this file (one record per round + a summary).
    #[arg(long)]
    out: Option<std::path::PathBuf>,

    /// Label for the run — goes into the output file.
    #[arg(long, default_value = "stress")]
    label: String,

    /// Pass `SlotHashes` + all registered `DeviceRegistry` PDAs as remaining
    /// accounts to `request_randomness_auto` so the program runs on-chain
    /// Fisher-Yates selection (v7.3+). Trades ~250-750 ms avg latency for a
    /// verifiable, coord-independent selection.
    #[arg(long, default_value_t = false)]
    on_chain_select: bool,

    /// Exclude devices whose on-chain `device_pubkey` starts with any of these
    /// hex prefixes (comma-separated, e.g. `025e62666100`). Used to keep the
    /// on-chain Fisher-Yates pool restricted to nodes known to be online.
    /// Only meaningful with --on-chain-select.
    #[arg(long, value_delimiter = ',')]
    exclude_device_prefix: Vec<String>,
}

#[derive(serde::Serialize)]
struct RoundResult {
    round_index: u32,
    round_id: u64,
    status: String,
    duration_ms: u128,
    request_tx: Option<String>,
    randomness_prefix: Option<String>,
    error: Option<String>,
}

#[derive(serde::Serialize)]
struct StressSummary {
    label: String,
    channel_index: u16,
    channel_pda: String,
    node_count: u8,
    total_rounds: u32,
    succeeded: u32,
    failed: u32,
    total_duration_ms: u128,
    avg_ms: u128,
    p50_ms: u128,
    p95_ms: u128,
    p99_ms: u128,
    min_ms: u128,
    max_ms: u128,
    start_balance_sol: f64,
    end_balance_sol: f64,
    spent_sol: f64,
    rounds: Vec<RoundResult>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new(
            "info,stress_driver=debug",
        ))
        .init();

    let args = Args::parse();

    let dice_pid = Pubkey::from_str(DICE_PROGRAM_ID)?;
    let payer = load_keypair(&args.keypair)?;
    let player = payer.pubkey();
    let rpc = Rpc::new(args.rpc_url.clone());

    let start_balance = rpc.get_balance(&player).await?;
    println!("stress-driver v1");
    println!("  payer: {}", player);
    println!("  start balance: {:.6} SOL", start_balance as f64 / 1e9);
    println!("  channel_index: {}, rounds: {}, node_count: {}", args.channel_index, args.rounds, args.node_count);

    let (channel_pda, _) = Pubkey::find_program_address(
        &[b"channel", player.as_ref(), &args.channel_index.to_le_bytes()],
        &dice_pid,
    );
    println!("  channel PDA: {}", channel_pda);

    // init_channel if needed
    if rpc.get_account_data(&channel_pda).await?.is_none() {
        println!("  init_channel (max_nodes={})", args.max_nodes);
        let ix = build_init_channel_ix(
            &dice_pid,
            &player,
            &channel_pda,
            args.channel_index,
            args.max_nodes,
            &Pubkey::default(),
            &player,
        );
        let sig = rpc.sign_and_send(&payer, vec![ix]).await?;
        wait_confirmed(&rpc, &sig).await?;
        println!("    TX: {}", sig);
    }

    // Pre-fund if requested
    if args.prefund_sol > 0.0 {
        let lamports = (args.prefund_sol * 1e9) as u64;
        println!("  pre-funding channel with {:.4} SOL ({} lamports)", args.prefund_sol, lamports);
        let ix = build_fund_channel_ix(&dice_pid, &player, &channel_pda, lamports);
        let sig = rpc.sign_and_send(&payer, vec![ix]).await?;
        wait_confirmed(&rpc, &sig).await?;
        println!("    TX: {}", sig);
    }

    // Check channel balance vs required fee
    let required_lamports = (args.rounds as u64) * REQUEST_FEE_LAMPORTS;
    println!("  required channel balance for {} rounds: {} lamports", args.rounds, required_lamports);

    // Drain stale Finalized state if present
    drain_to_idle(&rpc, &payer, &dice_pid, &channel_pda).await?;

    // Open a persistent WebSocket accountSubscribe on the channel PDA. Every
    // on-chain state mutation pushes (status, round_id, randomness) through
    // the `channel_state_rx` watch, so `run_one_round` no longer has to poll
    // via getAccountInfo. Replaces the 250 ms RPC poll loop → instant Idle
    // detection. Reconnects automatically if the WS drops.
    let ws_url = http_to_ws(&args.rpc_url);
    let (channel_state_tx, channel_state_rx) =
        watch::channel::<(u8, u64, [u8; 32])>((u8::MAX, 0, [0u8; 32]));
    let ws_channel_pda = channel_pda;
    tokio::spawn(async move {
        channel_ws_subscriber(ws_url, ws_channel_pda, channel_state_tx).await;
    });

    // Enumerate registered DeviceRegistry PDAs up-front if on-chain
    // selection is requested. These are passed as remaining_accounts on
    // every `request_randomness_auto` so the program can run on-chain
    // Fisher-Yates. One-time cost at startup.
    let device_registries: Vec<Pubkey> = if args.on_chain_select {
        let excluded_prefixes: Vec<Vec<u8>> = args
            .exclude_device_prefix
            .iter()
            .filter_map(|s| hex::decode(s.trim()).ok())
            .collect();
        match rpc.list_device_registries(&dice_pid).await {
            Ok(pairs_all) => {
                let total = pairs_all.len();
                let v: Vec<Pubkey> = pairs_all
                    .into_iter()
                    .filter(|(_pda, device_pk)| {
                        !excluded_prefixes
                            .iter()
                            .any(|pre| device_pk.starts_with(pre))
                    })
                    .map(|(pda, _)| pda)
                    .collect();
                let excluded_count = total - v.len();
                if v.len() >= args.node_count as usize {
                    println!(
                        "  on-chain selection: {} DeviceRegistry PDAs usable ({} excluded by prefix)",
                        v.len(),
                        excluded_count
                    );
                    v
                } else {
                    anyhow::bail!(
                        "--on-chain-select: only {} usable DeviceRegistry PDAs after exclusions, need >= {}",
                        v.len(),
                        args.node_count
                    );
                }
            }
            Err(e) => anyhow::bail!("--on-chain-select: enumeration failed: {}", e),
        }
    } else {
        Vec::new()
    };

    println!();
    println!("── running {} rounds ─────────────────────────────", args.rounds);
    let overall_start = Instant::now();

    let mut results: Vec<RoundResult> = Vec::with_capacity(args.rounds as usize);
    let mut durations_ms: Vec<u128> = Vec::with_capacity(args.rounds as usize);
    let mut succeeded = 0u32;
    let mut failed = 0u32;

    for i in 0..args.rounds {
        let round_start = Instant::now();
        match run_one_round(
            &rpc,
            &payer,
            &dice_pid,
            &channel_pda,
            args.node_count,
            args.round_timeout_secs,
            channel_state_rx.clone(),
            &device_registries,
        )
        .await
        {
            Ok((round_id, req_sig, randomness)) => {
                let ms = round_start.elapsed().as_millis();
                durations_ms.push(ms);
                succeeded += 1;
                let rand_prefix = hex::encode(&randomness[..8]);
                println!(
                    "[{:>4}/{}] round_id={} {:>6}ms ✓ rand={}...",
                    i + 1,
                    args.rounds,
                    round_id,
                    ms,
                    rand_prefix
                );
                results.push(RoundResult {
                    round_index: i,
                    round_id,
                    status: "ok".to_string(),
                    duration_ms: ms,
                    request_tx: Some(req_sig.to_string()),
                    randomness_prefix: Some(rand_prefix),
                    error: None,
                });
            }
            Err(e) => {
                let ms = round_start.elapsed().as_millis();
                failed += 1;
                eprintln!("[{:>4}/{}] FAIL in {}ms — {}", i + 1, args.rounds, ms, e);
                results.push(RoundResult {
                    round_index: i,
                    round_id: 0,
                    status: "fail".to_string(),
                    duration_ms: ms,
                    request_tx: None,
                    randomness_prefix: None,
                    error: Some(e.to_string()),
                });
                // If a round wedges, attempt to drain it to idle before
                // proceeding so the next request doesn't auto-fail with
                // RoundNotComplete.
                let _ = drain_to_idle(&rpc, &payer, &dice_pid, &channel_pda).await;
            }
        }

        if args.sleep_between_ms > 0 {
            tokio::time::sleep(Duration::from_millis(args.sleep_between_ms)).await;
        }
    }

    let total_ms = overall_start.elapsed().as_millis();
    let end_balance = rpc.get_balance(&player).await.unwrap_or(0);
    let (p50, p95, p99, min, max, avg) = latency_stats(&durations_ms);

    println!();
    println!("── summary ───────────────────────────────────────");
    println!("  total time:   {} ms ({:.1} s)", total_ms, total_ms as f64 / 1000.0);
    println!("  succeeded:    {}/{}", succeeded, args.rounds);
    println!("  failed:       {}", failed);
    println!("  avg round:    {} ms", avg);
    println!("  p50 / p95 / p99 / min / max: {} / {} / {} / {} / {}", p50, p95, p99, min, max);
    println!("  start bal:    {:.6} SOL", start_balance as f64 / 1e9);
    println!("  end bal:      {:.6} SOL", end_balance as f64 / 1e9);
    println!("  spent:        {:.6} SOL", (start_balance.saturating_sub(end_balance)) as f64 / 1e9);

    if let Some(out_path) = args.out {
        let summary = StressSummary {
            label: args.label,
            channel_index: args.channel_index,
            channel_pda: channel_pda.to_string(),
            node_count: args.node_count,
            total_rounds: args.rounds,
            succeeded,
            failed,
            total_duration_ms: total_ms,
            avg_ms: avg,
            p50_ms: p50,
            p95_ms: p95,
            p99_ms: p99,
            min_ms: min,
            max_ms: max,
            start_balance_sol: start_balance as f64 / 1e9,
            end_balance_sol: end_balance as f64 / 1e9,
            spent_sol: (start_balance.saturating_sub(end_balance)) as f64 / 1e9,
            rounds: results,
        };
        let json = serde_json::to_string_pretty(&summary)?;
        std::fs::write(&out_path, json)?;
        println!("  wrote summary: {}", out_path.display());
    }

    if failed > 0 {
        std::process::exit(1);
    }
    Ok(())
}

fn latency_stats(durations_ms: &[u128]) -> (u128, u128, u128, u128, u128, u128) {
    if durations_ms.is_empty() {
        return (0, 0, 0, 0, 0, 0);
    }
    let mut sorted: Vec<u128> = durations_ms.to_vec();
    sorted.sort();
    let n = sorted.len();
    let p = |pct: f64| -> u128 {
        let idx = ((n as f64 - 1.0) * pct) as usize;
        sorted[idx]
    };
    let min = sorted[0];
    let max = sorted[n - 1];
    let sum: u128 = sorted.iter().sum();
    let avg = sum / n as u128;
    (p(0.50), p(0.95), p(0.99), min, max, avg)
}

/// Run one full round on a reused channel. Uses a shared WebSocket
/// `accountSubscribe` watch receiver (`state_rx`) to detect channel state
/// changes instantly — no per-round RPC polling. The round completes when
/// we observe either:
///   - status == Idle (0) AND round_id == pre_round_id  (v7.4+ auto-Idle)
///   - status == Finalized (4)                          (callback path)
async fn run_one_round(
    rpc: &Rpc,
    payer: &Keypair,
    dice_pid: &Pubkey,
    channel_pda: &Pubkey,
    node_count: u8,
    timeout_secs: u64,
    mut state_rx: watch::Receiver<(u8, u64, [u8; 32])>,
    device_registries: &[Pubkey],
) -> Result<(u64, Signature, [u8; 32])> {
    let player = payer.pubkey();

    let req_ix = build_request_randomness_auto_ix(
        dice_pid, &player, channel_pda, node_count, device_registries,
    );
    let req_sig = rpc.sign_and_send(payer, vec![req_ix]).await?;
    wait_confirmed(rpc, &req_sig).await?;

    // Capture the round_id we just kicked off so we can detect the
    // v7.4 auto-Idle path (where the channel goes Pending → Idle in
    // one TX bundle without ever publishing a Finalized observation).
    let post_req_data = rpc.get_account_data(channel_pda).await?.unwrap_or_default();
    let pre_round_id = if post_req_data.len() >= 84 {
        u64::from_le_bytes(post_req_data[76..84].try_into().unwrap())
    } else { 0 };

    // Mark current watch value as seen so the next `changed().await` only
    // fires on genuinely new state (pushed after request_randomness_auto).
    state_rx.borrow_and_update();

    let deadline = Instant::now() + Duration::from_secs(timeout_secs);
    loop {
        // Prefer WS push; fall back to an RPC poll every 2 s so we don't
        // stall if the WS subscription misses a notification (can happen on
        // reconnect — subscription re-initialises but the state change
        // already fired on the old socket).
        let poll_wait = Duration::from_millis(2_000);
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            anyhow::bail!("round timed out waiting for Finalized/Idle");
        }
        let wait = poll_wait.min(remaining);

        let ws_signal = tokio::time::timeout(wait, state_rx.changed()).await;
        let (status, round_id, randomness) = if ws_signal.is_ok() {
            let r = ws_signal.unwrap();
            if r.is_err() {
                anyhow::bail!("ws state channel closed");
            }
            *state_rx.borrow()
        } else {
            // WS silent for 2 s — do one RPC poll as backstop.
            match rpc.get_account_data(channel_pda).await? {
                Some(data) if data.len() >= 183 => {
                    let st = data[75];
                    let rid = u64::from_le_bytes(data[76..84].try_into().unwrap());
                    let rnd: [u8; 32] = data[151..183].try_into().unwrap();
                    (st, rid, rnd)
                }
                _ => continue,
            }
        };

        if status == 0 && round_id == pre_round_id {
            // v7.4+ auto-Idle path — round done, no deliver_callback needed.
            return Ok((round_id, req_sig, randomness));
        }
        if status == 4 {
            // Pre-v7.4 Finalized — driver still needs to drain via empty
            // deliver_callback for the next request_randomness_auto to work.
            let ix = build_deliver_callback_idle_ix(dice_pid, &player, channel_pda, round_id);
            let sig = rpc.sign_and_send(payer, vec![ix]).await?;
            wait_confirmed(rpc, &sig).await?;
            return Ok((round_id, req_sig, randomness));
        }
        if status == 5 {
            anyhow::bail!("round reached Failed status");
        }
        // Otherwise (Pending/CommitPhase/RevealPhase) — keep waiting.
    }
}

/// Convert an HTTP(S) RPC URL to its WebSocket counterpart.
fn http_to_ws(http_url: &str) -> String {
    if let Some(rest) = http_url.strip_prefix("https://") {
        format!("wss://{}", rest)
    } else if let Some(rest) = http_url.strip_prefix("http://") {
        format!("ws://{}", rest)
    } else {
        http_url.to_string()
    }
}

/// Maintain a persistent Solana WebSocket `accountSubscribe` on the channel
/// PDA. Publishes (status, round_id, randomness) tuples to `tx` on every
/// on-chain state change. Reconnects with a short backoff on disconnect.
///
/// Replaces the driver's per-round `getAccountInfo` polling loop — latency
/// from on-chain state change → driver wake drops from ≤250 ms to the WS
/// push delay (~50 ms on Helius).
async fn channel_ws_subscriber(
    ws_url: String,
    channel_pda: Pubkey,
    tx: watch::Sender<(u8, u64, [u8; 32])>,
) {
    let pda_str = channel_pda.to_string();
    loop {
        match connect_and_subscribe(&ws_url, &pda_str, &tx).await {
            Ok(()) => {
                // Stream ended cleanly (e.g. server close) — reconnect.
                tracing::warn!("channel_ws_subscriber stream ended, reconnecting");
            }
            Err(e) => {
                tracing::warn!(error = %e, "channel_ws_subscriber error, reconnecting");
            }
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

async fn connect_and_subscribe(
    ws_url: &str,
    pda_str: &str,
    tx: &watch::Sender<(u8, u64, [u8; 32])>,
) -> Result<()> {
    let (mut ws, _resp) = connect_async(ws_url).await.context("ws connect")?;
    let sub_req = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "accountSubscribe",
        "params": [pda_str, { "encoding": "base64", "commitment": "confirmed" }]
    });
    ws.send(Message::Text(sub_req.to_string())).await.context("ws send subscribe")?;

    while let Some(msg) = ws.next().await {
        let msg = msg.context("ws recv")?;
        let text = match msg {
            Message::Text(t) => t,
            Message::Binary(b) => String::from_utf8_lossy(&b).into_owned(),
            Message::Close(_) => break,
            _ => continue,
        };
        let v: Value = match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(_) => continue,
        };
        // Notifications come as {"method":"accountNotification","params":{"result":{"value":{"data":[...]}}}}
        if v.get("method").and_then(|m| m.as_str()) != Some("accountNotification") {
            continue;
        }
        let data_arr = match v
            .pointer("/params/result/value/data")
            .and_then(|d| d.as_array())
        {
            Some(a) => a,
            None => continue,
        };
        let b64 = match data_arr.get(0).and_then(|s| s.as_str()) {
            Some(s) => s,
            None => continue,
        };
        let bytes = match base64::engine::general_purpose::STANDARD.decode(b64) {
            Ok(b) => b,
            Err(_) => continue,
        };
        if bytes.len() < 183 {
            continue;
        }
        let status = bytes[75];
        let round_id = u64::from_le_bytes(bytes[76..84].try_into().unwrap());
        let randomness: [u8; 32] = bytes[151..183].try_into().unwrap();
        let _ = tx.send((status, round_id, randomness));
    }
    Ok(())
}

/// Recover the channel back to Idle from any non-Idle state.
///
/// - Finalized (4) → `deliver_callback` with empty remaining_accounts.
/// - Pending (1) / CommitPhase (2) / RevealPhase (3) → `fail_round`, but
///   only after the on-chain commit/reveal deadline has passed (at most
///   ~60 s). We wait for that if needed so the next round can start.
/// - Failed (5) → call request_randomness_auto on the next round and let
///   reset_for_new_round clobber the state. (Handled by the caller.)
async fn drain_to_idle(
    rpc: &Rpc,
    payer: &Keypair,
    dice_pid: &Pubkey,
    channel_pda: &Pubkey,
) -> Result<()> {
    let data = rpc.get_account_data(channel_pda).await?.unwrap_or_default();
    if data.len() < 96 {
        return Ok(());
    }
    let status = data[75];
    let round_id = u64::from_le_bytes(data[76..84].try_into().unwrap());

    match status {
        0 => Ok(()), // already Idle
        4 => {
            println!("  channel Finalized (round_id={}), deliver_callback → Idle", round_id);
            let ix = build_deliver_callback_idle_ix(dice_pid, &payer.pubkey(), channel_pda, round_id);
            let sig = rpc.sign_and_send(payer, vec![ix]).await?;
            wait_confirmed(rpc, &sig).await?;
            Ok(())
        }
        1 | 2 | 3 | 5 => {
            println!(
                "  channel status={} (round_id={}), waiting up to 70 s for deadline then fail_round",
                status, round_id
            );
            // Try fail_round immediately; if deadline hasn't passed, sleep
            // a few seconds and retry. Commit timeout is 150 slots ~= 60 s.
            for attempt in 0..15 {
                let ix = build_fail_round_ix(dice_pid, &payer.pubkey(), channel_pda);
                match rpc.sign_and_send(payer, vec![ix]).await {
                    Ok(sig) => {
                        wait_confirmed(rpc, &sig).await?;
                        println!("  fail_round TX: {}", sig);
                        return Ok(());
                    }
                    Err(e) => {
                        if attempt == 14 {
                            return Err(anyhow!("fail_round never succeeded: {}", e));
                        }
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                }
            }
            Err(anyhow!("drain_to_idle exhausted retries"))
        }
        _ => Ok(()),
    }
}

// ─── Instruction builders ───────────────────────────────────────────────────

fn anchor_discriminator(name: &str) -> [u8; 8] {
    let h = Sha256::digest(format!("global:{name}").as_bytes());
    let mut out = [0u8; 8];
    out.copy_from_slice(&h[..8]);
    out
}

fn build_init_channel_ix(
    program_id: &Pubkey,
    authority: &Pubkey,
    channel: &Pubkey,
    channel_index: u16,
    max_nodes: u8,
    callback_program_id: &Pubkey,
    coordinator: &Pubkey,
) -> Instruction {
    let mut data = anchor_discriminator("init_channel").to_vec();
    data.extend_from_slice(&channel_index.to_le_bytes());
    data.push(max_nodes);
    data.extend_from_slice(callback_program_id.as_ref());
    data.extend_from_slice(coordinator.as_ref());
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*authority, true),
            AccountMeta::new(*channel, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data,
    }
}

fn build_fund_channel_ix(
    program_id: &Pubkey,
    authority: &Pubkey,
    channel: &Pubkey,
    amount: u64,
) -> Instruction {
    let mut data = anchor_discriminator("fund_channel").to_vec();
    data.extend_from_slice(&amount.to_le_bytes());
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*authority, true),
            AccountMeta::new(*channel, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data,
    }
}

fn build_request_randomness_auto_ix(
    program_id: &Pubkey,
    authority: &Pubkey,
    channel: &Pubkey,
    node_count: u8,
    // If non-empty: `request_randomness_auto` will run on-chain node
    // selection. First entry becomes SlotHashes sysvar, rest are
    // DeviceRegistry PDAs. Empty → coord picks off-chain.
    device_registries: &[Pubkey],
) -> Instruction {
    let mut data = anchor_discriminator("request_randomness_auto").to_vec();
    data.push(node_count);
    let mut accounts = vec![
        AccountMeta::new(*authority, true),
        AccountMeta::new(*channel, false),
        AccountMeta::new_readonly(system_program::id(), false),
    ];
    if !device_registries.is_empty() {
        accounts.push(AccountMeta::new_readonly(
            solana_sdk::sysvar::slot_hashes::id(),
            false,
        ));
        for dr in device_registries {
            accounts.push(AccountMeta::new_readonly(*dr, false));
        }
    }
    Instruction { program_id: *program_id, accounts, data }
}

fn build_fail_round_ix(
    program_id: &Pubkey,
    caller: &Pubkey,
    channel: &Pubkey,
) -> Instruction {
    let data = anchor_discriminator("fail_round").to_vec();
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new_readonly(*caller, true),
            AccountMeta::new(*channel, false),
        ],
        data,
    }
}

fn build_deliver_callback_idle_ix(
    program_id: &Pubkey,
    coordinator: &Pubkey,
    channel: &Pubkey,
    round_id: u64,
) -> Instruction {
    let mut data = anchor_discriminator("deliver_callback").to_vec();
    data.extend_from_slice(&round_id.to_le_bytes());
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new_readonly(*coordinator, true),
            AccountMeta::new(*channel, false),
        ],
        data,
    }
}

// ─── Minimal reqwest RPC ───────────────────────────────────────────────────

struct Rpc {
    url: String,
    client: reqwest::Client,
}

impl Rpc {
    fn new(url: String) -> Self {
        Self { url, client: reqwest::Client::new() }
    }

    async fn call(&self, method: &str, params: Value) -> Result<Value> {
        let body = json!({ "jsonrpc":"2.0","id":1,"method":method,"params":params });
        let v: Value = self
            .client
            .post(&self.url)
            .json(&body)
            .send()
            .await?
            .json()
            .await?;
        if let Some(err) = v.get("error") {
            return Err(anyhow!("RPC error: {}", err));
        }
        v.get("result").cloned().ok_or_else(|| anyhow!("missing result"))
    }

    async fn get_balance(&self, pk: &Pubkey) -> Result<u64> {
        let r = self
            .call(
                "getBalance",
                json!([pk.to_string(), {"commitment": "processed"}]),
            )
            .await?;
        r["value"].as_u64().ok_or_else(|| anyhow!("missing balance"))
    }

    async fn get_latest_blockhash(&self) -> Result<Hash> {
        let r = self
            .call("getLatestBlockhash", json!([{"commitment": "confirmed"}]))
            .await?;
        let s = r["value"]["blockhash"]
            .as_str()
            .ok_or_else(|| anyhow!("missing blockhash"))?;
        Hash::from_str(s).map_err(|e| anyhow!("parse blockhash: {}", e))
    }

    async fn sign_and_send(&self, payer: &Keypair, ixs: Vec<Instruction>) -> Result<Signature> {
        let bh = self.get_latest_blockhash().await?;
        let tx = Transaction::new_signed_with_payer(&ixs, Some(&payer.pubkey()), &[payer], bh);
        let bytes = bincode::serialize(&tx)?;
        let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
        let r = self
            .call(
                "sendTransaction",
                json!([encoded, {"encoding":"base64","skipPreflight":false,"preflightCommitment":"confirmed"}]),
            )
            .await?;
        let s = r.as_str().ok_or_else(|| anyhow!("sig not string"))?;
        Signature::from_str(s).map_err(|e| anyhow!("parse sig: {}", e))
    }

    async fn get_account_data(&self, pk: &Pubkey) -> Result<Option<Vec<u8>>> {
        let r = self
            .call(
                "getAccountInfo",
                json!([pk.to_string(), {"encoding":"base64","commitment":"processed"}]),
            )
            .await?;
        let val = match r.get("value") { Some(v) => v, None => return Ok(None) };
        if val.is_null() {
            return Ok(None);
        }
        let data_arr = val
            .get("data")
            .and_then(|d| d.as_array())
            .ok_or_else(|| anyhow!("missing data array"))?;
        let b64 = data_arr[0].as_str().ok_or_else(|| anyhow!("missing data b64"))?;
        Ok(Some(
            base64::engine::general_purpose::STANDARD.decode(b64)?,
        ))
    }

    /// Enumerate all registered `DeviceRegistry` PDAs for the dice program.
    /// Returns (DeviceRegistry pubkey, device_pubkey) pairs so callers can
    /// filter by device_pubkey content without a second RPC round-trip.
    async fn list_device_registries(
        &self,
        program_id: &Pubkey,
    ) -> Result<Vec<(Pubkey, Vec<u8>)>> {
        // DeviceRegistry discriminator = SHA-256("account:DeviceRegistry")[0..8]
        // Matches programs/dice/src/instructions/select_nodes.rs: DEVICE_REGISTRY_DISC.
        let disc_bytes: [u8; 8] = [103, 245, 70, 187, 154, 60, 208, 216];
        let b58 = bs58_encode(&disc_bytes);
        let r = self
            .call(
                "getProgramAccounts",
                json!([
                    program_id.to_string(),
                    {
                        "encoding": "base64",
                        "commitment": "confirmed",
                        "filters": [{ "memcmp": { "offset": 0, "bytes": b58 } }]
                    }
                ]),
            )
            .await?;
        let arr = r.as_array().ok_or_else(|| anyhow!("gPA result not array"))?;
        let mut out = Vec::with_capacity(arr.len());
        for entry in arr {
            let pk_str = match entry.get("pubkey").and_then(|v| v.as_str()) {
                Some(s) => s,
                None => continue,
            };
            let pk = match Pubkey::from_str(pk_str) {
                Ok(p) => p,
                Err(_) => continue,
            };
            // DeviceRegistry layout: 8 (disc) + 33 (device_pubkey) + ...
            let data = entry
                .pointer("/account/data")
                .and_then(|d| d.as_array())
                .and_then(|a| a.first())
                .and_then(|s| s.as_str());
            let device_pubkey = match data {
                Some(b64) => {
                    let bytes = base64::engine::general_purpose::STANDARD
                        .decode(b64)
                        .unwrap_or_default();
                    if bytes.len() >= 41 { bytes[8..41].to_vec() } else { Vec::new() }
                }
                None => Vec::new(),
            };
            out.push((pk, device_pubkey));
        }
        Ok(out)
    }
}

/// Minimal base58 encoder for fixed-size byte arrays (used by the DeviceRegistry
/// filter — Solana JSON-RPC requires base58 for memcmp byte patterns).
fn bs58_encode(input: &[u8]) -> String {
    const ALPHABET: &[u8] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
    // Count leading zeros
    let mut leading_zeros = 0usize;
    for &b in input {
        if b == 0 { leading_zeros += 1; } else { break; }
    }
    // Convert bytes → big integer → base58 digits.
    let mut digits: Vec<u8> = Vec::with_capacity(input.len() * 2);
    for &b in input {
        let mut carry = b as u32;
        for d in digits.iter_mut() {
            carry += (*d as u32) * 256;
            *d = (carry % 58) as u8;
            carry /= 58;
        }
        while carry > 0 {
            digits.push((carry % 58) as u8);
            carry /= 58;
        }
    }
    let mut out = String::with_capacity(leading_zeros + digits.len());
    for _ in 0..leading_zeros { out.push('1'); }
    for &d in digits.iter().rev() { out.push(ALPHABET[d as usize] as char); }
    out
}

/// Polls every 250 ms accepting `confirmed` (the safe original behavior).
/// Reverted from the 150ms+processed variant which surfaced cascading
/// state-read races under devnet backlog load.
async fn wait_confirmed(rpc: &Rpc, sig: &Signature) -> Result<()> {
    for _ in 0..50 {
        tokio::time::sleep(Duration::from_millis(250)).await;
        let r = rpc
            .call("getSignatureStatuses", json!([[sig.to_string()]]))
            .await?;
        if let Some(status) = r["value"].get(0) {
            if !status.is_null() {
                if let Some(err) = status.get("err") {
                    if !err.is_null() {
                        return Err(anyhow!("tx failed: {}", err));
                    }
                }
                if let Some(confs) = status.get("confirmationStatus").and_then(|v| v.as_str()) {
                    if confs == "confirmed" || confs == "finalized" {
                        return Ok(());
                    }
                }
            }
        }
    }
    Err(anyhow!("confirmation timeout for {}", sig))
}

fn load_keypair(path: &std::path::Path) -> Result<Keypair> {
    let data = std::fs::read_to_string(path)?;
    let bytes: Vec<u8> = serde_json::from_str(&data)?;
    Keypair::from_bytes(&bytes).map_err(|e| anyhow!("invalid keypair: {}", e))
}

// Silence unused warning for the SystemTime import in the stub below.
#[allow(dead_code)]
fn _epoch() -> SystemTime {
    SystemTime::UNIX_EPOCH
}
