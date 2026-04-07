//! DICE Notary — decentralized timestamping and attestation service.
//!
//! Multi-witness hardware attestation: connected ESP32 nodes sign document
//! hashes with their hardware-bound ECDSA keys, producing a self-contained
//! receipt that is independently verifiable.

use std::{
    collections::VecDeque,
    sync::Arc,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tracing::{info, warn};
use uuid::Uuid;

use crate::{
    metrics::Metrics,
    node_session::{get_active_nodes, NodeRegistry},
    protocol::messages::{DiceMessage, JobAssignment},
    state_machine::{Round, RoundEntry, RoundMap},
};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const MAX_HISTORY: usize = 100;

// ---------------------------------------------------------------------------
// Request / Response types
// ---------------------------------------------------------------------------

/// Incoming notarization request from the API.
#[derive(Debug, Deserialize)]
pub struct NotarizeRequest {
    /// Document hash as hex string (64 chars = 32 bytes SHA-256).
    pub hash: String,
    /// Hash algorithm used (defaults to "sha256").
    #[serde(default = "default_hash_algo")]
    pub hash_algorithm: String,
    /// Optional metadata attached to the attestation.
    pub metadata: Option<serde_json::Value>,
}

fn default_hash_algo() -> String {
    "sha256".to_string()
}

/// A single witness attestation entry in the receipt.
#[derive(Debug, Clone, Serialize)]
pub struct WitnessInfo {
    pub device_pubkey: String,
    pub commit_hash: String,
    pub signature: String,
}

/// Verification instructions embedded in the receipt.
#[derive(Debug, Clone, Serialize)]
pub struct VerificationInfo {
    pub instructions: String,
}

/// Complete notary receipt — self-contained, independently verifiable.
#[derive(Debug, Clone, Serialize)]
pub struct NotaryReceipt {
    pub version: String,
    #[serde(rename = "type")]
    pub receipt_type: String,
    pub receipt_id: String,

    pub attestation: AttestationData,
    pub witnesses: Vec<WitnessInfo>,
    pub witness_count: u8,
    pub threshold: u8,

    pub protocol: ProtocolInfo,
    pub verification: VerificationInfo,
}

#[derive(Debug, Clone, Serialize)]
pub struct AttestationData {
    pub content_hash: String,
    pub hash_algorithm: String,
    pub timestamp_unix: u64,
    pub timestamp_iso: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProtocolInfo {
    pub network: String,
    pub coordinator: String,
    pub min_witnesses: u8,
}

// ---------------------------------------------------------------------------
// Shared state
// ---------------------------------------------------------------------------

pub struct NotaryManager {
    pub history: VecDeque<NotaryReceipt>,
    pub total_attestations: u64,
}

pub type NotaryState = Arc<Mutex<NotaryManager>>;

pub fn new_notary_state() -> NotaryState {
    Arc::new(Mutex::new(NotaryManager {
        history: VecDeque::with_capacity(MAX_HISTORY),
        total_attestations: 0,
    }))
}

// ---------------------------------------------------------------------------
// Notarization handler
// ---------------------------------------------------------------------------

/// Handle a notarization request.
///
/// Flow:
/// 1. Parse and validate the hash
/// 2. Select N nodes from the registry
/// 3. Dispatch hash as a VRF-style JobAssignment (reusing existing pipeline)
/// 4. Wait for commit signatures from hardware nodes
/// 5. Build and return a self-contained receipt
///
/// For the hackathon, we piggyback on the VRF commit flow. Nodes receive a
/// JobAssignment with the content_hash as the request_id. Their ECDSA-signed
/// commit submission constitutes attestation that the hardware device processed
/// this hash at this time.
pub async fn handle_notarize(
    request: NotarizeRequest,
    registry: &NodeRegistry,
    rounds: &RoundMap,
    metrics: &Metrics,
    min_witnesses: usize,
    coordinator_pubkey: &str,
) -> Result<NotaryReceipt> {
    let start = Instant::now();

    // 1. Validate hash input.
    let hash_bytes = hex::decode(&request.hash)
        .map_err(|_| anyhow!("invalid hex hash"))?;
    if hash_bytes.len() != 32 {
        return Err(anyhow!("hash must be 32 bytes (64 hex chars), got {}", hash_bytes.len()));
    }

    let mut content_hash = [0u8; 32];
    content_hash.copy_from_slice(&hash_bytes);

    // 2. Get active nodes.
    let active_nodes = get_active_nodes(registry).await;
    if active_nodes.len() < min_witnesses {
        return Err(anyhow!(
            "need {} witnesses but only {} nodes online",
            min_witnesses,
            active_nodes.len()
        ));
    }

    // Select up to 7 nodes (same as VRF).
    let selected: Vec<[u8; 33]> = active_nodes.into_iter().take(7).collect();
    let node_count = selected.len();

    // 3. Dispatch as a JobAssignment (piggyback on existing pipeline).
    let sequence = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    let deadline_ts = sequence + 30; // 30-second deadline for notary

    let round = Round::new(
        content_hash,
        selected.clone(),
        min_witnesses,
        std::time::Duration::from_secs(30),
    );

    let db_id = Uuid::new_v4();
    let entry = RoundEntry {
        round,
        db_id,
        started_at: Instant::now(),
        requester: solana_sdk::pubkey::Pubkey::default(),
        sequence,
        channel_authority: None,
        channel_index: None,
    };

    {
        let mut map = rounds.lock().await;
        map.insert(content_hash, entry);
    }

    // Encode and send JobAssignment to selected nodes.
    let job = DiceMessage::JobAssignment(JobAssignment {
        request_id: content_hash.to_vec(),
        round_seq: sequence,
        deadline_ts,
    });

    let encoded = job.encode()
        .map_err(|e| anyhow!("CBOR encode failed: {}", e))?;

    let mut dispatched = 0usize;
    {
        let reg = registry.read().await;
        for node_id in &selected {
            if let Some(session) = reg.get(node_id) {
                if session.tx.try_send(encoded.clone()).is_ok() {
                    dispatched += 1;
                }
            }
        }
    }

    if dispatched < min_witnesses {
        // Clean up the round.
        let mut map = rounds.lock().await;
        map.remove(&content_hash);
        return Err(anyhow!(
            "could only dispatch to {}/{} nodes, need {}",
            dispatched,
            node_count,
            min_witnesses
        ));
    }

    info!(
        hash = %request.hash,
        dispatched,
        node_count,
        "notarize request dispatched to nodes"
    );

    // 4. Wait for commits to arrive (poll the round state).
    //    The normal VRF handler in main.rs will process CommitSubmissions.
    //    We wait until enough commits arrive or timeout.
    let timeout = tokio::time::sleep(std::time::Duration::from_secs(30));
    tokio::pin!(timeout);

    let mut witnesses = Vec::new();

    loop {
        tokio::select! {
            _ = &mut timeout => {
                warn!(hash = %request.hash, "notarize timed out waiting for commits");
                break;
            }
            _ = tokio::time::sleep(std::time::Duration::from_millis(500)) => {
                let map = rounds.lock().await;
                if let Some(entry) = map.get(&content_hash) {
                    let (commits_received, _) = entry.round.progress_counts();
                    if commits_received >= min_witnesses {
                        // Collect witness data from the round.
                        witnesses = collect_witnesses(&entry.round, &selected);
                        break;
                    }
                } else {
                    // Round was finalized/failed by the VRF handler — still collect what we can.
                    break;
                }
            }
        }
    }

    // If we didn't get enough from polling, try once more.
    if witnesses.len() < min_witnesses {
        let map = rounds.lock().await;
        if let Some(entry) = map.get(&content_hash) {
            witnesses = collect_witnesses(&entry.round, &selected);
        }
    }

    // Clean up: remove from round map (notary doesn't need finalization).
    {
        let mut map = rounds.lock().await;
        map.remove(&content_hash);
    }

    if witnesses.len() < min_witnesses {
        return Err(anyhow!(
            "only got {}/{} witness signatures, need {}",
            witnesses.len(),
            node_count,
            min_witnesses
        ));
    }

    // 5. Build receipt.
    let unix_now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let iso_now = chrono::Utc::now().to_rfc3339();

    let receipt = NotaryReceipt {
        version: "1.0".into(),
        receipt_type: "dice-notary-receipt".into(),
        receipt_id: db_id.to_string(),
        attestation: AttestationData {
            content_hash: format!("{}:{}", request.hash_algorithm, request.hash),
            hash_algorithm: request.hash_algorithm,
            timestamp_unix: unix_now,
            timestamp_iso: iso_now,
        },
        witnesses: witnesses.clone(),
        witness_count: witnesses.len() as u8,
        threshold: min_witnesses as u8,
        protocol: ProtocolInfo {
            network: "solana-devnet".into(),
            coordinator: coordinator_pubkey.to_string(),
            min_witnesses: min_witnesses as u8,
        },
        verification: VerificationInfo {
            instructions: format!(
                "To verify: (1) Confirm each witness signature is valid ECDSA secp256k1 \
                 over commit_hash using device_pubkey. (2) Confirm device_pubkey is \
                 registered in the DICE DeviceRegistry on-chain. (3) {} witnesses \
                 signed, threshold is {}.",
                witnesses.len(),
                min_witnesses
            ),
        },
    };

    let latency_ms = start.elapsed().as_millis() as f64 / 1000.0;
    metrics.notary_attestations_total.inc();
    metrics.notary_attestation_latency_seconds.observe(latency_ms);

    info!(
        receipt_id = %receipt.receipt_id,
        witnesses = witnesses.len(),
        latency_ms = start.elapsed().as_millis(),
        "notary attestation complete"
    );

    Ok(receipt)
}

/// Extract witness attestation info from a round's commit data.
fn collect_witnesses(round: &Round, _selected: &[[u8; 33]]) -> Vec<WitnessInfo> {
    round
        .commit_data()
        .iter()
        .map(|(node_id, commit_hash, signature)| WitnessInfo {
            device_pubkey: hex::encode(node_id),
            commit_hash: hex::encode(commit_hash),
            signature: hex::encode(signature),
        })
        .collect()
}

/// Get notary stats.
pub fn get_notary_stats(manager: &NotaryManager) -> serde_json::Value {
    serde_json::json!({
        "total_attestations": manager.total_attestations,
        "recent_attestations": manager.history.len(),
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_hash() {
        let hash = "a".repeat(64);
        let bytes = hex::decode(&hash).unwrap();
        assert_eq!(bytes.len(), 32);
    }

    #[test]
    fn test_reject_invalid_hex() {
        let result = hex::decode("zzzz");
        assert!(result.is_err());
    }

    #[test]
    fn test_reject_wrong_length_hash() {
        let hash = "ab".repeat(16); // 16 bytes, not 32
        let bytes = hex::decode(&hash).unwrap();
        assert_ne!(bytes.len(), 32);
    }

    #[test]
    fn test_receipt_structure() {
        let receipt = NotaryReceipt {
            version: "1.0".into(),
            receipt_type: "dice-notary-receipt".into(),
            receipt_id: Uuid::new_v4().to_string(),
            attestation: AttestationData {
                content_hash: "sha256:abc123".into(),
                hash_algorithm: "sha256".into(),
                timestamp_unix: 1712534400,
                timestamp_iso: "2026-04-07T12:00:00Z".into(),
            },
            witnesses: vec![WitnessInfo {
                device_pubkey: "02aabb".into(),
                commit_hash: "ccdd".into(),
                signature: "eeff".into(),
            }],
            witness_count: 1,
            threshold: 1,
            protocol: ProtocolInfo {
                network: "solana-devnet".into(),
                coordinator: "CoordPub".into(),
                min_witnesses: 1,
            },
            verification: VerificationInfo {
                instructions: "Verify sigs".into(),
            },
        };

        let json = serde_json::to_value(&receipt).unwrap();
        assert_eq!(json["version"], "1.0");
        assert_eq!(json["type"], "dice-notary-receipt");
        assert!(json["witnesses"].is_array());
        assert_eq!(json["witness_count"], 1);
    }

    #[test]
    fn test_notary_history_ring_buffer() {
        let mut mgr = NotaryManager {
            history: VecDeque::with_capacity(MAX_HISTORY),
            total_attestations: 0,
        };

        for _ in 0..(MAX_HISTORY + 5) {
            if mgr.history.len() >= MAX_HISTORY {
                mgr.history.pop_front();
            }
            mgr.history.push_back(NotaryReceipt {
                version: "1.0".into(),
                receipt_type: "dice-notary-receipt".into(),
                receipt_id: Uuid::new_v4().to_string(),
                attestation: AttestationData {
                    content_hash: "sha256:test".into(),
                    hash_algorithm: "sha256".into(),
                    timestamp_unix: 0,
                    timestamp_iso: "".into(),
                },
                witnesses: vec![],
                witness_count: 0,
                threshold: 0,
                protocol: ProtocolInfo {
                    network: "test".into(),
                    coordinator: "test".into(),
                    min_witnesses: 0,
                },
                verification: VerificationInfo {
                    instructions: "test".into(),
                },
            });
            mgr.total_attestations += 1;
        }

        assert_eq!(mgr.history.len(), MAX_HISTORY);
    }
}
