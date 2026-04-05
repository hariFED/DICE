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
    ///
    /// Supports two wire formats:
    ///   1. Array envelope: `["heartbeat", {"node_id": ..., ...}]` (mock nodes, SDK)
    ///   2. Integer-key map: `{0: type, 1: field, ...}` (ESP32 firmware)
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let value: ciborium::Value =
            ciborium::de::from_reader(bytes).context("CBOR decode failed")?;

        match &value {
            // Format 2: firmware integer-key map {0: type_uint, 1: ..., 2: ..., ...}
            ciborium::Value::Map(pairs) => {
                Self::decode_intkey_map(pairs)
            }
            // Format 1: array envelope ["tag", {payload}]
            ciborium::Value::Array(a) if a.len() == 2 => {
                Self::decode_array_envelope(a)
            }
            _ => Err(anyhow!("expected CBOR map or array of length 2")),
        }
    }

    /// Decode the firmware's integer-key map format.
    ///
    /// Layout: {0: type, 1: field1, 2: field2, ...}
    /// Type values: 1=heartbeat, 2=job_assign, 3=commit, 4=reveal, 5=round_result
    fn decode_intkey_map(pairs: &[(ciborium::Value, ciborium::Value)]) -> Result<Self> {
        // Build a lookup table: key_int -> &Value
        let mut fields: std::collections::HashMap<u64, &ciborium::Value> = std::collections::HashMap::new();
        for (k, v) in pairs {
            if let ciborium::Value::Integer(i) = k {
                let key: u64 = (*i).try_into().unwrap_or(999);
                fields.insert(key, v);
            }
        }

        // Key 0 is the message type
        let msg_type = fields.get(&0)
            .and_then(|v| if let ciborium::Value::Integer(i) = v { Some(i) } else { None })
            .map(|i| -> u64 { (*i).try_into().unwrap_or(0) })
            .ok_or_else(|| anyhow!("missing type field (key 0)"))?;

        // Helper closures
        let get_bytes = |key: u64| -> Result<Vec<u8>> {
            match fields.get(&key) {
                Some(ciborium::Value::Bytes(b)) => Ok(b.clone()),
                _ => Err(anyhow!("missing or invalid bytes field at key {}", key)),
            }
        };
        let get_uint = |key: u64| -> Result<u64> {
            match fields.get(&key) {
                Some(ciborium::Value::Integer(i)) => Ok((*i).try_into().unwrap_or(0)),
                _ => Err(anyhow!("missing or invalid uint field at key {}", key)),
            }
        };

        match msg_type {
            // Heartbeat: {0:0, 1:node_id, 2:latency_ms, 3:uptime_secs, 4:jobs_completed, 5:timestamp}
            0 => Ok(DiceMessage::Heartbeat(Heartbeat {
                node_id: get_bytes(1)?,
                latency_ms: get_uint(2)?,
                uptime_secs: get_uint(3)?,
                jobs_completed: get_uint(4)?,
                timestamp: get_uint(5)?,
            })),
            // CommitSubmission: {0:2, 1:request_id, 2:node_id, 3:commit_hash, 4:signature}
            2 => Ok(DiceMessage::CommitSubmission(CommitSubmission {
                request_id: get_bytes(1)?,
                node_id: get_bytes(2)?,
                commit_hash: get_bytes(3)?,
                signature: get_bytes(4)?,
            })),
            // RevealSubmission: {0:3, 1:request_id, 2:node_id, 3:entropy, 4:signature}
            3 => Ok(DiceMessage::RevealSubmission(RevealSubmission {
                request_id: get_bytes(1)?,
                node_id: get_bytes(2)?,
                entropy: get_bytes(3)?,
                signature: get_bytes(4)?,
            })),
            other => Err(anyhow!("unknown firmware message type: {}", other)),
        }
    }

    /// Decode the array envelope format: ["tag", {payload_with_text_keys}]
    fn decode_array_envelope(arr: &[ciborium::Value]) -> Result<Self> {
        let tag = match &arr[0] {
            ciborium::Value::Text(t) => t.clone(),
            _ => return Err(anyhow!("first element must be a text tag")),
        };

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
    /// Uses the firmware's integer-key map format for messages sent to devices
    /// (JobAssignment, RoundResult) and the array envelope format for others.
    pub fn encode(&self) -> Result<Vec<u8>> {
        use ciborium::Value;

        // Helper: create a CBOR integer value
        fn cbor_uint(v: u64) -> Value { Value::Integer(ciborium::value::Integer::from(v)) }

        let envelope = match self {
            // JobAssignment → firmware format: {0:1, 1:request_id, 2:round_seq, 3:deadline_ts}
            DiceMessage::JobAssignment(j) => {
                Value::Map(vec![
                    (cbor_uint(0), cbor_uint(1)),
                    (cbor_uint(1), Value::Bytes(j.request_id.clone())),
                    (cbor_uint(2), cbor_uint(j.round_seq)),
                    (cbor_uint(3), cbor_uint(j.deadline_ts)),
                ])
            }
            // RoundResult → firmware format: {0:4, 1:request_id, 2:status, 3:randomness}
            DiceMessage::RoundResult(r) => {
                Value::Map(vec![
                    (cbor_uint(0), cbor_uint(4)),
                    (cbor_uint(1), Value::Bytes(r.request_id.clone())),
                    (cbor_uint(2), Value::Text(r.status.clone())),
                    (cbor_uint(3), Value::Bytes(r.randomness.clone())),
                ])
            }
            // Other messages use array envelope format
            _ => {
                let (tag, payload_value) = match self {
                    DiceMessage::Heartbeat(h) => (
                        "heartbeat",
                        Value::serialized(h).context("serialise Heartbeat")?,
                    ),
                    DiceMessage::CommitSubmission(c) => (
                        "commit_submission",
                        Value::serialized(c).context("serialise CommitSubmission")?,
                    ),
                    DiceMessage::RevealSubmission(r) => (
                        "reveal_submission",
                        Value::serialized(r).context("serialise RevealSubmission")?,
                    ),
                    _ => unreachable!(),
                };
                Value::Array(vec![Value::Text(tag.to_string()), payload_value])
            }
        };

        let mut out = Vec::new();
        ciborium::ser::into_writer(&envelope, &mut out).context("CBOR encode")?;
        Ok(out)
    }
}
