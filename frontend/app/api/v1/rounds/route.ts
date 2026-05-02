import { coordGet, errorResponse, jsonResponse, sanitize } from "@/lib/bff";

export const revalidate = 3;

interface CoordRound {
  round_id?: string | number;
  channel?: string;
  status?: string;
  duration_ms?: number;
  randomness_hex?: string;
  tx_signature?: string;
  node_count?: number;
  started_at?: string;
  finalized_at?: string;
  [k: string]: unknown;
}

/**
 * GET /api/v1/rounds?limit=N — last N finalized rounds.
 *
 * Clamped at 200 rows to protect the coord from amplification-style abuse
 * (attacker requesting limit=1_000_000 to exhaust our DB).
 */
export async function GET(req: Request): Promise<Response> {
  try {
    const url = new URL(req.url);
    const limitRaw = Number(url.searchParams.get("limit") ?? 50);
    const limit = Math.max(1, Math.min(200, Number.isFinite(limitRaw) ? limitRaw : 50));
    // Coord serves at /rounds (no /api/v1/ prefix). limit query unused
    // upstream today — coord returns a fixed-size buffer — but kept here so
    // we can plumb it through if/when coord adds pagination.
    const raw = await coordGet<CoordRound[] | { rounds: CoordRound[] }>(
      `/rounds?limit=${limit}`,
    );
    const list: CoordRound[] = Array.isArray(raw)
      ? raw
      : Array.isArray(raw?.rounds)
        ? raw.rounds
        : [];
    return jsonResponse(
      { rounds: sanitize(list), count: list.length, limit },
      { cacheSeconds: 3 },
    );
  } catch (e) {
    return errorResponse(e);
  }
}
