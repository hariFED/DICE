"use client"

import Link from "next/link"
import { BracketLink } from "@/components/shared/BracketButton"

/**
 * Operator pitch — the "run a node, earn SOL" call to action.
 * Three big pixel stats + a primary CTA. Minimal copy.
 */

export function OperatorPitch() {
  return (
    <section className="relative border-b border-border bg-foreground text-background">
      <div className="mx-auto w-full max-w-[1400px] px-4 sm:px-6 lg:px-8 py-20 md:py-24">
        <div className="grid grid-cols-1 lg:grid-cols-12 gap-8 mb-12">
          <div className="lg:col-span-3">
            <span className="chapter-num" style={{ color: "var(--background)", opacity: 0.2 }}>
              09
            </span>
            <p className="mt-2 font-mono text-[11px] uppercase tracking-[0.2em] opacity-60">
              run · a · node
            </p>
          </div>
          <div className="lg:col-span-6 lg:col-start-5">
            <h2 className="font-sans text-3xl md:text-4xl lg:text-5xl tracking-[-0.02em] leading-[1.05]">
              Own a piece of the oracle. <span className="italic font-light opacity-60">Earn SOL.</span>
            </h2>
            <p className="mt-5 font-mono text-sm opacity-70 max-w-md leading-relaxed">
              No rack, no stake lockup, no governance theater. A $89 box
              on your shelf becomes a randomness node.
            </p>
          </div>
        </div>

        <div className="grid grid-cols-3 gap-4 md:gap-8 border-t border-b border-background/20 py-10 mb-10">
          <div>
            <p className="font-pixel text-[44px] md:text-[72px] leading-none">$89</p>
            <p className="mt-2 font-mono text-[11px] opacity-60 uppercase tracking-wider">
              device price · shipped
            </p>
          </div>
          <div>
            <p className="font-pixel text-[44px] md:text-[72px] leading-none">70%</p>
            <p className="mt-2 font-mono text-[11px] opacity-60 uppercase tracking-wider">
              margin at target load
            </p>
          </div>
          <div>
            <p className="font-pixel text-[44px] md:text-[72px] leading-none">0</p>
            <p className="mt-2 font-mono text-[11px] opacity-60 uppercase tracking-wider">
              stake · minimum · SOL
            </p>
          </div>
        </div>

        <div className="flex flex-wrap items-center gap-3">
          <Link
            href="/preorder"
            className="inline-flex items-center gap-2 bg-background text-foreground font-pixel text-sm px-4 py-2.5 hover:opacity-90 transition-opacity"
          >
            [ › Pre-order_node ]
          </Link>
          <BracketLink href="/docs/operator" variant="ghost" className="!text-background !border-background/40 hover:!border-background">
            Operator_docs
          </BracketLink>
        </div>
      </div>
    </section>
  )
}

export default OperatorPitch
