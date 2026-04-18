import type { Metadata } from "next"
import { DocsPageShell } from "@/components/docs/PageShell"
import { H2, H3, P, Code, Ul, Li } from "@/components/docs/Prose"
import { Callout } from "@/components/docs/Callout"

export const metadata: Metadata = {
  title: "Pricing",
  description:
    "0.002 SOL per round, flat. 70/20/10 split between nodes, treasury, and reserve.",
}

const TOC = [
  { id: "flat-rate", label: "Flat rate", level: 2 as const },
  { id: "breakdown", label: "Where the 0.002 SOL goes", level: 2 as const },
  { id: "paid-atomically", label: "Paid atomically", level: 2 as const },
  { id: "operator-math", label: "Operator unit economics", level: 2 as const },
  { id: "surge-pricing", label: "No surge pricing", level: 2 as const },
]

export default function PricingPage() {
  return (
    <DocsPageShell
      href="/docs/reference/pricing"
      title="Pricing"
      lead="A flat 0.002 SOL per round, split 70/20/10 between the nodes that participated, DICE treasury, and a protocol reserve. No dynamic pricing, no per-byte charges, no hidden fees."
      toc={TOC}
    >
      <H2 id="flat-rate">Flat rate</H2>
      <P>
        Every call to <Code>request_randomness_auto</Code> debits exactly{" "}
        <Code>0.002 SOL</Code> from the calling channel's balance,
        regardless of <Code>node_count</Code>, regardless of load. A higher
        node_count gives you more hardware signatures but doesn't cost more
        — operators split the fee proportionally and accept smaller
        per-node shares when a round recruits more devices.
      </P>
      <Callout type="info" title="Why not per-node pricing?">
        <p>
          It would create incentives for callers to under-recruit and for
          operators to collude into small sets. Flat pricing decouples
          security posture (choose your <Code>node_count</Code>) from
          economics (always the same outlay).
        </p>
      </Callout>

      <H2 id="breakdown">Where the 0.002 SOL goes</H2>
      <div className="my-5 overflow-x-auto rounded-xl border border-white/[0.08]">
        <table className="w-full text-left text-[14.5px]">
          <thead className="bg-white/[0.03] text-zinc-300">
            <tr>
              <th className="px-3 py-2 font-medium">Recipient</th>
              <th className="px-3 py-2 font-medium">Share</th>
              <th className="px-3 py-2 font-medium">Per-round (lamports)</th>
              <th className="px-3 py-2 font-medium">Purpose</th>
            </tr>
          </thead>
          <tbody className="text-zinc-400">
            <tr className="border-t border-white/[0.06]">
              <td className="px-3 py-2 text-[#00FF85]">Node operators</td>
              <td className="px-3 py-2 font-mono">70 %</td>
              <td className="px-3 py-2 font-mono">1_400_000</td>
              <td className="px-3 py-2">
                Credited to node vaults. Split proportionally by participants.
              </td>
            </tr>
            <tr className="border-t border-white/[0.04]">
              <td className="px-3 py-2 text-zinc-300">Treasury</td>
              <td className="px-3 py-2 font-mono">20 %</td>
              <td className="px-3 py-2 font-mono">400_000</td>
              <td className="px-3 py-2">
                Coordinator infra, device onboarding, R&amp;D.
              </td>
            </tr>
            <tr className="border-t border-white/[0.04]">
              <td className="px-3 py-2 text-zinc-300">Reserve</td>
              <td className="px-3 py-2 font-mono">10 %</td>
              <td className="px-3 py-2 font-mono">200_000</td>
              <td className="px-3 py-2">
                Insurance, slashing coverage, future protocol votes.
              </td>
            </tr>
          </tbody>
        </table>
      </div>
      <P>
        Lamports are Solana's base unit: <Code>1 SOL = 1_000_000_000 lamports</Code>.
        So 0.002 SOL is <Code>2_000_000 lamports</Code> total.
      </P>

      <H2 id="paid-atomically">Paid atomically</H2>
      <P>
        Payouts happen <i>in the same transaction</i> that finalises your
        round. There's no separate "claim your rewards" loop for
        integrators. Operators call <Code>claim_rewards_v2</Code> against
        their vault when they want to withdraw, but the <i>crediting</i>{" "}
        part is atomic with finalisation, so a round that reverts also
        reverts the payout. You never end up in a half-paid state.
      </P>

      <H2 id="operator-math">Operator unit economics</H2>
      <P>
        Useful context if you're curious about sustainability — or if
        you're thinking about running nodes yourself.
      </P>
      <Ul>
        <Li>
          <b>Node share per round</b> — at 4 nodes per round, each operator
          earns <Code>0.00035 SOL</Code> per round (1_400_000 ÷ 4).
        </Li>
        <Li>
          <b>Break-even</b> — an ESP32-S3 costs ~$10. At $150 SOL, one node
          breaks even after ~190 rounds of participation.
        </Li>
        <Li>
          <b>Daily run-rate</b> — a network running 10,000 rounds/day
          pushes ~14 SOL/day to operator vaults, ~4 SOL to treasury, and ~2
          SOL to reserve.
        </Li>
      </Ul>
      <Callout type="tip" title="Pre-paid, not post-paid">
        <p>
          Channels hold a balance that debits per-round. This avoids
          needing a payment check on every{" "}
          <Code>request_randomness_auto</Code> call and keeps the CPI fast
          and simple. Top-ups (<Code>fund_channel</Code>) can come from any
          wallet.
        </p>
      </Callout>

      <H2 id="surge-pricing">No surge pricing</H2>
      <P>
        The protocol has no dynamic fee curve. If the network is saturated,
        you'll see higher latency (longer round-trip) but never higher
        cost. Surge pricing would create bad incentives for operators — it
        would pay more to dApps that burst and penalise steady-state users.
      </P>
    </DocsPageShell>
  )
}
