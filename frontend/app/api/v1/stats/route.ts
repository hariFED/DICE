import { coordGet, errorResponse, jsonResponse, sanitize } from "@/lib/bff";

// Per-route cache — next.js 16 route handlers are dynamic by default; we opt
// in to ISR-style caching with `revalidate` so many concurrent viewers don't
// stampede the coord.
export const revalidate = 3; // seconds

interface CoordStats {
  rounds_total?: number;
  active_nodes?: number;
  avg_latency_ms?: number;
  uptime_seconds?: number;
  [k: string]: unknown;
}

export async function GET(): Promise<Response> {
  try {
    const raw = await coordGet<CoordStats>("/api/v1/stats");
    return jsonResponse(sanitize(raw), { cacheSeconds: 3 });
  } catch (e) {
    return errorResponse(e);
  }
}
