import type { Metadata } from "next"
import Link from "next/link"
import { DocsPageShell } from "@/components/docs/PageShell"
import { H2, P } from "@/components/docs/Prose"

export const metadata: Metadata = {
  title: "Advanced",
  description:
    "Streaming feeds, reproducing randomness, and on-chain selection internals.",
}

const SECTIONS = [
  {
    title: "Streaming feed",
    href: "/docs/advanced/streaming",
    desc: "A PDA that publishes fresh randomness on a cadence — use it when you need randomness faster than per-round requests.",
  },
  {
    title: "Verifying randomness",
    href: "/docs/advanced/verify",
    desc: "Reproduce the 32-byte output from chain data: SHA-256 over the revealed entropy shares. One-line check.",
  },
  {
    title: "On-chain selection",
    href: "/docs/advanced/selection",
    desc: "Audit mode internals: seeded Fisher-Yates over the device list, reproducible from SlotHashes.",
  },
]

export default function AdvancedLanding() {
  return (
    <DocsPageShell
      href="/docs/advanced"
      title="Advanced"
      lead="Past the happy path. Mechanisms most integrators never need — but useful once you do."
    >
      <P>
        These pages are optional reading. You can ship a game, a mint, or a
        lottery without ever looking at them. They matter when you want to
        reproduce a result from chain data, serve randomness at a higher
        rate than the per-round protocol, or satisfy an auditor who wants
        to replay device selection.
      </P>

      <H2 id="pages">Pages</H2>
      <div className="mt-6 grid grid-cols-1 gap-3 sm:grid-cols-2">
        {SECTIONS.map((s) => (
          <Link
            key={s.href}
            href={s.href}
            className="group flex flex-col rounded-sm border border-border bg-muted/30 p-5 transition-colors hover:border-border hover:bg-muted/40"
          >
            <span className="text-[15px] font-semibold text-foreground group-hover:text-[var(--foreground)] transition-colors">
              {s.title}
            </span>
            <span className="mt-1 text-[13.5px] leading-[1.55] text-muted-foreground">
              {s.desc}
            </span>
          </Link>
        ))}
      </div>
    </DocsPageShell>
  )
}
