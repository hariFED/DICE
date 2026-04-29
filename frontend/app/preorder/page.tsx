"use client"

import { useState } from "react"
import { motion, AnimatePresence } from "framer-motion"
import { Header } from "@/components/shared/Header"
import { Footer } from "@/components/shared/Footer"
import { BracketButton, BracketLink } from "@/components/shared/BracketButton"

/**
 * Pre-order flow — 4-step Stripe-style onboarding.
 *
 *   1. Who you are    (name + email)
 *   2. What you want  (role + quantity + tier)
 *   3. Where to ship  (wallet + use-case + mailing list)
 *   4. Review + submit
 *
 * Left: active step form; Right: persistent order summary.
 * Stepper row at the top shows progress.
 */

type FormState = {
  name: string
  email: string
  role: "operator" | "developer" | "investor" | "other" | ""
  quantity: number
  tier: "starter" | "pro" | "rack"
  wallet: string
  use_case: string
  mailing_list: boolean
}

const INITIAL: FormState = {
  name: "",
  email: "",
  role: "",
  quantity: 1,
  tier: "starter",
  wallet: "",
  use_case: "",
  mailing_list: true,
}

const TIERS = {
  starter: { label: "STARTER", price: 89, body: "1 node · home rack · 10-min setup" },
  pro:     { label: "PRO",     price: 249, body: "3 nodes · priority routing · analytics" },
  rack:    { label: "RACK",    price: 799, body: "10 nodes · SLA contract · white-glove bringup" },
} as const

const STEPS = [
  { id: 1, key: "contact",  label: "Contact" },
  { id: 2, key: "bundle",   label: "Bundle" },
  { id: 3, key: "delivery", label: "Delivery" },
  { id: 4, key: "review",   label: "Review" },
] as const

export default function PreOrderPage() {
  const [form, setForm] = useState<FormState>(INITIAL)
  const [step, setStep] = useState(1)
  const [status, setStatus] = useState<"idle" | "submitting" | "ok" | "error">("idle")
  const [errorMsg, setErrorMsg] = useState<string>("")

  function update<K extends keyof FormState>(key: K, value: FormState[K]) {
    setForm((f) => ({ ...f, [key]: value }))
  }

  const tier = TIERS[form.tier]
  const qty = form.quantity || 1
  const subtotal = tier.price * qty
  const shipping = 19
  const total = subtotal + shipping

  const canAdvance = {
    1: form.name.trim().length > 1 && /@/.test(form.email),
    2: form.role !== "" && qty > 0,
    3: true,
    4: true,
  }[step as 1 | 2 | 3 | 4]

  async function submit() {
    setStatus("submitting")
    setErrorMsg("")
    try {
      const res = await fetch("/api/v1/preorder", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          ...form,
          quantity: qty.toString(),
          bundle: form.tier,
          estimated_total_usd: total,
        }),
      })
      if (!res.ok) {
        const body = (await res.json().catch(() => ({}))) as { error?: string }
        throw new Error(body.error ?? `HTTP ${res.status}`)
      }
      setStatus("ok")
    } catch (err) {
      setStatus("error")
      setErrorMsg(err instanceof Error ? err.message : "unknown_error")
    }
  }

  return (
    <main className="relative min-h-screen flex flex-col">
      <Header />
      <section className="flex-1 mx-auto w-full max-w-[1280px] px-4 sm:px-6 lg:px-8 pt-28 pb-12 md:pb-16">
        {/* Header with stepper */}
        <div className="border-b border-border pb-8 mb-10">
          <div className="grid grid-cols-1 md:grid-cols-2 gap-6 items-end">
            <div>
              <p className="font-mono text-[11px] uppercase tracking-[0.2em] text-muted-foreground mb-3">
                reserve · early · access
              </p>
              <h1 className="font-sans text-4xl md:text-5xl tracking-[-0.025em] leading-[1.05] text-foreground">
                Reserve your <span className="italic font-light text-muted-foreground">DICE node.</span>
              </h1>
            </div>
            <p className="font-mono text-sm text-muted-foreground leading-relaxed max-w-md md:justify-self-end">
              No card charged today. You&apos;ll receive an invoice 2 weeks
              before your device ships. Refund any time before ship.
            </p>
          </div>

          {/* Stepper */}
          <div className="mt-10 grid grid-cols-4 gap-0">
            {STEPS.map((s) => {
              const active = s.id === step
              const done = s.id < step
              return (
                <div key={s.id} className="relative">
                  <div className="flex items-center gap-3">
                    <span
                      className={`font-pixel text-sm border flex items-center justify-center w-7 h-7 leading-none transition-all ${
                        active
                          ? "bg-foreground text-background border-foreground"
                          : done
                          ? "bg-muted text-foreground border-foreground/40"
                          : "bg-background text-muted-foreground border-border"
                      }`}
                    >
                      {done ? "✓" : `0${s.id}`}
                    </span>
                    <div>
                      <p className={`font-mono text-[10px] uppercase tracking-wider ${active ? "text-foreground" : "text-muted-foreground/60"}`}>
                        Step {s.id}
                      </p>
                      <p className={`font-pixel text-sm leading-none mt-0.5 ${active || done ? "text-foreground" : "text-muted-foreground/50"}`}>
                        {s.label}
                      </p>
                    </div>
                  </div>
                  <div className={`absolute top-3.5 left-10 right-0 border-t border-dashed ${done ? "border-foreground/60" : "border-border"} hidden md:block`} />
                </div>
              )
            })}
          </div>
        </div>

        {/* Main layout: form + summary */}
        <div className="grid grid-cols-1 lg:grid-cols-12 gap-10">
          {/* LEFT — form */}
          <div className="lg:col-span-7">
            {status === "ok" ? (
              <SuccessCard />
            ) : (
              <div className="relative min-h-[400px]">
                <AnimatePresence mode="wait">
                  <motion.div
                    key={step}
                    initial={{ opacity: 0, x: 12 }}
                    animate={{ opacity: 1, x: 0 }}
                    exit={{ opacity: 0, x: -12 }}
                    transition={{ duration: 0.22 }}
                  >
                    {step === 1 && (
                      <StepContact form={form} update={update} />
                    )}
                    {step === 2 && (
                      <StepBundle form={form} update={update} />
                    )}
                    {step === 3 && (
                      <StepDelivery form={form} update={update} />
                    )}
                    {step === 4 && (
                      <StepReview form={form} total={total} subtotal={subtotal} shipping={shipping} tierLabel={tier.label} />
                    )}
                  </motion.div>
                </AnimatePresence>

                {status === "error" && (
                  <p className="mt-5 text-sm font-mono text-[var(--status-err)]">
                    ✕ couldn&apos;t submit: {errorMsg}
                  </p>
                )}

                {/* Nav buttons */}
                <div className="mt-10 flex items-center justify-between gap-4 border-t border-dashed border-border pt-5">
                  {step > 1 ? (
                    <BracketButton variant="ghost" onClick={() => setStep((s) => s - 1)}>
                      ← Back
                    </BracketButton>
                  ) : (
                    <span />
                  )}
                  {step < 4 ? (
                    <BracketButton
                      variant="primary"
                      disabled={!canAdvance}
                      onClick={() => setStep((s) => s + 1)}
                    >
                      Next →
                    </BracketButton>
                  ) : (
                    <BracketButton
                      variant="primary"
                      disabled={status === "submitting"}
                      onClick={submit}
                    >
                      {status === "submitting" ? "Submitting…" : "Reserve"}
                    </BracketButton>
                  )}
                </div>
              </div>
            )}
          </div>

          {/* RIGHT — summary */}
          <aside className="lg:col-span-5 lg:sticky lg:top-24 lg:self-start">
            <div className="border border-border">
              <div className="p-5 border-b border-border flex items-center justify-between">
                <p className="font-pixel text-sm text-foreground">ORDER SUMMARY</p>
                <span className="font-mono text-[10px] uppercase tracking-wider text-muted-foreground">
                  not a charge
                </span>
              </div>

              <div className="p-5 space-y-4 font-mono text-[13px]">
                <Row label="Bundle"     value={tier.label} />
                <Row label="Tier price" value={`$${tier.price}`} />
                <Row label="Quantity"   value={`× ${qty}`} />
                <hr className="border-dashed border-border" />
                <Row label="Subtotal"   value={`$${subtotal}`} />
                <Row label="Shipping"   value={`$${shipping}`} hint="tracked · global" />
                <Row label="Tax"        value="— calc. at invoice" muted />
                <hr className="border-dashed border-border" />
                <div className="flex justify-between items-baseline">
                  <span className="font-pixel text-sm text-foreground">TOTAL</span>
                  <span className="font-pixel text-[32px] text-foreground leading-none">${total}</span>
                </div>
              </div>

              <div className="px-5 py-4 border-t border-border bg-muted/30 space-y-2 font-mono text-[11px] text-muted-foreground">
                <p>• Reservation holds your production slot.</p>
                <p>• Invoice sent 2 weeks pre-ship.</p>
                <p>• Refund any time before shipment.</p>
                <p>• Firmware + mTLS cert ship pre-provisioned.</p>
              </div>
            </div>

            {/* Trust row */}
            <div className="mt-6 grid grid-cols-3 gap-0 border border-border divide-x divide-border text-center">
              <div className="p-4">
                <p className="font-pixel text-sm text-foreground">SSL</p>
                <p className="font-mono text-[10px] text-muted-foreground mt-1">encrypted submit</p>
              </div>
              <div className="p-4">
                <p className="font-pixel text-sm text-foreground">0 card</p>
                <p className="font-mono text-[10px] text-muted-foreground mt-1">no charge today</p>
              </div>
              <div className="p-4">
                <p className="font-pixel text-sm text-foreground">refund</p>
                <p className="font-mono text-[10px] text-muted-foreground mt-1">until ship-date</p>
              </div>
            </div>
          </aside>
        </div>
      </section>
      <Footer />
    </main>
  )
}

// ── Steps ─────────────────────────────────────────────────────────────────

function StepContact({
  form,
  update,
}: {
  form: FormState
  update: <K extends keyof FormState>(k: K, v: FormState[K]) => void
}) {
  return (
    <div className="space-y-5">
      <header>
        <p className="font-mono text-[11px] uppercase tracking-[0.2em] text-muted-foreground mb-2">
          step · 01 · contact
        </p>
        <h2 className="font-sans text-2xl md:text-3xl tracking-[-0.02em] text-foreground">
          Who&apos;s reserving?
        </h2>
      </header>

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

      <Field label="email" required hint="we'll email your invoice here">
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
    </div>
  )
}

function StepBundle({
  form,
  update,
}: {
  form: FormState
  update: <K extends keyof FormState>(k: K, v: FormState[K]) => void
}) {
  return (
    <div className="space-y-6">
      <header>
        <p className="font-mono text-[11px] uppercase tracking-[0.2em] text-muted-foreground mb-2">
          step · 02 · bundle
        </p>
        <h2 className="font-sans text-2xl md:text-3xl tracking-[-0.02em] text-foreground">
          Pick your bundle.
        </h2>
      </header>

      {/* Tier cards */}
      <div className="space-y-3">
        {(Object.keys(TIERS) as Array<keyof typeof TIERS>).map((tkey) => {
          const t = TIERS[tkey]
          const active = form.tier === tkey
          return (
            <button
              key={tkey}
              type="button"
              onClick={() => update("tier", tkey)}
              className={`w-full text-left border p-5 transition-colors ${
                active ? "border-foreground bg-muted/40" : "border-border hover:border-muted-foreground"
              }`}
            >
              <div className="flex items-baseline justify-between mb-2">
                <div className="flex items-baseline gap-3">
                  <span
                    className={`inline-block w-3 h-3 border ${active ? "bg-foreground border-foreground" : "border-muted-foreground/50"}`}
                  />
                  <span className="font-pixel text-foreground">{t.label}</span>
                </div>
                <span className="font-pixel text-xl text-foreground">${t.price}</span>
              </div>
              <p className="ml-6 font-mono text-[12px] text-muted-foreground">{t.body}</p>
            </button>
          )
        })}
      </div>

      <div className="grid grid-cols-2 gap-5">
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

        <Field label="quantity" hint="× bundle">
          <div className="flex items-center gap-0 border border-input rounded-sm">
            <button
              type="button"
              onClick={() => update("quantity", Math.max(1, form.quantity - 1))}
              className="px-3 py-2 text-lg text-foreground hover:bg-muted border-r border-input"
            >
              −
            </button>
            <input
              type="number"
              min={1}
              max={99}
              value={form.quantity}
              onChange={(e) => update("quantity", Math.max(1, parseInt(e.target.value) || 1))}
              className="flex-1 text-center bg-background font-mono text-sm py-2 tabular-nums outline-none"
            />
            <button
              type="button"
              onClick={() => update("quantity", form.quantity + 1)}
              className="px-3 py-2 text-lg text-foreground hover:bg-muted border-l border-input"
            >
              +
            </button>
          </div>
        </Field>
      </div>
    </div>
  )
}

function StepDelivery({
  form,
  update,
}: {
  form: FormState
  update: <K extends keyof FormState>(k: K, v: FormState[K]) => void
}) {
  return (
    <div className="space-y-5">
      <header>
        <p className="font-mono text-[11px] uppercase tracking-[0.2em] text-muted-foreground mb-2">
          step · 03 · delivery
        </p>
        <h2 className="font-sans text-2xl md:text-3xl tracking-[-0.02em] text-foreground">
          A couple more details.
        </h2>
      </header>

      <Field label="solana_wallet" hint="optional · used to register operator payouts">
        <input
          type="text"
          maxLength={48}
          value={form.wallet}
          onChange={(e) => update("wallet", e.target.value)}
          className="input font-mono text-[13px]"
          placeholder="7xAbc…YourPubkey"
        />
      </Field>

      <Field label="use_case" hint="one or two sentences · helps us prioritize">
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
    </div>
  )
}

function StepReview({
  form,
  total,
  subtotal,
  shipping,
  tierLabel,
}: {
  form: FormState
  total: number
  subtotal: number
  shipping: number
  tierLabel: string
}) {
  return (
    <div className="space-y-6">
      <header>
        <p className="font-mono text-[11px] uppercase tracking-[0.2em] text-muted-foreground mb-2">
          step · 04 · review
        </p>
        <h2 className="font-sans text-2xl md:text-3xl tracking-[-0.02em] text-foreground">
          Confirm + reserve.
        </h2>
      </header>

      <div className="border border-border">
        <ReviewRow label="Name"          value={form.name} />
        <ReviewRow label="Email"         value={form.email} />
        <ReviewRow label="Role"          value={form.role || "—"} />
        <ReviewRow label="Bundle"        value={tierLabel} />
        <ReviewRow label="Quantity"      value={`× ${form.quantity}`} />
        <ReviewRow label="Wallet"        value={form.wallet || "— not provided —"} />
        <ReviewRow label="Use case"      value={form.use_case || "— not provided —"} />
        <ReviewRow label="Mailing list"  value={form.mailing_list ? "opted in" : "no"} />
      </div>

      <div className="border border-foreground bg-muted/30 p-5">
        <div className="flex items-baseline justify-between font-mono text-sm">
          <span className="text-muted-foreground">Subtotal</span>
          <span className="text-foreground">${subtotal}</span>
        </div>
        <div className="mt-1 flex items-baseline justify-between font-mono text-sm">
          <span className="text-muted-foreground">Shipping</span>
          <span className="text-foreground">${shipping}</span>
        </div>
        <hr className="my-2 border-dashed border-border" />
        <div className="flex items-baseline justify-between">
          <span className="font-pixel text-sm text-foreground">TOTAL · estimate</span>
          <span className="font-pixel text-[28px] text-foreground leading-none">${total}</span>
        </div>
        <p className="mt-3 font-mono text-[11px] text-muted-foreground leading-relaxed">
          This is an estimate. No card will be charged now. Invoice will be sent 2 weeks before shipment.
        </p>
      </div>
    </div>
  )
}

// ── Atoms ────────────────────────────────────────────────────────────────

function Row({
  label,
  value,
  hint,
  muted,
}: {
  label: string
  value: string
  hint?: string
  muted?: boolean
}) {
  return (
    <div className="flex items-baseline justify-between">
      <span className={muted ? "text-muted-foreground" : "text-muted-foreground"}>{label}</span>
      <span className={`tabular-nums ${muted ? "text-muted-foreground" : "text-foreground"}`}>
        {value}
        {hint && <span className="ml-2 text-[10px] text-muted-foreground/60">· {hint}</span>}
      </span>
    </div>
  )
}

function ReviewRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="grid grid-cols-[120px_1fr] items-start gap-4 px-5 py-3 border-b last:border-b-0 border-dashed border-border font-mono text-[13px]">
      <span className="text-muted-foreground uppercase tracking-wider text-[10px]">{label}</span>
      <span className="text-foreground break-words">{value}</span>
    </div>
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

function SuccessCard() {
  return (
    <div className="border border-foreground p-8 md:p-12 text-center">
      <pre aria-hidden className="text-foreground/80 font-mono text-xs leading-tight mx-auto select-none">
{`╔══════════════╗
║   ✓ RESERVED  ║
╚══════════════╝`}
      </pre>
      <h2 className="mt-5 font-sans text-3xl tracking-[-0.02em] text-foreground">
        You&apos;re on the list.
      </h2>
      <p className="mt-3 font-mono text-sm text-muted-foreground max-w-md mx-auto leading-relaxed">
        Check your inbox for a confirmation. We&apos;ll email you the invoice 2 weeks
        before your node ships.
      </p>
      <div className="mt-7 flex gap-3 justify-center flex-wrap">
        <BracketLink href="/docs" variant="primary">Read_docs</BracketLink>
        <BracketLink href="/explorer" variant="ghost">Explore_network</BracketLink>
      </div>
    </div>
  )
}
