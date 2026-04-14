export interface Node {
  node_id: string
  latency_ms: number
  uptime_secs: number
  jobs_completed: number
  connected_secs: number
}

export interface NodesResponse {
  nodes: Node[]
  count: number
}

export interface Round {
  request_id: string
  status:
    | "collecting_commits"
    | "collecting_reveals"
    | "finalized"
    | "failed"
  node_count: number
  commits_received: number
  reveals_received: number
  randomness: string | null
  elapsed_ms: number
  timestamp: number
}

export interface RoundsResponse {
  rounds: Round[]
}

export interface RoundDetail {
  id: string
  request_id: string
  status: string
  randomness: string | null
  created_at: string
  finalized_at: string | null
}

export interface QueueResponse {
  pending: number
  total_dispatched: number
  total_queued: number
  total_dropped: number
  nodes: QueueNode[]
}

export interface QueueNode {
  node_id: string
  active_rounds: number
  capacity: number
  has_capacity: boolean
}

export interface NetworkStats {
  nodes_online: number
  nodes_registered: number
  total_rounds: number
  success_rate: number
  avg_latency_ms: number
  queue_depth: number
}
