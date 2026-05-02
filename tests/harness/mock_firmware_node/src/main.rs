// Mock Firmware Node
// Simulates N ESP32-S3 DICE nodes for integration and E2E testing.
// Faithfully reproduces the firmware's k256 ECDSA signing and CBOR protocol.
//
// Usage:
//   mock-firmware-node --count 7 --coordinator ws://localhost:8443 --insecure

use std::{sync::Arc, time::Duration};

use anyhow::{Context, Result};
use clap::Parser;
use futures_util::{SinkExt, StreamExt};
use k256::ecdsa::{signature::Signer, Signature, SigningKey};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::time::sleep;
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Parser)]
#[command(
    name = "mock-firmware-node",
    about = "Simulates DICE ESP32-S3 nodes for integration testing"
)]
struct Args {
    /// Number of simulated nodes to spawn
    #[arg(short = 'n', long, default_value_t = 5)]
    count: usize,

    /// Coordinator WebSocket URL
    #[arg(short, long, default_value = "ws://localhost:8443")]
    coordinator: String,

    /// Skip TLS certificate verification (for ws:// or self-signed certs)
    #[arg(long, default_value_t = false)]
    insecure: bool,

    /// Heartbeat interval in milliseconds
    #[arg(long, default_value_t = 5000)]
    heartbeat_ms: u64,

    /// Delay in milliseconds between receiving a job and submitting commit
    #[arg(long, default_value_t = 50)]
    commit_delay_ms: u64,

    /// Delay in milliseconds between commit and reveal
    #[arg(long, default_value_t = 500)]
    reveal_delay_ms: u64,
}

// ---------------------------------------------------------------------------
// DICE CBOR protocol (mirrors firmware dice_protocol.c)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
struct Heartbeat {
    node_id: Vec<u8>,
    latency_ms: u64,
    uptime_secs: u64,
    jobs_completed: u64,
    timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
struct JobAssignment {
    request_id: Vec<u8>,
    round_seq: u64,
    deadline_ts: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
struct CommitSubmission {
    request_id: Vec<u8>,
    node_id: Vec<u8>,
    commit_hash: Vec<u8>,
    signature: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
struct RevealSubmission {
    request_id: Vec<u8>,
    node_id: Vec<u8>,
    entropy: Vec<u8>,
    signature: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
struct RoundResult {
    request_id: Vec<u8>,
    status: String,
    randomness: Vec<u8>,
}

/// Encode a tagged CBOR envelope: `[tag_text, payload_map]`
fn cbor_encode<T: Serialize>(tag: &str, payload: &T) -> Result<Vec<u8>> {
    let payload_val = ciborium::Value::serialized(payload)
        .context("CBOR serialize payload")?;
    let envelope = ciborium::Value::Array(vec![
        ciborium::Value::Text(tag.to_string()),
        payload_val,
    ]);
    let mut out = Vec::new();
    ciborium::ser::into_writer(&envelope, &mut out)
        .context("CBOR encode envelope")?;
    Ok(out)
}

/// Decode a CBOR message in either format:
///   1. Array envelope: `["tag", {payload}]` (legacy/SDK)
///   2. Integer-key map: `{0: type, 1: field, ...}` (firmware format, used by v7.7+ coordinator)
fn cbor_decode(bytes: &[u8]) -> Result<(String, ciborium::Value)> {
    let value: ciborium::Value =
        ciborium::de::from_reader(bytes).context("CBOR decode")?;
    match value {
        // Format 1: array envelope ["tag", {payload}]
        ciborium::Value::Array(a) if a.len() == 2 => {
            let tag = match &a[0] {
                ciborium::Value::Text(t) => t.clone(),
                _ => anyhow::bail!("first element must be text tag"),
            };
            Ok((tag, a.into_iter().nth(1).unwrap()))
        }
        // Format 2: integer-key map {0: type_uint, 1: ..., 2: ..., ...}
        ciborium::Value::Map(ref pairs) => {
            // Extract message type from key 0
            let msg_type = pairs.iter()
                .find_map(|(k, v)| {
                    if let ciborium::Value::Integer(ki) = k {
                        let key: u64 = (*ki).try_into().ok()?;
                        if key == 0 {
                            if let ciborium::Value::Integer(vi) = v {
                                return Some((*vi).try_into().unwrap_or(0u64));
                            }
                        }
                    }
                    None
                })
                .ok_or_else(|| anyhow::anyhow!("missing type field (key 0) in integer-key map"))?;

            let tag = match msg_type {
                1 => "job_assignment",
                4 => "round_result",
                _ => anyhow::bail!("unknown integer-key message type: {}", msg_type),
            };

            // Convert integer-key map to a text-key map for deserialization
            Ok((tag.to_string(), value))
        }
        _ => anyhow::bail!("expected CBOR array[2] or map"),
    }
}

/// Deserialise the CBOR payload Value into a concrete type.
/// Handles both text-key maps (array envelope format) and integer-key maps (firmware format).
fn cbor_decode_payload<T: for<'de> Deserialize<'de>>(val: ciborium::Value) -> Result<T> {
    let mut buf = Vec::new();
    ciborium::ser::into_writer(&val, &mut buf).context("re-encode payload")?;
    ciborium::de::from_reader(buf.as_slice()).context("deserialise payload")
}

/// Extract a JobAssignment from an integer-key CBOR map.
/// Layout: {0:1, 1:request_id, 2:round_seq, 3:deadline_ts}
fn decode_job_assignment_intkey(val: &ciborium::Value) -> Result<JobAssignment> {
    let pairs = match val {
        ciborium::Value::Map(p) => p,
        _ => anyhow::bail!("expected map for integer-key JobAssignment"),
    };
    let mut request_id = None;
    let mut round_seq = None;
    let mut deadline_ts = None;
    for (k, v) in pairs {
        if let ciborium::Value::Integer(ki) = k {
            let key: u64 = (*ki).try_into().unwrap_or(999);
            match key {
                1 => if let ciborium::Value::Bytes(b) = v { request_id = Some(b.clone()); },
                2 => if let ciborium::Value::Integer(i) = v { round_seq = Some((*i).try_into().unwrap_or(0u64)); },
                3 => if let ciborium::Value::Integer(i) = v { deadline_ts = Some((*i).try_into().unwrap_or(0u64)); },
                _ => {}
            }
        }
    }
    Ok(JobAssignment {
        request_id: request_id.ok_or_else(|| anyhow::anyhow!("missing request_id"))?,
        round_seq: round_seq.ok_or_else(|| anyhow::anyhow!("missing round_seq"))?,
        deadline_ts: deadline_ts.ok_or_else(|| anyhow::anyhow!("missing deadline_ts"))?,
    })
}

/// Extract a RoundResult from an integer-key CBOR map.
/// Layout: {0:4, 1:request_id, 2:status, 3:randomness}
fn decode_round_result_intkey(val: &ciborium::Value) -> Result<RoundResult> {
    let pairs = match val {
        ciborium::Value::Map(p) => p,
        _ => anyhow::bail!("expected map for integer-key RoundResult"),
    };
    let mut request_id = None;
    let mut status = None;
    let mut randomness = None;
    for (k, v) in pairs {
        if let ciborium::Value::Integer(ki) = k {
            let key: u64 = (*ki).try_into().unwrap_or(999);
            match key {
                1 => if let ciborium::Value::Bytes(b) = v { request_id = Some(b.clone()); },
                2 => if let ciborium::Value::Text(t) = v { status = Some(t.clone()); },
                3 => if let ciborium::Value::Bytes(b) = v { randomness = Some(b.clone()); },
                _ => {}
            }
        }
    }
    Ok(RoundResult {
        request_id: request_id.ok_or_else(|| anyhow::anyhow!("missing request_id"))?,
        status: status.ok_or_else(|| anyhow::anyhow!("missing status"))?,
        randomness: randomness.ok_or_else(|| anyhow::anyhow!("missing randomness"))?,
    })
}

// ---------------------------------------------------------------------------
// Node identity
// ---------------------------------------------------------------------------

struct NodeIdentity {
    signing_key: SigningKey,
    node_id: [u8; 33], // compressed secp256k1 pubkey
}

impl NodeIdentity {
    fn random() -> Self {
        let signing_key = SigningKey::random(&mut rand::thread_rng());
        let vk = signing_key.verifying_key();
        let compressed = vk.to_encoded_point(true);
        let node_id: [u8; 33] = compressed
            .as_bytes()
            .try_into()
            .expect("compressed pubkey is always 33 bytes");
        NodeIdentity { signing_key, node_id }
    }

    /// Sign a 32-byte message with ECDSA secp256k1 (compact r‖s, 64 bytes).
    fn sign(&self, msg: &[u8; 32]) -> [u8; 64] {
        let sig: Signature = self.signing_key.sign(msg.as_slice());
        sig.to_bytes().into()
    }
}

// ---------------------------------------------------------------------------
// Single-node task
// ---------------------------------------------------------------------------

async fn run_node(identity: Arc<NodeIdentity>, args: Arc<Args>, node_index: usize) {
    let short_id = hex::encode(&identity.node_id[..6]);
    info!(node = %short_id, index = node_index, url = %args.coordinator, "node starting");

    let start = std::time::Instant::now();
    let mut jobs_completed: u64 = 0;

    loop {
        // Attempt WebSocket connection.
        match connect_and_run(&identity, &args, start, &mut jobs_completed, &short_id).await {
            Ok(()) => {
                info!(node = %short_id, "node disconnected cleanly, reconnecting in 3s");
            }
            Err(e) => {
                warn!(node = %short_id, error = %e, "node connection error, reconnecting in 3s");
            }
        }
        sleep(Duration::from_secs(3)).await;
    }
}

async fn connect_and_run(
    identity: &Arc<NodeIdentity>,
    args: &Arc<Args>,
    start: std::time::Instant,
    jobs_completed: &mut u64,
    short_id: &str,
) -> Result<()> {
    use tokio_tungstenite::{connect_async, tungstenite::Message};

    let (ws_stream, _) = connect_async(&args.coordinator)
        .await
        .with_context(|| format!("connect to {}", args.coordinator))?;

    info!(node = %short_id, "WebSocket connected");

    let (mut ws_sink, mut ws_stream) = ws_stream.split();

    // Heartbeat ticker.
    let heartbeat_interval = Duration::from_millis(args.heartbeat_ms);
    let mut hb_ticker = tokio::time::interval(heartbeat_interval);

    // Pending job: once we receive a JobAssignment, store it here.
    // Key: request_id bytes, Value: (entropy, commit_hash, deadline_ts)
    let mut pending_job: Option<PendingJob> = None;
    // Commit reveal timer: fires after reveal_delay_ms
    let mut reveal_at: Option<tokio::time::Instant> = None;

    loop {
        tokio::select! {
            // ---- Heartbeat tick ----
            _ = hb_ticker.tick() => {
                let hb = Heartbeat {
                    node_id: identity.node_id.to_vec(),
                    latency_ms: 1,
                    uptime_secs: start.elapsed().as_secs(),
                    jobs_completed: *jobs_completed,
                    timestamp: unix_now(),
                };
                let bytes = cbor_encode("heartbeat", &hb)?;
                ws_sink.send(Message::Binary(bytes.into())).await
                    .context("send heartbeat")?;
            }

            // ---- Reveal timer ----
            _ = async {
                if let Some(t) = reveal_at {
                    tokio::time::sleep_until(t).await;
                } else {
                    // Never fire if no timer set.
                    std::future::pending::<()>().await;
                }
            } => {
                if let Some(ref job) = pending_job {
                    let reveal = RevealSubmission {
                        request_id: job.request_id.clone(),
                        node_id: identity.node_id.to_vec(),
                        entropy: job.entropy.to_vec(),
                        signature: identity.sign(&job.entropy).to_vec(),
                    };
                    let bytes = cbor_encode("reveal_submission", &reveal)?;
                    ws_sink.send(Message::Binary(bytes.into())).await
                        .context("send reveal")?;
                    info!(node = %short_id, request = hex::encode(&job.request_id[..8]), "reveal sent");
                    *jobs_completed += 1;
                }
                // Clear the job and timer after sending reveal.
                pending_job = None;
                reveal_at = None;
            }

            // ---- Incoming message ----
            msg = ws_stream.next() => {
                let raw = match msg {
                    Some(Ok(Message::Binary(b))) => b,
                    Some(Ok(Message::Close(_))) | None => {
                        info!(node = %short_id, "WebSocket closed by coordinator");
                        return Ok(());
                    }
                    Some(Ok(_)) => continue,
                    Some(Err(e)) => return Err(e.into()),
                };

                let (tag, payload) = match cbor_decode(&raw) {
                    Ok(t) => t,
                    Err(e) => { warn!(node = %short_id, error = %e, "CBOR decode error"); continue; }
                };

                match tag.as_str() {
                    "job_assignment" => {
                        let job: JobAssignment = match decode_job_assignment_intkey(&payload)
                            .or_else(|_| cbor_decode_payload(payload.clone()))
                        {
                            Ok(j) => j,
                            Err(e) => { warn!(node = %short_id, error = %e, "bad JobAssignment"); continue; }
                        };

                        info!(
                            node = %short_id,
                            request = hex::encode(&job.request_id[..8.min(job.request_id.len())]),
                            seq = job.round_seq,
                            "job assignment received"
                        );

                        // Generate random entropy.
                        let mut entropy = [0u8; 32];
                        rand::thread_rng().fill_bytes(&mut entropy);

                        // Compute commit hash = SHA-256(entropy).
                        let commit_hash: [u8; 32] = Sha256::digest(entropy).into();

                        // Sign the commit hash.
                        let sig = identity.sign(&commit_hash);

                        // Store the pending job (entropy + commit for reveal later).
                        pending_job = Some(PendingJob {
                            request_id: job.request_id.clone(),
                            entropy,
                            commit_hash,
                        });

                        // Schedule reveal after delay.
                        reveal_at = Some(
                            tokio::time::Instant::now()
                                + Duration::from_millis(args.reveal_delay_ms),
                        );

                        // Send commit after a short delay.
                        sleep(Duration::from_millis(args.commit_delay_ms)).await;

                        let commit = CommitSubmission {
                            request_id: job.request_id,
                            node_id: identity.node_id.to_vec(),
                            commit_hash: commit_hash.to_vec(),
                            signature: sig.to_vec(),
                        };
                        let bytes = cbor_encode("commit_submission", &commit)?;
                        ws_sink.send(Message::Binary(bytes.into())).await
                            .context("send commit")?;
                        info!(
                            node = %short_id,
                            commit = hex::encode(&commit_hash[..8]),
                            "commit sent"
                        );
                    }

                    "round_result" => {
                        let result: RoundResult = match decode_round_result_intkey(&payload)
                            .or_else(|_| cbor_decode_payload(payload.clone()))
                        {
                            Ok(r) => r,
                            Err(e) => { warn!(node = %short_id, error = %e, "bad RoundResult"); continue; }
                        };
                        if result.status == "finalized" {
                            info!(
                                node = %short_id,
                                randomness = hex::encode(&result.randomness[..8.min(result.randomness.len())]),
                                "round finalized ✓"
                            );
                        } else {
                            warn!(node = %short_id, status = %result.status, "round ended with non-finalized status");
                        }
                    }

                    other => {
                        warn!(node = %short_id, tag = %other, "unexpected message tag");
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

struct PendingJob {
    request_id: Vec<u8>,
    entropy: [u8; 32],
    commit_hash: [u8; 32],
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> Result<()> {
    let args = Arc::new(Args::parse());

    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!(
        count = args.count,
        coordinator = %args.coordinator,
        insecure = args.insecure,
        "mock-firmware-node starting"
    );

    // Spawn N node tasks, each with their own k256 identity.
    let mut handles = Vec::with_capacity(args.count);
    for i in 0..args.count {
        let identity = Arc::new(NodeIdentity::random());
        let args = args.clone();
        info!(
            index = i,
            node_id = hex::encode(identity.node_id),
            "spawning node"
        );
        handles.push(tokio::spawn(run_node(identity, args, i)));
    }

    // Wait for all tasks (they loop forever unless the coordinator disappears).
    for handle in handles {
        if let Err(e) = handle.await {
            error!(error = ?e, "node task panicked");
        }
    }

    Ok(())
}
