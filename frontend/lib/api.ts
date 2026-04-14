import { API_URL } from "./constants"
import type {
  NodesResponse,
  RoundsResponse,
  QueueResponse,
  NetworkStats,
} from "./types"

// ---------------------------------------------------------------------------
// Mock data — used when the coordinator is offline
// ---------------------------------------------------------------------------

const MOCK_STATS: NetworkStats = {
  nodes_online: 14,
  nodes_registered: 20,
  total_rounds: 2847,
  success_rate: 0.987,
  avg_latency_ms: 1700,
  queue_depth: 0,
}

const MOCK_NODES: NodesResponse = {
  count: 14,
  nodes: [
    { node_id: "node-01", latency_ms: 1420, uptime_secs: 864000, jobs_completed: 312, connected_secs: 860000 },
    { node_id: "node-02", latency_ms: 1680, uptime_secs: 850200, jobs_completed: 287, connected_secs: 848000 },
    { node_id: "node-03", latency_ms: 2100, uptime_secs: 720000, jobs_completed: 198, connected_secs: 718000 },
    { node_id: "node-04", latency_ms: 1350, uptime_secs: 864000, jobs_completed: 341, connected_secs: 862000 },
    { node_id: "node-05", latency_ms: 1900, uptime_secs: 432000, jobs_completed: 156, connected_secs: 430000 },
    { node_id: "node-06", latency_ms: 1550, uptime_secs: 864000, jobs_completed: 298, connected_secs: 861000 },
    { node_id: "node-07", latency_ms: 2400, uptime_secs: 600000, jobs_completed: 143, connected_secs: 598000 },
    { node_id: "node-08", latency_ms: 1280, uptime_secs: 864000, jobs_completed: 356, connected_secs: 863000 },
    { node_id: "node-09", latency_ms: 1750, uptime_secs: 780000, jobs_completed: 224, connected_secs: 778000 },
    { node_id: "node-10", latency_ms: 1600, uptime_secs: 864000, jobs_completed: 275, connected_secs: 860000 },
    { node_id: "node-11", latency_ms: 1950, uptime_secs: 540000, jobs_completed: 167, connected_secs: 538000 },
    { node_id: "node-12", latency_ms: 1480, uptime_secs: 864000, jobs_completed: 305, connected_secs: 862000 },
    { node_id: "node-13", latency_ms: 2050, uptime_secs: 690000, jobs_completed: 189, connected_secs: 688000 },
    { node_id: "node-14", latency_ms: 1320, uptime_secs: 864000, jobs_completed: 334, connected_secs: 863000 },
  ],
}

const now = Date.now()

const MOCK_ROUNDS: RoundsResponse = {
  rounds: [
    { request_id: "req_a1b2c3d4e5f6", status: "finalized", node_count: 5, commits_received: 5, reveals_received: 5, randomness: "7f3a9c2e1d4b8f6a0e5c3d7b9a1f2e4d6c8b0a3e5f7d9c1b4a6e8f0d2c4b6a8", elapsed_ms: 1423, timestamp: now - 12000 },
    { request_id: "req_b2c3d4e5f6a7", status: "finalized", node_count: 5, commits_received: 5, reveals_received: 5, randomness: "3e8f1a2b4c6d8e0f2a4b6c8d0e2f4a6b8c0d2e4f6a8b0c2d4e6f8a0b2c4d6e8", elapsed_ms: 1687, timestamp: now - 25000 },
    { request_id: "req_c3d4e5f6a7b8", status: "collecting_reveals", node_count: 5, commits_received: 5, reveals_received: 3, randomness: null, elapsed_ms: 890, timestamp: now - 2000 },
    { request_id: "req_d4e5f6a7b8c9", status: "finalized", node_count: 5, commits_received: 5, reveals_received: 5, randomness: "a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2", elapsed_ms: 1245, timestamp: now - 38000 },
    { request_id: "req_e5f6a7b8c9d0", status: "finalized", node_count: 5, commits_received: 5, reveals_received: 5, randomness: "d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2c3d4e5", elapsed_ms: 1560, timestamp: now - 52000 },
    { request_id: "req_f6a7b8c9d0e1", status: "failed", node_count: 5, commits_received: 3, reveals_received: 0, randomness: null, elapsed_ms: 5000, timestamp: now - 65000 },
    { request_id: "req_a7b8c9d0e1f2", status: "finalized", node_count: 5, commits_received: 5, reveals_received: 5, randomness: "f0a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1", elapsed_ms: 1380, timestamp: now - 80000 },
    { request_id: "req_b8c9d0e1f2a3", status: "finalized", node_count: 5, commits_received: 5, reveals_received: 5, randomness: "c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6", elapsed_ms: 1820, timestamp: now - 95000 },
    { request_id: "req_c9d0e1f2a3b4", status: "collecting_commits", node_count: 5, commits_received: 2, reveals_received: 0, randomness: null, elapsed_ms: 340, timestamp: now - 500 },
    { request_id: "req_d0e1f2a3b4c5", status: "finalized", node_count: 5, commits_received: 5, reveals_received: 5, randomness: "e9f0a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0", elapsed_ms: 1150, timestamp: now - 110000 },
    { request_id: "req_e1f2a3b4c5d6", status: "finalized", node_count: 5, commits_received: 5, reveals_received: 5, randomness: "b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5", elapsed_ms: 1490, timestamp: now - 128000 },
    { request_id: "req_f2a3b4c5d6e7", status: "finalized", node_count: 5, commits_received: 5, reveals_received: 5, randomness: "a9b0c1d2e3f4a5b6c7d8e9f0a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0", elapsed_ms: 1630, timestamp: now - 142000 },
    { request_id: "req_a3b4c5d6e7f8", status: "failed", node_count: 5, commits_received: 4, reveals_received: 2, randomness: null, elapsed_ms: 5000, timestamp: now - 158000 },
    { request_id: "req_b4c5d6e7f8a9", status: "finalized", node_count: 5, commits_received: 5, reveals_received: 5, randomness: "1d2e3f4a5b6c7d8e9f0a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3", elapsed_ms: 1340, timestamp: now - 175000 },
    { request_id: "req_c5d6e7f8a9b0", status: "finalized", node_count: 5, commits_received: 5, reveals_received: 5, randomness: "6c7d8e9f0a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8", elapsed_ms: 1770, timestamp: now - 190000 },
    { request_id: "req_d6e7f8a9b0c1", status: "finalized", node_count: 5, commits_received: 5, reveals_received: 5, randomness: "8e9f0a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0", elapsed_ms: 1200, timestamp: now - 205000 },
    { request_id: "req_e7f8a9b0c1d2", status: "finalized", node_count: 5, commits_received: 5, reveals_received: 5, randomness: "0a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2", elapsed_ms: 1580, timestamp: now - 220000 },
    { request_id: "req_f8a9b0c1d2e3", status: "collecting_reveals", node_count: 5, commits_received: 5, reveals_received: 4, randomness: null, elapsed_ms: 1100, timestamp: now - 4000 },
    { request_id: "req_a9b0c1d2e3f4", status: "finalized", node_count: 5, commits_received: 5, reveals_received: 5, randomness: "2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2c3d4", elapsed_ms: 1450, timestamp: now - 240000 },
    { request_id: "req_b0c1d2e3f4a5", status: "finalized", node_count: 5, commits_received: 5, reveals_received: 5, randomness: "4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2c3d4e5f6", elapsed_ms: 1680, timestamp: now - 258000 },
  ],
}

const MOCK_QUEUE: QueueResponse = {
  pending: 0,
  total_dispatched: 2847,
  total_queued: 2860,
  total_dropped: 13,
  nodes: [
    { node_id: "node-01", active_rounds: 0, capacity: 3, has_capacity: true },
    { node_id: "node-02", active_rounds: 1, capacity: 3, has_capacity: true },
    { node_id: "node-04", active_rounds: 0, capacity: 3, has_capacity: true },
    { node_id: "node-06", active_rounds: 0, capacity: 3, has_capacity: true },
    { node_id: "node-08", active_rounds: 1, capacity: 3, has_capacity: true },
    { node_id: "node-10", active_rounds: 0, capacity: 3, has_capacity: true },
    { node_id: "node-12", active_rounds: 0, capacity: 3, has_capacity: true },
    { node_id: "node-14", active_rounds: 0, capacity: 3, has_capacity: true },
  ],
}

// ---------------------------------------------------------------------------
// Fetch helper — tries the real API, returns fallback on failure
// ---------------------------------------------------------------------------

async function fetchAPI<T>(path: string, fallback: T): Promise<T> {
  try {
    const res = await fetch(`${API_URL}${path}`, {
      next: { revalidate: 0 },
    })
    if (!res.ok) throw new Error(res.statusText)
    return res.json()
  } catch {
    return fallback
  }
}

// ---------------------------------------------------------------------------
// Public fetch functions
// ---------------------------------------------------------------------------

export async function fetchNodes(): Promise<NodesResponse> {
  return fetchAPI("/nodes", MOCK_NODES)
}

export async function fetchRounds(): Promise<RoundsResponse> {
  return fetchAPI("/rounds", MOCK_ROUNDS)
}

export async function fetchQueue(): Promise<QueueResponse> {
  return fetchAPI("/queue", MOCK_QUEUE)
}

export async function fetchStats(): Promise<NetworkStats> {
  return fetchAPI("/api/v1/stats", MOCK_STATS)
}
