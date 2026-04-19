"use client"

import { useState } from "react"
import { Header } from "@/components/shared/Header"
import { Footer } from "@/components/shared/Footer"
import { CornerBox } from "@/components/shared/CornerBox"
import { BracketButton, BracketLink } from "@/components/shared/BracketButton"

type FormState = {
  name: string
  email: string
  role: "operator" | "developer" | "investor" | "other" | ""
  quantity: "1" | "2-5" | "6-20" | "20+" | ""
  wallet: string
  use_case: string
  mailing_list: boolean
}

const INITIAL: FormState = {
  name: "",
  email: "",
  role: "",
  quantity: "",
  wallet: "",
  use_case: "",
  mailing_list: true,
}

export default function PreOrderPage() {
  const [form, setForm] = useState<FormState>(INITIAL)
  const [status, setStatus] = useState<"idle" | "submitting" | "ok" | "error">("idle")
  const [errorMsg, setErrorMsg] = useState<string>("")

  function update<K extends keyof FormState>(key: K, value: FormState[K]) {
    setForm((f) => ({ ...f, [key]: value }))
  }

  async function submit(e: React.FormEvent) {
    e.preventDefault()
    setStatus("submitting")
    setErrorMsg("")
    try {
      const res = await fetch("/api/v1/preorder", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(form),
      })
      if (!res.ok) {
        const body = (await res.json().catch(() => ({}))) as { error?: string }
        throw new Error(body.error ?? `HTTP ${res.status}`)
      }
      setStatus("ok")
      setForm(INITIAL)
    } catch (err) {
      setStatus("error")
      setErrorMsg(err instanceof Error ? err.message : "unknown_error")
    }
  }

  return (
    <main className="relative min-h-screen flex flex-col">
      <Header />
      <section className="flex-1 mx-auto w-full max-w-3xl px-4 sm:px-6 lg:px-8 py-16">
        <nav className="mb-6 font-mono text-xs text-muted-foreground">
          <span className="text-muted-foreground/60">$</span> cd ~ / dice / <span className="text-foreground">preorder</span>
        </nav>

        <div className="mb-10">
          <p className="ascii-label mb-4">pre-order · early access</p>
          <h1 className="text-4xl sm:text-5xl font-semibold tracking-tight">
            Reserve a hardware node.
          </h1>
          <p className="mt-5 max-w-2xl text-muted-foreground font-mono text-sm leading-relaxed">
            DICE runs on physical ESP32-S3 devices. Each unit joins the network, earns 70 %
            of the per-round fee, and ships pre-provisioned with its own secp256k1 identity
            and mTLS certificate. Fill this out and we&apos;ll reach back with provisioning
            steps, timelines, and operator terms.
          </p>
        </div>

        {status === "ok" ? (
          <SuccessCard onReset={() => setStatus("idle")} />
        ) : (
          <CornerBox title="preorder.sh" tag="v7.7" innerClassName="p-0">
            <form onSubmit={submit} className="p-6 sm:p-8 space-y-5">
              <Field label="full_name" required>
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

              <Field label="email" required>
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
                <Field label="role" required>
                  <select
                    required
                    value={form.role}
                    onChange={(e) => update("role", e.target.value as FormState["role"])}
                    className="input"
                  >
                    <option value="">— select —</option>
                    <option value="operator">Node operator</option>
                    <option value="developer">Developer / dApp builder</option>
                    <option value="investor">Investor</option>
                    <option value="other">Other / curious</option>
                  </select>
                </Field>

                <Field label="quantity" required>
                  <select
                    required
                    value={form.quantity}
                    onChange={(e) => update("quantity", e.target.value as FormState["quantity"])}
                    className="input"
                  >
                    <option value="">— how many —</option>
                    <option value="1">1 device</option>
                    <option value="2-5">2 – 5 devices</option>
                    <option value="6-20">6 – 20 devices</option>
                    <option value="20+">20+ devices</option>
                  </select>
                </Field>
              </div>

              <Field label="solana_wallet" hint="optional · for payout registration">
                <input
                  type="text"
                  maxLength={48}
                  value={form.wallet}
                  onChange={(e) => update("wallet", e.target.value)}
                  className="input font-mono text-[13px]"
                  placeholder="7xAbc…YourPubkey"
                />
              </Field>

              <Field label="use_case" hint="one or two sentences · helps prioritize">
                <textarea
                  rows={4}
                  maxLength={500}
                  value={form.use_case}
                  onChange={(e) => update("use_case", e.target.value)}
                  className="input resize-y min-h-[100px]"
                  placeholder="e.g. building an on-chain lottery that needs verifiable RNG my users can audit."
                />
              </Field>

              <label className="flex items-start gap-3 cursor-pointer">
                <input
                  type="checkbox"
                  checked={form.mailing_list}
                  onChange={(e) => update("mailing_list", e.target.checked)}
                  className="mt-1 h-4 w-4 accent-foreground"
                />
                <span className="text-sm font-mono text-muted-foreground">
                  Keep me in the loop with launch + firmware updates. No spam; unsubscribe any time.
                </span>
              </label>

              {status === "error" && (
                <p className="text-sm font-mono text-[var(--status-err)]">
                  ✕ couldn&apos;t submit: {errorMsg}. Try again or email exprmntsv2@gmail.com.
                </p>
              )}

              <div className="pt-4 flex items-center justify-between gap-4 border-t border-dashed border-border">
                <p className="text-xs font-mono text-muted-foreground">
                  We only use this to contact you about DICE. No third-party sharing.
                </p>
                <BracketButton type="submit" disabled={status === "submitting"} variant="primary">
                  {status === "submitting" ? "Submitting…" : "Reserve"}
                </BracketButton>
              </div>
            </form>
          </CornerBox>
        )}
      </section>
      <Footer />
    </main>
  )
}

function Field({
  label,
  hint,
  required,
  children,
}: {
  label: string
  hint?: string
  required?: boolean
  children: React.ReactNode
}) {
  return (
    <label className="block space-y-2">
      <span className="flex items-baseline justify-between gap-3">
        <span className="font-mono text-xs uppercase tracking-wider text-foreground">
          {label}
          {required && <span className="ml-1 text-muted-foreground">*</span>}
        </span>
        {hint && <span className="font-mono text-[11px] text-muted-foreground">{hint}</span>}
      </span>
      {children}
    </label>
  )
}

function SuccessCard({ onReset }: { onReset: () => void }) {
  return (
    <CornerBox title="reserved" tag="exit · 0">
      <div className="p-6 sm:p-8 text-center">
        <pre aria-hidden className="text-foreground/80 font-mono text-xs leading-tight mx-auto select-none">
{`╔════════════╗
║  ✓ STORED  ║
╚════════════╝`}
        </pre>
        <h2 className="mt-4 text-2xl font-semibold tracking-tight">You&apos;re on the list.</h2>
        <p className="mt-3 font-mono text-sm text-muted-foreground max-w-md mx-auto">
          We&apos;ll reach out as soon as provisioning slots open. In the meantime,
          the docs + explorer are live — have a look.
        </p>
        <div className="mt-7 flex gap-3 justify-center flex-wrap">
          <BracketButton onClick={onReset} variant="ghost">Submit_another</BracketButton>
          <BracketLink href="/docs" variant="primary">Read_docs</BracketLink>
        </div>
      </div>
    </CornerBox>
  )
}
