"use client";

import { useState } from "react";
import { Header } from "@/components/shared/Header";
import { Footer } from "@/components/shared/Footer";
import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";

type FormState = {
  name: string;
  email: string;
  role: "operator" | "developer" | "investor" | "other" | "";
  quantity: "1" | "2-5" | "6-20" | "20+" | "";
  wallet: string;
  use_case: string;
  mailing_list: boolean;
};

const INITIAL: FormState = {
  name: "",
  email: "",
  role: "",
  quantity: "",
  wallet: "",
  use_case: "",
  mailing_list: true,
};

export default function PreOrderPage() {
  const [form, setForm] = useState<FormState>(INITIAL);
  const [status, setStatus] = useState<"idle" | "submitting" | "ok" | "error">(
    "idle",
  );
  const [errorMsg, setErrorMsg] = useState<string>("");

  function update<K extends keyof FormState>(key: K, value: FormState[K]) {
    setForm((f) => ({ ...f, [key]: value }));
  }

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    setStatus("submitting");
    setErrorMsg("");
    try {
      const res = await fetch("/api/v1/preorder", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(form),
      });
      if (!res.ok) {
        const body = (await res.json().catch(() => ({}))) as { error?: string };
        throw new Error(body.error ?? `HTTP ${res.status}`);
      }
      setStatus("ok");
      setForm(INITIAL);
    } catch (err) {
      setStatus("error");
      setErrorMsg(err instanceof Error ? err.message : "unknown_error");
    }
  }

  return (
    <main className="relative min-h-screen flex flex-col">
      <Header />
      <section className="flex-1 mx-auto w-full max-w-3xl px-4 sm:px-6 lg:px-8 py-20">
        <div className="mb-10">
          <span className="text-[11px] tracking-[0.2em] uppercase text-zinc-500 font-semibold">
            Pre-order · early access
          </span>
          <h1 className="mt-3 text-4xl sm:text-5xl font-bold tracking-tight text-white">
            Reserve a hardware node
          </h1>
          <p className="mt-4 max-w-2xl text-zinc-400 text-base leading-relaxed">
            DICE runs on physical ESP32-S3 devices. Each unit joins the network,
            earns 70 % of the per-round fee, and ships pre-provisioned with its
            own secp256k1 identity and mTLS certificate. Fill this out and we&apos;ll
            reach out with provisioning steps, timelines, and operator terms.
          </p>
        </div>

        {status === "ok" ? (
          <SuccessCard onReset={() => setStatus("idle")} />
        ) : (
          <Card className="border-white/[0.06] bg-black/40 backdrop-blur-sm">
            <form onSubmit={submit} className="p-6 sm:p-8 space-y-5">
              <Field label="Full name" required>
                <input
                  type="text"
                  required
                  maxLength={80}
                  value={form.name}
                  onChange={(e) => update("name", e.target.value)}
                  className="input"
                  placeholder="Satoshi Nakamoto"
                />
              </Field>

              <Field label="Email" required>
                <input
                  type="email"
                  required
                  maxLength={120}
                  value={form.email}
                  onChange={(e) => update("email", e.target.value)}
                  className="input"
                  placeholder="you@example.com"
                />
              </Field>

              <div className="grid grid-cols-1 sm:grid-cols-2 gap-5">
                <Field label="Your role" required>
                  <select
                    required
                    value={form.role}
                    onChange={(e) =>
                      update("role", e.target.value as FormState["role"])
                    }
                    className="input"
                  >
                    <option value="">Select one…</option>
                    <option value="operator">Node operator</option>
                    <option value="developer">Developer / dApp builder</option>
                    <option value="investor">Investor</option>
                    <option value="other">Other / curious</option>
                  </select>
                </Field>

                <Field label="Quantity interest" required>
                  <select
                    required
                    value={form.quantity}
                    onChange={(e) =>
                      update(
                        "quantity",
                        e.target.value as FormState["quantity"],
                      )
                    }
                    className="input"
                  >
                    <option value="">How many…</option>
                    <option value="1">1 device</option>
                    <option value="2-5">2–5 devices</option>
                    <option value="6-20">6–20 devices</option>
                    <option value="20+">20+ devices</option>
                  </select>
                </Field>
              </div>

              <Field label="Solana wallet" hint="Optional — for payout registration.">
                <input
                  type="text"
                  maxLength={48}
                  value={form.wallet}
                  onChange={(e) => update("wallet", e.target.value)}
                  className="input font-mono text-[13px]"
                  placeholder="7xAbc…YourPubkey"
                />
              </Field>

              <Field
                label="What do you want to build / run?"
                hint="One or two sentences. Helps us prioritise."
              >
                <textarea
                  rows={4}
                  maxLength={500}
                  value={form.use_case}
                  onChange={(e) => update("use_case", e.target.value)}
                  className="input resize-y min-h-[100px]"
                  placeholder="e.g. I'm building an on-chain lottery on Solana and need a verifiable RNG that my users can audit."
                />
              </Field>

              <label className="flex items-start gap-3 cursor-pointer">
                <input
                  type="checkbox"
                  checked={form.mailing_list}
                  onChange={(e) => update("mailing_list", e.target.checked)}
                  className="mt-1 h-4 w-4 rounded border-white/20 bg-black/40 text-[#00FF85] focus:ring-[#00FF85]/40"
                />
                <span className="text-sm text-zinc-400">
                  Keep me in the loop with launch + firmware updates. No spam;
                  unsubscribe any time.
                </span>
              </label>

              {status === "error" && (
                <p className="text-sm text-red-400">
                  Couldn&apos;t submit: {errorMsg}. Try again or email
                  exprmntsv2@gmail.com.
                </p>
              )}

              <div className="pt-3 flex items-center justify-between gap-4 border-t border-white/[0.06]">
                <p className="text-xs text-zinc-500">
                  We only use this to contact you about DICE. No third-party
                  sharing.
                </p>
                <Button
                  type="submit"
                  disabled={status === "submitting"}
                  className="bg-[#00FF85] text-black hover:bg-[#00cc6a] font-medium"
                >
                  {status === "submitting" ? "Submitting…" : "Reserve"}
                </Button>
              </div>
            </form>
          </Card>
        )}
      </section>
      <Footer />
    </main>
  );
}

function Field({
  label,
  hint,
  required,
  children,
}: {
  label: string;
  hint?: string;
  required?: boolean;
  children: React.ReactNode;
}) {
  return (
    <label className="block space-y-2">
      <span className="flex items-baseline justify-between gap-3">
        <span className="text-sm font-medium text-zinc-200">
          {label}
          {required && <span className="ml-1 text-[#00FF85]">*</span>}
        </span>
        {hint && <span className="text-xs text-zinc-500">{hint}</span>}
      </span>
      {children}
    </label>
  );
}

function SuccessCard({ onReset }: { onReset: () => void }) {
  return (
    <Card className="border-[#00FF85]/20 bg-gradient-to-b from-[#00FF85]/[0.04] to-black/40 backdrop-blur-sm">
      <div className="p-8 sm:p-10 text-center">
        <div className="mx-auto mb-4 flex h-12 w-12 items-center justify-center rounded-full border border-[#00FF85]/40 bg-[#00FF85]/10 text-[#00FF85]">
          <svg
            xmlns="http://www.w3.org/2000/svg"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2.5"
            className="h-6 w-6"
          >
            <polyline points="20 6 9 17 4 12" />
          </svg>
        </div>
        <h2 className="text-2xl font-semibold text-white">You&apos;re on the list.</h2>
        <p className="mt-3 text-sm text-zinc-400 max-w-md mx-auto">
          We&apos;ll reach out as soon as provisioning slots open. In the meantime,
          the docs + explorer are live — have a look.
        </p>
        <div className="mt-6 flex gap-3 justify-center">
          <Button
            variant="outline"
            onClick={onReset}
            className="border-white/[0.1] bg-transparent text-white hover:bg-white/[0.04]"
          >
            Submit another
          </Button>
          <a href="/docs">
            <Button className="bg-white text-black hover:bg-zinc-200">
              Read the docs
            </Button>
          </a>
        </div>
      </div>
    </Card>
  );
}
