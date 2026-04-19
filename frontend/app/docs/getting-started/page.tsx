import Link from "next/link"
import type { Metadata } from "next"
import { DocsBreadcrumbs } from "@/components/docs/Breadcrumbs"
import { H1, H2, Lead, P } from "@/components/docs/Prose"
import { Callout } from "@/components/docs/Callout"

export const metadata: Metadata = {
  title: "Start here · what is VRF?",
  description: "A plain-language beginner guide to DICE: what verifiable randomness is, why hardware matters, and how to use it in your first dApp.",
}

export default function GettingStartedPage() {
  return (
    <div className="max-w-[880px]">
      <DocsBreadcrumbs href="/docs/getting-started" />
      <span className="inline-block font-mono text-[10px] uppercase tracking-[0.2em] text-muted-foreground border border-border rounded-sm px-2 py-1 mb-4">
        beginner · ~8 min read
      </span>
      <H1>Start here — what is VRF, and why hardware?</H1>
      <Lead>
        Before we get into code: a 3-paragraph orientation. If you already
        know what a VRF is, skip to <Link href="/docs/quickstart" className="underline underline-offset-4 decoration-muted-foreground hover:decoration-foreground">Quickstart</Link>.
      </Lead>

      <H2 id="problem">The problem, in plain words</H2>
      <P>
        Smart contracts can&apos;t generate a random number by themselves.
        Every node that validates the blockchain must compute the exact
        same state — so if you call <code className="font-mono text-foreground">Math.random()</code>, each
        node gets a different value and consensus breaks. You need
        randomness that comes from <em>outside</em> the chain but that
        nobody (including the oracle) can predict or manipulate.
      </P>
      <P>
        That&apos;s what a <strong>VRF</strong> (Verifiable Random Function)
        is for. A VRF produces a random value <em>and a proof</em> that
        the value came from a specific, committed-ahead source. If
        anyone cheats, the proof fails verification and the chain throws
        it out.
      </P>

      <VrfDiagram />

      <H2 id="hardware">Why DICE uses hardware instead of software</H2>
      <P>
        Most VRFs use a cryptographic key held by an oracle. The oracle
        claims: <q>I committed to this key, here&apos;s the signature.</q>{" "}
        That works, but it puts a lot of trust in the oracle operator —
        if they leak or misuse the key, they can bias outcomes.
      </P>
      <P>
        DICE replaces the software key with an ESP32-S3 microcontroller
        that generates random bytes from a hardware noise source inside
        the silicon. The chip then signs the value with a key it generated
        itself and never exposes. The oracle cannot leak a key it has
        never seen.
      </P>

      <HardwareVsSoftware />

      <H2 id="round">What a DICE round actually does</H2>
      <P>
        A single request plays out in four moves. The whole thing takes
        about four seconds, end to end.
      </P>

      <FourStepFlow />

      <Callout type="info">
        <strong>You do not need to understand commit-reveal in detail
        to use DICE.</strong> The SDK hides all of it behind a single{" "}
        <code className="font-mono">request_randomness</code> call. This
        section exists only so you know what you&apos;re asking for when
        you call it.
      </Callout>

      <H2 id="use-cases">What you can do with it</H2>
      <div className="mt-5 grid grid-cols-1 sm:grid-cols-3 gap-4 not-prose">
        {USE_CASES.map((u) => (
          <div key={u.title} className="border border-border p-4">
            <p className="font-pixel text-foreground text-lg leading-none">{u.num}</p>
            <p className="mt-2 font-sans text-[15px] tracking-[-0.01em] text-foreground">{u.title}</p>
            <p className="mt-1 font-mono text-[12px] leading-relaxed text-muted-foreground">{u.body}</p>
          </div>
        ))}
      </div>

      <H2 id="next">Next steps</H2>
      <div className="mt-5 space-y-2 font-mono text-sm">
        <NextLink href="/docs/getting-started/first-request" title="Walk through your first request">
          Hands-on: step-by-step with a tiny example program.
        </NextLink>
        <NextLink href="/docs/getting-started/glossary" title="Glossary">
          Skim the terms: commit, reveal, round, quorum, callback.
        </NextLink>
        <NextLink href="/docs/quickstart" title="Jump to the quickstart">
          For experienced Solana developers. Copy-paste integration.
        </NextLink>
        <NextLink href="/docs/architecture" title="Read the architecture">
          If you care about the system design.
        </NextLink>
      </div>
    </div>
  )
}

// ── page-specific visuals ─────────────────────────────────────────────────

function VrfDiagram() {
  return (
    <figure className="not-prose mt-8 mb-4 border border-border bg-muted/20">
      <div className="p-8">
        <svg viewBox="0 0 700 240" className="w-full h-auto" style={{ color: "var(--foreground)" }}>
          {/* Three boxes: dApp → VRF → chain */}
          <g fontFamily="var(--font-mono)" fontSize="11" fill="currentColor">
            {[
              { x: 60,  y: 90, label: "your dApp",    sub: "asks for a number" },
              { x: 280, y: 90, label: "VRF oracle",  sub: "rolls, signs, sends" },
              { x: 500, y: 90, label: "your dApp",   sub: "verifies, uses it" },
            ].map((b) => (
              <g key={b.label} transform={`translate(${b.x} ${b.y})`}>
                <rect width="140" height="60" fill="var(--background)" stroke="currentColor" strokeWidth="1" />
                <text x="70" y="26" textAnchor="middle" fontFamily="var(--font-pixel)" fontSize="12">{b.label}</text>
                <text x="70" y="44" textAnchor="middle" opacity="0.6">{b.sub}</text>
              </g>
            ))}
            {/* Arrows */}
            <path d="M 200 120 L 272 120" stroke="currentColor" strokeWidth="1" />
            <polygon points="268,116 276,120 268,124" fill="currentColor" />
            <path d="M 420 120 L 492 120" stroke="currentColor" strokeWidth="1" />
            <polygon points="488,116 496,120 488,124" fill="currentColor" />

            <text x="236" y="108" textAnchor="middle" opacity="0.7">request</text>
            <text x="456" y="108" textAnchor="middle" opacity="0.7">value + proof</text>
          </g>
        </svg>
      </div>
      <figcaption className="px-4 py-2 border-t border-dashed border-border font-mono text-[10px] uppercase tracking-wider text-muted-foreground">
        FIG 01 · the VRF pattern · request → signed result → on-chain verify
      </figcaption>
    </figure>
  )
}

function HardwareVsSoftware() {
  const rows = [
    ["source of randomness", "private key + HMAC",           "silicon RNG + in-chip signing"],
    ["key custody",           "oracle operator holds it",     "device holds it · never exits"],
    ["collusion risk",        "operator + validator",         "operator + ≥4 physical devices"],
    ["bias resistance",       "trust the oracle",             "commit-reveal · no single node biases"],
    ["cost per request",      "variable · subscription",      "0.002 SOL · flat"],
  ]
  return (
    <figure className="not-prose mt-6 mb-4 border border-border overflow-hidden">
      <table className="w-full font-mono text-[12.5px]">
        <thead>
          <tr className="bg-muted/40 border-b border-border">
            <th className="text-left px-4 py-2 font-pixel text-[11px] tracking-wider text-muted-foreground uppercase">dimension</th>
            <th className="text-left px-4 py-2 font-pixel text-[11px] tracking-wider text-muted-foreground uppercase">software VRF (typical)</th>
            <th className="text-left px-4 py-2 font-pixel text-[11px] tracking-wider text-foreground uppercase">DICE · hardware</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((r, i) => (
            <tr key={i} className="border-b last:border-b-0 border-dashed border-border">
              <td className="px-4 py-2.5 text-muted-foreground">{r[0]}</td>
              <td className="px-4 py-2.5 text-muted-foreground">{r[1]}</td>
              <td className="px-4 py-2.5 text-foreground">{r[2]}</td>
            </tr>
          ))}
        </tbody>
      </table>
      <figcaption className="px-4 py-2 border-t border-dashed border-border font-mono text-[10px] uppercase tracking-wider text-muted-foreground">
        FIG 02 · a side-by-side · not exhaustive
      </figcaption>
    </figure>
  )
}

function FourStepFlow() {
  const steps = [
    { n: "01", label: "request",  body: "your program calls request_randomness. 0.002 SOL charged." },
    { n: "02", label: "commit",   body: "six nodes each generate a byte, hash it, and publish the hash." },
    { n: "03", label: "reveal",   body: "once all six commits are on-chain, each node reveals its byte." },
    { n: "04", label: "finalize", body: "program XORs the bytes, signs the result, delivers to your callback." },
  ]
  return (
    <figure className="not-prose mt-6 mb-4 border border-border bg-muted/20">
      <div className="p-6">
        <div className="grid grid-cols-1 md:grid-cols-4 gap-0">
          {steps.map((s, i) => (
            <div key={s.n} className="relative p-4 border-r last:border-r-0 border-dashed border-border">
              <p className="font-pixel text-[28px] text-foreground leading-none mb-2">{s.n}</p>
              <p className="font-pixel text-sm text-foreground uppercase tracking-wider mb-2">{s.label}</p>
              <p className="font-mono text-[11px] leading-relaxed text-muted-foreground">{s.body}</p>
              {i < steps.length - 1 && (
                <span className="absolute top-1/2 -right-2 w-4 h-4 flex items-center justify-center bg-background border border-border font-mono text-[10px] text-muted-foreground">
                  →
                </span>
              )}
            </div>
          ))}
        </div>
      </div>
      <figcaption className="px-4 py-2 border-t border-dashed border-border font-mono text-[10px] uppercase tracking-wider text-muted-foreground">
        FIG 03 · one round · ~4 seconds · six nodes
      </figcaption>
    </figure>
  )
}

function NextLink({ href, title, children }: { href: string; title: string; children: React.ReactNode }) {
  return (
    <Link
      href={href}
      className="group flex items-start gap-4 border border-border p-4 hover:border-foreground transition-colors"
    >
      <span className="font-pixel text-sm text-muted-foreground group-hover:text-foreground shrink-0 pt-0.5">→</span>
      <span className="flex-1">
        <span className="block font-pixel text-sm text-foreground">{title}</span>
        <span className="block mt-1 text-[12.5px] text-muted-foreground">{children}</span>
      </span>
    </Link>
  )
}

const USE_CASES = [
  { num: "A", title: "Gaming · game outcomes",  body: "Dice rolls, card shuffles, raid-boss selection. Players can verify results came from hardware." },
  { num: "B", title: "DeFi · raffles & lotteries", body: "Weekly prize pools, airdrop lotteries, launchpad picks. Unbiased and auditable." },
  { num: "C", title: "NFT · trait generation",    body: "Assign rarity, pick mint order, reveal traits. Cryptographically fair." },
]
