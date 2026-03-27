use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Individual message structs
// ---------------------------------------------------------------------------

/// Periodic heartbeat sent by a node to signal liveness and report stats.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Heartbeat {
    /// Compressed secp256k1 public key (33 bytes)
    pub node_id: Vec<u8>,
    /// Round-trip latency in milliseconds
    pub latency_ms: u64,
    /// Node uptime in seconds
    pub uptime_secs: u64,
    /// Total randomness jobs completed by this node
    pub jobs_completed: u64,
    /// Unix timestamp of this heartbeat (seconds)
    pub timestamp: u64,
}

/// Assignment sent by coordinator to selected nodes to kick off a round.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct JobAssignment {
    /// On-chain request identifier (32 bytes)
    pub request_id: Vec<u8>,
    /// Monotonically increasing round sequence number
    pub round_seq: u64,
    /// Unix deadline timestamp (seconds) — node must commit before this
    pub deadline_ts: u64,
}

/// Commit submitted by a node during the commit phase.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CommitSubmission {
    pub request_id: Vec<u8>,
    /// Compressed secp256k1 public key of the submitting node (33 bytes)
    pub node_id: Vec<u8>,
    /// SHA-256(entropy) — the commitment (32 bytes)
    pub commit_hash: Vec<u8>,
    /// secp256k1 ECDSA signature over commit_hash (64 bytes)
    pub signature: Vec<u8>,
}

/// Reveal submitted by a node during the reveal phase.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RevealSubmission {
    pub request_id: Vec<u8>,
    pub node_id: Vec<u8>,
    /// The raw entropy value (32 bytes)
    pub entropy: Vec<u8>,
    /// secp256k1 ECDSA signature over entropy (64 bytes)
    pub signature: Vec<u8>,
}

/// Final round outcome broadcast to all participating nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RoundResult {
    pub request_id: Vec<u8>,
    /// "finalized" | "failed"
    pub status: String,
    /// Combined randomness (32 bytes); zero-length on failure
    pub randomness: Vec<u8>,
}

// ---------------------------------------------------------------------------
// Dispatch enum
// ---------------------------------------------------------------------------

/// Top-level enum used to dispatch incoming CBOR messages.
///
/// The CBOR encoding wraps the payload in a two-element array:
///   `[<tag: text>, <payload: map>]`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum DiceMessage {
    Heartbeat(Heartbeat),
    JobAssignment(JobAssignment),
    CommitSubmission(CommitSubmission),
    RevealSubmission(RevealSubmission),
    RoundResult(RoundResult),
}

impl DiceMessage {
    /// Decode a CBOR-encoded `DiceMessage` from raw bytes.
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let value: ciborium::Value =
            ciborium::de::from_reader(bytes).context("CBOR decode failed")?;

        // Expected format: array [ tag_text, payload_map ]
        let arr = match value {
            ciborium::Value::Array(a) if a.len() == 2 => a,
            _ => return Err(anyhow!("expected CBOR array of length 2")),
        };

        let tag = match &arr[0] {
            ciborium::Value::Text(t) => t.clone(),
            _ => return Err(anyhow!("first element must be a text tag")),
        };

        // Re-encode the payload map so we can deserialise it via serde.
        let mut payload_bytes = Vec::new();
        ciborium::ser::into_writer(&arr[1], &mut payload_bytes)
            .context("re-encoding payload failed")?;

        let msg = match tag.as_str() {
            "heartbeat" => {
                let h: Heartbeat = ciborium::de::from_reader(payload_bytes.as_slice())
                    .context("deserialise Heartbeat")?;
                DiceMessage::Heartbeat(h)
            }
            "job_assignment" => {
                let j: JobAssignment = ciborium::de::from_reader(payload_bytes.as_slice())
                    .context("deserialise JobAssignment")?;
                DiceMessage::JobAssignment(j)
            }
            "commit_submission" => {
                let c: CommitSubmission = ciborium::de::from_reader(payload_bytes.as_slice())
                    .context("deserialise CommitSubmission")?;
                DiceMessage::CommitSubmission(c)
            }
            "reveal_submission" => {
                let r: RevealSubmission = ciborium::de::from_reader(payload_bytes.as_slice())
                    .context("deserialise RevealSubmission")?;
                DiceMessage::RevealSubmission(r)
            }
            "round_result" => {
                let r: RoundResult = ciborium::de::from_reader(payload_bytes.as_slice())
                    .context("deserialise RoundResult")?;
                DiceMessage::RoundResult(r)
            }
            other => return Err(anyhow!("unknown message tag: {}", other)),
        };

        Ok(msg)
    }

    /// Encode this `DiceMessage` to CBOR bytes.
    ///
    /// Produces the same `[tag, payload]` array format expected by `decode`.
    pub fn encode(&self) -> Result<Vec<u8>> {
        let (tag, payload_value) = match self {
            DiceMessage::Heartbeat(h) => (
                "heartbeat",
                ciborium::Value::serialized(h).context("serialise Heartbeat")?,
            ),
            DiceMessage::JobAssignment(j) => (
                "job_assignment",
                ciborium::Value::serialized(j).context("serialise JobAssignment")?,
            ),
            DiceMessage::CommitSubmission(c) => (
                "commit_submission",
                ciborium::Value::serialized(c).context("serialise CommitSubmission")?,
            ),
            DiceMessage::RevealSubmission(r) => (
                "reveal_submission",
                ciborium::Value::serialized(r).context("serialise RevealSubmission")?,
            ),
            DiceMessage::RoundResult(r) => (
                "round_result",
                ciborium::Value::serialized(r).context("serialise RoundResult")?,
            ),
        };

        let envelope =
            ciborium::Value::Array(vec![ciborium::Value::Text(tag.to_string()), payload_value]);

        let mut out = Vec::new();
        ciborium::ser::into_writer(&envelope, &mut out).context("CBOR encode envelope")?;
        Ok(out)
    }
}
