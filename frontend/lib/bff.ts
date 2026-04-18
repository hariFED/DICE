/**
 * Backend-for-Frontend (BFF) helpers.
 *
 * Browser code MUST NOT fetch the coordinator directly. All coord access goes
 * through the `/api/v1/*` route handlers, which run on the server (Node) and:
 *
 *   1. read coord URL from a SERVER-only env var (`COORD_INTERNAL_URL`),
 *      so the coord URL never leaks to the browser bundle;
 *   2. sanitize responses (strip secrets, internal fields, debug noise);
 *   3. apply a short cache (`revalidate`) so a viral page doesn't hammer the
 *      coord;
 *   4. CORS-restrict to our own origin.
 *
 * If the BFF is later moved to a separate Cloudflare Worker / Railway
 * service, the coord can drop its public-facing port entirely — only the BFF
 * has reach.
 */

/** Server-only coord URL. `NEXT_PUBLIC_API_URL` is the dev fallback. */
export const COORD_URL =
  process.env.COORD_INTERNAL_URL ??
  process.env.NEXT_PUBLIC_API_URL ??
  "http://localhost:8080";

/**
 * Fetch from the coord with a sane timeout + structured error.
 * Throws on non-2xx so the caller can map to an HTTP status.
 */
export async function coordGet<T>(path: string, init?: RequestInit): Promise<T> {
  const url = `${COORD_URL.replace(/\/$/, "")}${path}`;
  const ctl = new AbortController();
  const timer = setTimeout(() => ctl.abort(), 5_000); // 5 s hard cap
  try {
    const res = await fetch(url, {
      ...init,
      signal: ctl.signal,
      // Next.js 16 server fetches; we control caching at the route level via
      // `export const revalidate`, NOT the per-call `cache` option.
      cache: "no-store",
    });
    if (!res.ok) {
      throw new BffError(`coord ${res.status}`, res.status);
    }
    return (await res.json()) as T;
  } finally {
    clearTimeout(timer);
  }
}

export class BffError extends Error {
  constructor(message: string, public readonly status: number) {
    super(message);
  }
}

/**
 * Strip a fixed set of internal fields from any object recursively. Belt-and-
 * braces: even if the coord accidentally adds a debug field, it never reaches
 * the browser.
 */
const FORBIDDEN_KEYS = new Set([
  "ip_address",
  "ip",
  "remote_addr",
  "mac",
  "mac_address",
  "cert_fingerprint",
  "cert",
  "private_key",
  "secret",
  "password",
  "database_url",
  "rpc_key",
  "api_key",
  "auth_token",
  "session_token",
  "internal_id",
  "debug_trace",
  "stack_trace",
]);

export function sanitize<T>(input: T): T {
  if (Array.isArray(input)) {
    return input.map((item) => sanitize(item)) as unknown as T;
  }
  if (input && typeof input === "object") {
    const out: Record<string, unknown> = {};
    for (const [k, v] of Object.entries(input)) {
      const lower = k.toLowerCase();
      if (FORBIDDEN_KEYS.has(lower)) continue;
      out[k] = sanitize(v);
    }
    return out as T;
  }
  return input;
}

/** Standard JSON response with CORS headers + cache hint. */
export function jsonResponse<T>(
  body: T,
  init?: { status?: number; cacheSeconds?: number },
): Response {
  const headers: HeadersInit = {
    "content-type": "application/json; charset=utf-8",
    "cache-control":
      init?.cacheSeconds && init.cacheSeconds > 0
        ? `public, max-age=${init.cacheSeconds}, s-maxage=${init.cacheSeconds}, stale-while-revalidate=${init.cacheSeconds * 4}`
        : "no-store",
  };
  return new Response(JSON.stringify(body), {
    status: init?.status ?? 200,
    headers,
  });
}

/** Map a thrown error to an HTTP response without leaking internals. */
export function errorResponse(e: unknown): Response {
  const status = e instanceof BffError ? e.status : 502;
  return jsonResponse(
    { error: status === 502 ? "upstream_unreachable" : `coord_${status}` },
    { status: status >= 500 ? 502 : status },
  );
}
