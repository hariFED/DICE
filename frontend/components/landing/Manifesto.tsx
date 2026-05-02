"use client"

import { motion } from "framer-motion"

/**
 * Manifesto / pillar grid — 3 tiles with big numerals, short titles, body
 * copy. Editorial pacing: lots of vertical whitespace, dashed dividers,
 * pixel headings. No icons.
 */

const PILLARS = [
  {
    n: "I",
    title: "Hardware-true entropy",
    body:
      "Every byte of randomness comes from a physical ESP32-S3 RNG. No software PRNG, no mock, no fallback. The silicon generates it, the silicon signs it.",
  },
  {
    n: "II",
    title: "No token, no gatekeeping",
    body:
      "Consumers pay 0.002 SOL per request. Operators earn SOL directly. There is no DICE token. There is no allowlist. There is no governance trap.",
  },
  {
    n: "III",
    title: "Commit-reveal or nothing",
    body:
      "No node can bias the output. Every node commits a hash, reveals a preimage, and the round is aggregated on-chain. Dishonesty is provable and slashable.",
  },
]

export function Manifesto() {
  return (
    <section className="relative border-b border-border">
      <div className="mx-auto w-full max-w-[1400px] px-4 sm:px-6 lg:px-8 py-20 md:py-28">
        <div className="grid grid-cols-1 lg:grid-cols-12 gap-8 mb-14">
          <div className="lg:col-span-3">
            <span className="chapter-num">04</span>
            <p className="mt-2 font-mono text-[11px] uppercase tracking-[0.2em] text-muted-foreground">
              manifesto · three · pillars
            </p>
          </div>
          <div className="lg:col-span-6 lg:col-start-5">
            <h2 className="font-sans text-3xl md:text-4xl lg:text-5xl tracking-[-0.02em] leading-[1.05] text-foreground">
              Three things we <span className="italic font-light text-muted-foreground">won't trade.</span>
            </h2>
            <p className="mt-5 font-mono text-sm text-muted-foreground max-w-md leading-relaxed">
              Every design decision in DICE starts from these three
              constraints. If a feature violates one, it doesn't ship.
            </p>
          </div>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          {PILLARS.map((p, i) => (
            <motion.div
              key={p.n}
              initial={{ opacity: 0, y: 12 }}
              whileInView={{ opacity: 1, y: 0 }}
              viewport={{ once: true, margin: "-80px" }}
              transition={{ duration: 0.5, delay: i * 0.12 }}
              className="group relative bg-grid border border-border p-6 md:p-8 overflow-hidden"
            >
              <span className="absolute top-3 left-3 font-mono text-[10px] uppercase tracking-wider text-muted-foreground/50">
                0{i + 1} / 03
              </span>
              <p className="font-pixel text-[48px] leading-none text-foreground mt-6 mb-6">{p.n}</p>
              <h3 className="font-sans text-xl md:text-2xl tracking-[-0.01em] text-foreground mb-3">
                {p.title}
              </h3>
              <p className="font-mono text-[13px] leading-relaxed text-muted-foreground">
                {p.body}
              </p>
            </motion.div>
          ))}
        </div>
      </div>
    </section>
  )
}

export default Manifesto
