import type {
  NodesResponse,
  RoundsResponse,
  QueueResponse,
  NetworkStats,
} from "./types"

// ---------------------------------------------------------------------------
// Browser → BFF (same-origin, relative URLs).
//
// Browser code MUST NOT fetch the coord directly. All coord traffic flows:
//
//   browser → /api/v1/{nodes,rounds,queue,stats}     (Vercel route handler)
//        → ${COORD_INTERNAL_URL}/{nodes,rounds,queue,api/v1/stats}
//        → coord on Hostinger
//
// `COORD_INTERNAL_URL` lives only on the server (Node runtime) and never
// reaches the client bundle, so the coord origin stays hidden from the browser.
//
// On error we throw — React Query then surfaces the error state to the UI
// instead of returning fake data that masks outages. The `MOCK_*` constants
// previously here were a dev convenience that ended up shipping to prod and
// silently hiding real failures (see explorer "stuck on saved data" bug,
// 2026-04-25).
// ---------------------------------------------------------------------------

async function bffGet<T>(path: string): Promise<T> {
  const res = await fetch(path, {
    // Tell Next.js this is a runtime fetch — never serve cached server data
    // for the explorer. The route handler has its own short revalidate.
    cache: "no-store",
  })
  if (!res.ok) {
    throw new Error(`bff ${path} → ${res.status}`)
  }
  return res.json() as Promise<T>
}

export async function fetchNodes(): Promise<NodesResponse> {
  return bffGet<NodesResponse>("/api/v1/nodes")
}

export async function fetchRounds(): Promise<RoundsResponse> {
  return bffGet<RoundsResponse>("/api/v1/rounds")
}

export async function fetchQueue(): Promise<QueueResponse> {
  return bffGet<QueueResponse>("/api/v1/queue")
}

export async function fetchStats(): Promise<NetworkStats> {
  return bffGet<NetworkStats>("/api/v1/stats")
}
