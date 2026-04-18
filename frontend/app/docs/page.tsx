import Link from "next/link"
import type { Metadata } from "next"
import { DocsBreadcrumbs } from "@/components/docs/Breadcrumbs"
import { H1, Lead, P } from "@/components/docs/Prose"

export const metadata: Metadata = {
  title: "Welcome to DICE Docs",
  description:
    "Everything you need to integrate hardware-backed verifiable randomness on Solana.",
}

const PATHS = [
  {
    label: "I'm new here",
    title: "Start with the introduction",
    href: "/docs/introduction",
    desc: "What DICE is, why hardware matters, and how the trust model works.",
  },
  {
    label: "I want to ship, now",
    title: "Jump to the quickstart",
    href: "/docs/quickstart",
    desc: "Three changes. One CPI call. Working randomness in ~60 seconds of reading.",
  },
  {
    label: "I care about how it works",
    title: "Read the architecture",
    href: "/docs/architecture",
    desc: "ESP32 devices, commit-reveal, and the Solana program that settles every round.",
  },
  {
    label: "I need an API reference",
    title: "Browse the reference",
    href: "/docs/reference",
    desc: "Payload layout, selection modes, pricing, and the full error enum.",
  },
]

const HIGHLIGHTS = [
  { value: "4.0 s", label: "Avg round latency", sub: "v7.7 streaming · 50-round bench" },
  { value: "0.002 SOL", label: "Per-request fee", sub: "Flat · no surge pricing" },
  { value: "4–50", label: "Nodes per request", sub: "You pick size; any one honest node guarantees entropy" },
  { value: "32 B", label: "Randomness payload", sub: "Drop-in for any VRF consumer" },
]

export default function DocsHome() {
  return (
    <div className="max-w-[880px]">
      <DocsBreadcrumbs href="/docs" />
      <H1>Welcome to DICE</H1>
      <Lead>
        DICE is a hardware-backed verifiable randomness oracle for Solana. It
        replaces a cryptographic assumption with a physical one — randomness
        that comes from real ESP32-S3 silicon, committed and revealed on
        chain, delivered to your program via one CPI call.
      </Lead>

      <P>
        These docs are for Solana developers who want randomness in their dApp
        and would rather not build (or trust) an oracle themselves. Whether
        you ship a coin flip, a lottery, an NFT mint order, or a prediction
        market — the integration shape is the same. Three things change in
        your code; everything else is handled.
      </P>

      {/* Pick-your-path grid */}
      <div className="mt-10 grid grid-cols-1 gap-3 sm:grid-cols-2">
        {PATHS.map((p) => (
          <Link
            key={p.href}
            href={p.href}
            className="group flex flex-col justify-between rounded-xl border border-white/[0.08] bg-white/[0.02] p-5 transition-colors hover:border-[#00FF85]/40 hover:bg-white/[0.04]"
          >
            <span className="text-[11px] font-semibold uppercase tracking-[0.14em] text-[#00FF85]">
              {p.label}
            </span>
            <span className="mt-2 text-lg font-semibold text-white group-hover:text-[#00FF85] transition-colors">
              {p.title}
            </span>
            <span className="mt-1 text-[14px] leading-[1.55] text-zinc-400">
              {p.desc}
            </span>
            <span className="mt-4 text-[12px] text-zinc-600 group-hover:text-zinc-300">
              Read →
            </span>
          </Link>
        ))}
      </div>

      {/* Highlights strip */}
      <div className="mt-10 grid grid-cols-2 gap-3 sm:grid-cols-4">
        {HIGHLIGHTS.map((h) => (
          <div
            key={h.label}
            className="rounded-lg border border-white/[0.06] bg-white/[0.015] p-4"
          >
            <p className="font-heading text-xl font-semibold text-white">
              {h.value}
            </p>
            <p className="mt-0.5 text-[12px] font-medium text-zinc-300">
              {h.label}
            </p>
            <p className="mt-1 text-[11px] leading-snug text-zinc-500">
              {h.sub}
            </p>
          </div>
        ))}
      </div>

      {/* Program IDs */}
      <div className="mt-10 rounded-xl border border-white/[0.08] bg-white/[0.02] p-5">
        <p className="text-[11px] font-semibold uppercase tracking-[0.14em] text-zinc-500">
          On chain (v7.7 · production)
        </p>
        <div className="mt-3 flex flex-col gap-2 text-[13px]">
          <div className="flex items-center justify-between gap-3">
            <span className="text-zinc-400">dice</span>
            <code className="truncate font-mono text-[#7cf9c1]">
              FMwPuCjkfZXN2MuNJQiUzZC3hnxHcD8mrTuntsqA84XD
            </code>
          </div>
          <div className="flex items-center justify-between gap-3">
            <span className="text-zinc-400">coin_toss</span>
            <code className="truncate font-mono text-[#7cf9c1]">
              7r6UstdP6qTFK4HSqU4mFGPGyCVWd3JVjBeafQPyvspH
            </code>
          </div>
        </div>
      </div>
    </div>
  )
}
