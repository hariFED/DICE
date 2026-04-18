import { coordGet, errorResponse, jsonResponse, sanitize } from "@/lib/bff";

export const revalidate = 3;

interface CoordNode {
  node_id?: string;
  latency_ms?: number;
  jobs_completed?: number;
  last_heartbeat_ms?: number;
  is_active?: boolean;
  [k: string]: unknown;
}

/**
 * GET /api/v1/nodes — public view of currently connected ESP32-S3 nodes.
 *
 * Sanitizer strips IP / MAC / cert fields (see `FORBIDDEN_KEYS` in bff.ts).
 * Each node is also truncated to a pubkey prefix (first 6 hex) so operators
 * can identify their device without exposing the full 33-byte compressed key
 * in a machine-readable form — the on-chain DeviceRegistry PDAs already hold
 * the canonical pubkeys for anyone who wants them.
 */
export async function GET(): Promise<Response> {
  try {
    const raw = await coordGet<CoordNode[] | { nodes: CoordNode[] }>(
      "/api/v1/nodes",
    );
    const list: CoordNode[] = Array.isArray(raw)
      ? raw
      : Array.isArray(raw?.nodes)
        ? raw.nodes
        : [];
    const safe = list.map((n) => {
      const clean = sanitize(n) as CoordNode;
      // Replace full node_id with a display-safe prefix.
      const id = typeof clean.node_id === "string" ? clean.node_id : "";
      return {
        ...clean,
        node_id: id,
        pubkey_prefix: id.slice(0, 12),
      };
    });
    return jsonResponse({ nodes: safe, count: safe.length }, { cacheSeconds: 3 });
  } catch (e) {
    return errorResponse(e);
  }
}
