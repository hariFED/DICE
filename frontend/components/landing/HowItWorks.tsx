"use client"

import { motion } from "framer-motion"
import { CornerBox } from "@/components/shared/CornerBox"

const STEPS = [
  { n: "01", title: "REQUEST",       blurb: "Developer calls request_randomness() on-chain. 0.002 SOL fee auto-deducted from channel." },
  { n: "02", title: "SELECT",        blurb: "4–7 nodes trustlessly chosen via Solana SlotHashes. No coordinator manipulation possible." },
  { n: "03", title: "COMMIT-REVEAL", blurb: "Each node generates entropy from hardware TRNG, commits hash, reveals. ECDSA verifies authenticity." },
  { n: "04", title: "DELIVER",       blurb: "Combined entropy delivered via CPI callback to your program. Fully verifiable on-chain." },
]

const card = {
  hidden: { opacity: 0, y: 16 },
  visible: (i: number) => ({
    opacity: 1, y: 0,
    transition: { delay: 0.12 * i, duration: 0.5, ease: [0.25, 0.4, 0.25, 1] as [number, number, number, number] },
  }),
}

export function HowItWorks() {
  return (
    <section className="relative py-28 border-b border-border">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        {/* Section header — terminal-style banner */}
        <SectionHeader index="01" name="pipeline" lead="From request to verified randomness in under four seconds. Four atomic stages, one cross-program callback, zero coordinator trust." title="A round, end-to-end." />

        {/* Steps grid — each step is a CornerBox */}
        <div className="mt-12 grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-6">
          {STEPS.map((s, i) => (
            <motion.div
              key={s.n}
              custom={i}
              variants={card}
              initial="hidden"
              whileInView="visible"
              viewport={{ once: true, margin: "-60px" }}
            >
              <CornerBox title={`step · ${s.n}`} innerClassName="p-5 sm:p-6">
                <div className="flex items-baseline gap-3 mb-3">
                  <span className="font-mono text-3xl text-muted-foreground/50 tabular-nums">{s.n}</span>
                  <span className="ascii-label text-foreground">{s.title}</span>
                </div>
                <p className="text-sm font-mono text-muted-foreground leading-relaxed">{s.blurb}</p>
              </CornerBox>
            </motion.div>
          ))}
        </div>
      </div>
    </section>
  )
}

export function SectionHeader({
  index,
  name,
  title,
  lead,
}: {
  index: string
  name: string
  title: string
  lead: string
}) {
  return (
    <div className="grid grid-cols-1 lg:grid-cols-12 gap-8 items-start">
      {/* Big pixel chapter number */}
      <div className="lg:col-span-2">
        <p className="font-pixel text-6xl md:text-7xl text-foreground leading-none tabular-nums">
          {index}
        </p>
        <p className="mt-3 ascii-label text-[10px]">§ {name}</p>
      </div>

      <div className="lg:col-span-5 lg:col-start-4">
        <h2 className="text-4xl md:text-5xl font-medium tracking-[-0.02em] leading-[0.95]">
          {title}
        </h2>
        <div aria-hidden className="mt-5 font-mono text-xs text-muted-foreground/40 select-none whitespace-nowrap overflow-hidden">
          ═══════════════════════════════════════════════════════════
        </div>
      </div>

      <p className="lg:col-span-4 lg:col-start-9 text-muted-foreground font-mono leading-relaxed text-[13.5px]">
        {lead}
      </p>
    </div>
  )
}

export default HowItWorks
