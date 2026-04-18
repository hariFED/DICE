import type { Metadata } from "next"
import Link from "next/link"
import { DocsPageShell } from "@/components/docs/PageShell"
import { H2, P } from "@/components/docs/Prose"

export const metadata: Metadata = {
  title: "Integration",
  description:
    "A four-page tour through integrating DICE: channel setup, requesting, callback, formulas.",
}

const SECTIONS = [
  {
    step: "1",
    title: "Channel setup",
    href: "/docs/integration/setup",
    desc: "Create and fund the PDA that binds your program as DICE's callback target. Once per dApp, at deploy time.",
  },
  {
    step: "2",
    title: "Request randomness",
    href: "/docs/integration/request",
    desc: "The one CPI call you add inside your game instruction. Full Accounts struct included.",
  },
  {
    step: "3",
    title: "Callback handler",
    href: "/docs/integration/callback",
    desc: "Define DiceResult, declare the accounts, write the outcome line. Game-specific logic lives here.",
  },
  {
    step: "4",
    title: "Outcome formulas",
    href: "/docs/integration/formulas",
    desc: "Five copy-paste formulas — dice, coin, wheel, lottery, threshold — that cover 90% of VRF use-cases.",
  },
]

export default function IntegrationLanding() {
  return (
    <DocsPageShell
      href="/docs/integration"
      title="Integration"
      lead="Four short pages, in order. Read them top-to-bottom and you'll ship."
    >
      <P>
        The integration is the same shape regardless of what you're
        building. A channel binds DICE to your program at deploy time, then
        every player instruction kicks off a round with a single CPI, and
        DICE calls back with 32 bytes when the hardware is done. The only
        game-specific code is your outcome formula and your own game-state
        account.
      </P>

      <H2 id="pages">Pages</H2>

      <div className="mt-6 grid grid-cols-1 gap-3 sm:grid-cols-2">
        {SECTIONS.map((s) => (
          <Link
            key={s.href}
            href={s.href}
            className="group flex gap-4 rounded-xl border border-white/[0.08] bg-white/[0.02] p-5 transition-colors hover:border-[#00FF85]/40 hover:bg-white/[0.04]"
          >
            <span className="flex h-8 w-8 shrink-0 items-center justify-center rounded-full border border-[#00FF85]/40 text-[13px] font-semibold text-[#00FF85]">
              {s.step}
            </span>
            <span className="flex flex-col">
              <span className="text-[15px] font-semibold text-white group-hover:text-[#00FF85] transition-colors">
                {s.title}
              </span>
              <span className="mt-1 text-[13.5px] leading-[1.55] text-zinc-400">
                {s.desc}
              </span>
            </span>
          </Link>
        ))}
      </div>
    </DocsPageShell>
  )
}
