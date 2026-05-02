import { coordGet, errorResponse, jsonResponse, sanitize } from "@/lib/bff";

export const revalidate = 3;

interface CoordQueue {
  pending?: number;
  total_dispatched?: number;
  total_queued?: number;
  total_dropped?: number;
  nodes?: Array<{
    node_id?: string;
    active_rounds?: number;
    capacity?: number;
    has_capacity?: boolean;
    [k: string]: unknown;
  }>;
  [k: string]: unknown;
}

/**
 * GET /api/v1/queue — coordinator dispatch-queue snapshot.
 *
 * Used by the explorer to surface back-pressure ("queue depth") and
 * per-node capacity. No PII; sanitizer still scrubs forbidden keys.
 */
export async function GET(): Promise<Response> {
  try {
    const raw = await coordGet<CoordQueue>("/queue");
    return jsonResponse(sanitize(raw), { cacheSeconds: 3 });
  } catch (e) {
    return errorResponse(e);
  }
}
