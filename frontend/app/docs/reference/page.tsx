import type { Metadata } from "next"
import Link from "next/link"
import { DocsPageShell } from "@/components/docs/PageShell"
import { H2, P } from "@/components/docs/Prose"

export const metadata: Metadata = {
  title: "Reference",
  description:
    "Payload layout, selection modes, pricing breakdown, and the full error enum.",
}

const SECTIONS = [
  {
    title: "Callback payload",
    href: "/docs/reference/payload",
    desc: "Exact 72-byte wire format that lands as your dice_callback's instruction data.",
  },
  {
    title: "Selection modes",
    href: "/docs/reference/modes",
    desc: "Streaming vs audit — who picks the devices, what it costs, when to use which.",
  },
  {
    title: "Pricing",
    href: "/docs/reference/pricing",
    desc: "0.002 SOL per round, 70/20/10 split, operator unit economics at scale.",
  },
  {
    title: "Errors",
    href: "/docs/reference/errors",
    desc: "Every DiceError variant with a plain-English root cause.",
  },
]

export default function ReferenceLanding() {
  return (
    <DocsPageShell
      href="/docs/reference"
      title="Reference"
      lead="The flat, linkable version of everything an integrator might need to grep."
    >
      <P>
        These four pages are tables and shape diagrams. You'll probably
        bookmark the errors page; you may never need to look at the payload
        layout unless you're debugging a CPI issue or building your own
        client library.
      </P>

      <H2 id="pages">Pages</H2>
      <div className="mt-6 grid grid-cols-1 gap-3 sm:grid-cols-2">
        {SECTIONS.map((s) => (
          <Link
            key={s.href}
            href={s.href}
            className="group flex flex-col rounded-xl border border-white/[0.08] bg-white/[0.02] p-5 transition-colors hover:border-[#00FF85]/40 hover:bg-white/[0.04]"
          >
            <span className="text-[15px] font-semibold text-white group-hover:text-[#00FF85] transition-colors">
              {s.title}
            </span>
            <span className="mt-1 text-[13.5px] leading-[1.55] text-zinc-400">
              {s.desc}
            </span>
          </Link>
        ))}
      </div>
    </DocsPageShell>
  )
}
