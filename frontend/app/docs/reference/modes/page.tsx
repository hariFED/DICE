import type { Metadata } from "next"
import { DocsPageShell } from "@/components/docs/PageShell"
import { H2, H3, P, Code, Ul, Li } from "@/components/docs/Prose"
import { Callout } from "@/components/docs/Callout"

export const metadata: Metadata = {
  title: "Selection modes",
  description: "Streaming vs audit — how nodes are picked, latency trade-off, when to use each.",
}

const TOC = [
  { id: "two-modes", label: "Two modes, same output", level: 2 as const },
  { id: "comparison", label: "Comparison table", level: 2 as const },
  { id: "streaming", label: "Streaming mode", level: 2 as const },
  { id: "audit", label: "Audit mode", level: 2 as const },
  { id: "how-to-choose", label: "How to choose", level: 2 as const },
]

export default function ModesPage() {
  return (
    <DocsPageShell
      href="/docs/reference/modes"
      title="Selection modes"
      lead="Every round produces the same 32-byte result; what differs is who picks the devices. Streaming is the fast default, audit is the reproducible one."
      toc={TOC}
    >
      <H2 id="two-modes">Two modes, same output</H2>
      <P>
        DICE supports two ways to pick which devices participate in a
        round. They're interchangeable — your callback code is identical —
        but they live at opposite ends of a latency / verifiability
        trade-off.
      </P>
      <Ul>
        <Li>
          <b>Streaming</b> (default): the off-chain coordinator picks the
          subset of online devices. Fastest path end-to-end.
        </Li>
        <Li>
          <b>Audit</b>: the on-chain program picks the subset using a
          Fisher-Yates shuffle seeded by <Code>SlotHashes</Code>. Anyone can
          reproduce the selection from chain data.
        </Li>
      </Ul>

      <H2 id="comparison">Comparison table</H2>
      <div className="my-5 overflow-x-auto rounded-sm border border-border">
        <table className="w-full text-left text-[14.5px]">
          <thead className="bg-muted/40 text-foreground">
            <tr>
              <th className="px-3 py-2 font-medium">&nbsp;</th>
              <th className="px-3 py-2 font-medium">Streaming (default)</th>
              <th className="px-3 py-2 font-medium">Audit</th>
            </tr>
          </thead>
          <tbody className="text-muted-foreground">
            <tr className="border-t border-border">
              <td className="px-3 py-2 text-foreground">Who picks devices</td>
              <td className="px-3 py-2">Coordinator · off-chain</td>
              <td className="px-3 py-2">Program · on-chain Fisher-Yates</td>
            </tr>
            <tr className="border-t border-border">
              <td className="px-3 py-2 text-foreground">Seed source</td>
              <td className="px-3 py-2">Coordinator-local</td>
              <td className="px-3 py-2">
                <Code>SlotHashes</Code> sysvar
              </td>
            </tr>
            <tr className="border-t border-border">
              <td className="px-3 py-2 text-foreground">Avg round latency</td>
              <td className="px-3 py-2 font-mono text-[#7cf9c1]">4.05 s</td>
              <td className="px-3 py-2 font-mono text-[#7cf9c1]">4.15 s (+8%)</td>
            </tr>
            <tr className="border-t border-border">
              <td className="px-3 py-2 text-foreground">p95</td>
              <td className="px-3 py-2">4.62 s</td>
              <td className="px-3 py-2">4.93 s</td>
            </tr>
            <tr className="border-t border-border">
              <td className="px-3 py-2 text-foreground">Verifiable selection</td>
              <td className="px-3 py-2 text-muted-foreground/60">—</td>
              <td className="px-3 py-2 text-[var(--foreground)]">Yes · reproducible</td>
            </tr>
            <tr className="border-t border-border">
              <td className="px-3 py-2 text-foreground">Fee</td>
              <td className="px-3 py-2">0.002 SOL</td>
              <td className="px-3 py-2">0.002 SOL</td>
            </tr>
            <tr className="border-t border-border">
              <td className="px-3 py-2 text-foreground">Best for</td>
              <td className="px-3 py-2">Gaming · UX-critical</td>
              <td className="px-3 py-2">Regulated · RWA · high-stakes</td>
            </tr>
          </tbody>
        </table>
      </div>
      <P className="text-[13.5px]">
        Numbers above are real 50-round devnet benches on production
        hardware (v7.7 streaming / v7.5 audit).
      </P>

      <H2 id="streaming">Streaming mode</H2>
      <P>
        The coordinator maintains a rolling list of online devices, sorts
        them by liveness, and picks the top-<Code>n</Code> ready nodes when
        a request lands. Fast, simple, and works well because commit-reveal
        aggregation already makes the output unbiasable — <i>which</i>{" "}
        devices end up in the set doesn't matter for the security property.
      </P>
      <P>
        Streaming is the right choice for games and interactive dApps. The
        trade-off is that selection is not publicly reproducible: if a
        regulator asks "why these four devices," the answer is "whichever
        four were fastest to ack" — defensible, but not by-construction.
      </P>

      <H2 id="audit">Audit mode</H2>
      <P>
        The program reads the current{" "}
        <Code>SlotHashes</Code> sysvar as an unpredictable seed, then runs
        Fisher-Yates over the registered-device list to pick the exact{" "}
        <Code>n</Code> devices. The selection is a pure function of
        on-chain state, so anyone with a full node can replay it.
      </P>
      <P>
        This is what you use when a third party (auditor, regulator,
        insurance carrier) needs to verify <i>from chain data alone</i>{" "}
        that a specific set of devices produced a specific result. A full
        walk-through of the reproduction is on{" "}
        <a
          href="/docs/advanced/selection"
          className="text-[var(--foreground)] underline decoration-[var(--foreground)]/30 underline-offset-4"
        >
          On-chain selection
        </a>
        .
      </P>

      <Callout type="tip" title="Flip per-round">
        <p>
          You can switch modes per request by passing the selection
          preference in the client-side account list — you don't need to
          deploy a new program or run a new coordinator for it. Most teams
          start streaming and move individual high-stakes rounds to audit.
        </p>
      </Callout>

      <H2 id="how-to-choose">How to choose</H2>
      <Ul>
        <Li>
          <b>Default: streaming.</b> It's the faster, simpler answer and it
          retains the full cryptographic guarantee.
        </Li>
        <Li>
          <b>Switch to audit</b> if you need to answer an auditor's
          "reproduce it from chain data" question, if you're serving a
          regulated RWA flow, or if your users won't accept coordinator
          authority over device selection.
        </Li>
        <Li>
          <b>Don't split by stakes.</b> Decide on the property your dApp
          needs once — latency or reproducibility — and commit to it. Mixed
          modes in a single user session are surprising to end users.
        </Li>
      </Ul>
    </DocsPageShell>
  )
}
