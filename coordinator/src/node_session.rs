use std::{collections::HashMap, sync::Arc, time::Instant};

use tokio::sync::{mpsc, RwLock};
use tracing::{debug, info, warn};

/// Live state for a single connected ESP32-S3 node.
pub struct NodeSession {
    /// Compressed secp256k1 public key (33 bytes) — stable node identity.
    pub node_id: [u8; 33],
    /// Most recent reported round-trip latency in milliseconds.
    pub latency_ms: u32,
    /// Node uptime reported in last heartbeat (seconds).
    pub uptime_secs: u64,
    /// Total jobs completed reported in last heartbeat.
    pub jobs_completed: u64,
    /// Wall-clock instant of the most recently processed heartbeat.
    pub last_heartbeat: Instant,
    /// Wall-clock instant at which this session was registered.
    pub connected_at: Instant,
    /// Channel to push outbound CBOR messages to this node's WebSocket writer.
    pub tx: mpsc::Sender<Vec<u8>>,
}

/// Maximum age of the last heartbeat before a node is considered inactive.
const ACTIVE_HEARTBEAT_SECS: u64 = 45;

/// Shared, async-safe registry of all live node sessions.
pub type NodeRegistry = Arc<RwLock<HashMap<[u8; 33], NodeSession>>>;

/// Create a new, empty `NodeRegistry`.
pub fn new_registry() -> NodeRegistry {
    Arc::new(RwLock::new(HashMap::new()))
}

/// Register a newly-connected node, replacing any stale prior session.
pub async fn register(registry: &NodeRegistry, node_id: [u8; 33], tx: mpsc::Sender<Vec<u8>>) {
    let session = NodeSession {
        node_id,
        latency_ms: 0,
        uptime_secs: 0,
        jobs_completed: 0,
        last_heartbeat: Instant::now(),
        connected_at: Instant::now(),
        tx,
    };

    let mut map = registry.write().await;
    if map.contains_key(&node_id) {
        warn!(
            node = hex::encode(node_id),
            "replacing existing session for re-connecting node"
        );
    }
    map.insert(node_id, session);
    info!(
        node = hex::encode(node_id),
        total = map.len(),
        "node registered"
    );
}

/// Remove a disconnected node from the registry.
pub async fn deregister(registry: &NodeRegistry, node_id: &[u8; 33]) {
    let mut map = registry.write().await;
    if map.remove(node_id).is_some() {
        info!(
            node = hex::encode(node_id),
            remaining = map.len(),
            "node deregistered"
        );
    } else {
        debug!(
            node = hex::encode(node_id),
            "deregister called for unknown node"
        );
    }
}

/// Return the `node_id` of every node whose last heartbeat arrived within the
/// last [`ACTIVE_HEARTBEAT_SECS`] seconds.
pub async fn get_active_nodes(registry: &NodeRegistry) -> Vec<[u8; 33]> {
    let map = registry.read().await;
    map.values()
        .filter(|s| s.last_heartbeat.elapsed().as_secs() <= ACTIVE_HEARTBEAT_SECS)
        .map(|s| s.node_id)
        .collect()
}

/// Update the heartbeat statistics for a known node.
///
/// No-ops silently if the node is not in the registry (e.g. race between
/// heartbeat arrival and deregistration).
pub async fn update_heartbeat(
    registry: &NodeRegistry,
    node_id: &[u8; 33],
    latency_ms: u32,
    uptime_secs: u64,
    jobs_completed: u64,
) {
    let mut map = registry.write().await;
    if let Some(session) = map.get_mut(node_id) {
        session.latency_ms = latency_ms;
        session.uptime_secs = uptime_secs;
        session.jobs_completed = jobs_completed;
        session.last_heartbeat = Instant::now();
    } else {
        debug!(
            node = hex::encode(node_id),
            "heartbeat update for unknown node (probably disconnecting)"
        );
    }
}

/// Snapshot of a node session suitable for serialisation (no `Instant` fields).
#[derive(Debug, serde::Serialize)]
pub struct NodeInfo {
    pub node_id: String,
    pub latency_ms: u32,
    pub uptime_secs: u64,
    pub jobs_completed: u64,
    pub connected_secs: u64,
}

/// Collect [`NodeInfo`] snapshots for all currently active nodes.
pub async fn get_active_node_infos(registry: &NodeRegistry) -> Vec<NodeInfo> {
    let map = registry.read().await;
    map.values()
        .filter(|s| s.last_heartbeat.elapsed().as_secs() <= ACTIVE_HEARTBEAT_SECS)
        .map(|s| NodeInfo {
            node_id: hex::encode(s.node_id),
            latency_ms: s.latency_ms,
            uptime_secs: s.uptime_secs,
            jobs_completed: s.jobs_completed,
            connected_secs: s.connected_at.elapsed().as_secs(),
        })
        .collect()
}
