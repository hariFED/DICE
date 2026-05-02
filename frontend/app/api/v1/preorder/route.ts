import { jsonResponse } from "@/lib/bff";

/**
 * POST /api/v1/preorder — pre-order form submission.
 *
 * v1 strategy: log to stdout + forward to a Formspree (or similar) endpoint
 * if `PREORDER_FORWARD_URL` is set as a SERVER-ONLY env var. No database
 * required — every submission shows up in the platform logs, and the
 * operator can watch a Formspree inbox for the real storage.
 *
 * If `PREORDER_FORWARD_URL` is not set, the endpoint still accepts requests
 * and returns 200 so local dev + preview deploys don't bomb, but it only
 * prints to the console.
 */

interface PreorderBody {
  name?: string;
  email?: string;
  role?: string;
  quantity?: string;
  wallet?: string;
  use_case?: string;
  mailing_list?: boolean;
}

const ALLOWED_ROLES = new Set([
  "operator",
  "developer",
  "investor",
  "other",
]);

const ALLOWED_QUANTITIES = new Set(["1", "2-5", "6-20", "20+"]);

const EMAIL_RE = /^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$/;

function clip(input: string | undefined, max: number): string {
  return (input ?? "").trim().slice(0, max);
}

export async function POST(req: Request): Promise<Response> {
  let body: PreorderBody;
  try {
    body = (await req.json()) as PreorderBody;
  } catch {
    return jsonResponse({ error: "invalid_json" }, { status: 400 });
  }

  // ── Field validation ────────────────────────────────────────────────
  const name = clip(body.name, 80);
  const email = clip(body.email, 120);
  const role = clip(body.role, 32);
  const quantity = clip(body.quantity, 8);
  const wallet = clip(body.wallet, 48);
  const useCase = clip(body.use_case, 500);
  const mailingList = !!body.mailing_list;

  if (!name) return jsonResponse({ error: "name_required" }, { status: 400 });
  if (!email || !EMAIL_RE.test(email))
    return jsonResponse({ error: "email_invalid" }, { status: 400 });
  if (!ALLOWED_ROLES.has(role))
    return jsonResponse({ error: "role_invalid" }, { status: 400 });
  if (!ALLOWED_QUANTITIES.has(quantity))
    return jsonResponse({ error: "quantity_invalid" }, { status: 400 });

  const record = {
    submitted_at: new Date().toISOString(),
    name,
    email,
    role,
    quantity,
    wallet,
    use_case: useCase,
    mailing_list: mailingList,
  };

  console.log("[preorder]", JSON.stringify(record));

  const forward = process.env.PREORDER_FORWARD_URL;
  if (forward) {
    try {
      await fetch(forward, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(record),
      });
    } catch (e) {
      // Storage failed but user submission is still acknowledged — the log
      // above is our fallback.
      console.error("[preorder] forward failed:", e);
    }
  }

  return jsonResponse({ ok: true });
}
