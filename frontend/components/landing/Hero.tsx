"use client"

import { motion } from "framer-motion"
import { useStats } from "@/lib/hooks"
import { DottedWorldMap } from "@/components/landing/DottedWorldMap"
import { BracketLink } from "@/components/shared/BracketButton"
import { BRAND } from "@/lib/constants"

const fadeUp = {
  hidden: { opacity: 0, y: 18 },
  visible: (i: number) => ({
    opacity: 1,
    y: 0,
    transition: {
      delay: 0.1 * i,
      duration: 0.6,
      ease: [0.25, 0.4, 0.25, 1] as [number, number, number, number],
    },
  }),
}

export function Hero() {
  const { data: stats } = useStats()

  const latencyDisplay = stats?.avg_latency_ms != null
    ? `${(stats.avg_latency_ms / 1000).toFixed(1)}s`
    : "≈ 4s"
  const rateDisplay = (() => {
    if (!stats) return "—"
    const r = stats.success_rate
    if (typeof r !== "number") return "—"
    return `${(r <= 1 ? r * 100 : r).toFixed(1)}%`
  })()

  return (
    <section className="relative overflow-hidden border-b border-border">
      <div className="mx-auto w-full max-w-[1400px] px-4 sm:px-6 lg:px-8 py-14 md:py-20">
        {/* Meta row */}
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{ duration: 0.4 }}
          className="mb-10 md:mb-16 grid grid-cols-2 md:grid-cols-12 gap-4 items-baseline font-mono text-[11px] uppercase tracking-wider text-muted-foreground"
        >
          <span className="col-span-1 md:col-span-2 font-pixel text-foreground text-xs">00 / HERO</span>
          <span className="hidden md:flex md:col-span-3 items-center gap-1.5">
            <span className="inline-block h-1.5 w-1.5 rounded-full bg-[var(--status-ok)] animate-pulse" />
            online · devnet
          </span>
          <span className="hidden md:block md:col-span-3">
            <span className="text-muted-foreground/50">from</span>{" "}
            <span className="text-foreground">{BRAND.parent}</span>
          </span>
          <span className="hidden md:block md:col-span-4 text-right text-muted-foreground/60 font-pixel">
            {new Date().toISOString().slice(0, 16).replace("T", " ")} UTC
          </span>
        </motion.div>

        {/* Main — content left, globe right (proper circle, no mask) */}
        <div className="grid grid-cols-1 lg:grid-cols-12 gap-8 lg:gap-10 items-start">
          {/* LEFT — copy */}
          <div className="lg:col-span-6 xl:col-span-5">
            {/* Wordmark + D.I.C.E. acronym */}
            <motion.div
              initial={{ opacity: 0, y: 8 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ duration: 0.6 }}
              className="mb-7"
            >
              <p className="font-pixel text-foreground text-5xl md:text-6xl leading-none tracking-tight">
                DICE
              </p>
              <div className="mt-3 grid grid-cols-4 gap-x-3 max-w-md font-mono text-[10px] uppercase tracking-wider">
                {(
                  [
                    ["D", BRAND.acronym.D],
                    ["I", BRAND.acronym.I],
                    ["C", BRAND.acronym.C],
                    ["E", BRAND.acronym.E],
                  ] as const
                ).map(([letter, word]) => (
                  <div key={letter}>
                    <p className="font-pixel text-foreground text-sm">{letter}</p>
                    <p className="text-muted-foreground mt-0.5">{word}</p>
                  </div>
                ))}
              </div>
            </motion.div>

            {/* Editorial headline */}
            <motion.h1
              custom={1}
              variants={fadeUp}
              initial="hidden"
              animate="visible"
              className="font-sans text-[11vw] sm:text-[8vw] md:text-[5.5vw] lg:text-[4.4vw] xl:text-[4vw] font-medium tracking-[-0.025em] leading-[0.92] text-foreground"
              style={{ wordBreak: "keep-all" }}
            >
              Hardware-
              <br />
              <span className="italic font-light">backed</span>
              <br />
              randomness.
            </motion.h1>

            {/* Subtitle */}
            <motion.div
              custom={2}
              variants={fadeUp}
              initial="hidden"
              animate="visible"
              className="mt-8 max-w-[42ch] border-l-2 border-foreground/30 pl-4 font-mono text-[13px] text-muted-foreground leading-[1.7]"
            >
              <p>
                <span className="text-muted-foreground/50 mr-1.5">»</span>
                Physical ESP32 devices producing true entropy via commit-reveal.
              </p>
              <p className="mt-1.5">
                <span className="text-muted-foreground/50 mr-1.5">»</span>
                Sub-four-second VRF rounds on Solana.
              </p>
              <p className="mt-1.5">
                <span className="text-muted-foreground/50 mr-1.5">»</span>
                <span className="text-foreground">0.002 SOL</span> per request, no token.
              </p>
            </motion.div>

            {/* Dual pixel displays */}
            <motion.div
              custom={3}
              variants={fadeUp}
              initial="hidden"
              animate="visible"
              className="mt-10 grid grid-cols-2 gap-6 max-w-md"
            >
              <div className="border-t border-foreground pt-2">
                <p className="ascii-label text-[10px] mb-1">avg / round</p>
                <p className="font-pixel text-4xl md:text-5xl text-foreground tabular-nums leading-none">
                  {latencyDisplay}
                </p>
              </div>
              <div className="border-t border-foreground pt-2">
                <p className="ascii-label text-[10px] mb-1">success</p>
                <p className="font-pixel text-4xl md:text-5xl text-foreground tabular-nums leading-none">
                  {rateDisplay}
                </p>
              </div>
            </motion.div>

            {/* CTAs */}
            <motion.div
              custom={4}
              variants={fadeUp}
              initial="hidden"
              animate="visible"
              className="mt-12 flex flex-wrap items-center gap-3"
            >
              <BracketLink href="/explorer" variant="primary">Explore_Network</BracketLink>
              <BracketLink href="/docs" variant="ghost">Read_Docs</BracketLink>
            </motion.div>
          </div>

          {/* RIGHT — globe, circular, with framing labels */}
          <motion.div
            initial={{ opacity: 0, scale: 0.97 }}
            animate={{ opacity: 1, scale: 1 }}
            transition={{ delay: 0.3, duration: 1, ease: [0.25, 0.4, 0.25, 1] }}
            className="relative lg:col-span-6 xl:col-span-7 lg:pt-4"
          >
            {/* Top meta */}
            <div className="flex items-baseline justify-between font-mono text-[10px] uppercase tracking-wider text-muted-foreground mb-3">
              <span>[ entropy · mesh ]</span>
              <span className="font-pixel text-foreground">20 NODES</span>
            </div>

            {/* Dotted world map */}
            <div className="relative">
              <DottedWorldMap className="w-full" />

              {/* Subtle corner glyphs framing it */}
              <span aria-hidden className="absolute top-0 left-0 font-mono text-muted-foreground/40 text-xs leading-none select-none">╭</span>
              <span aria-hidden className="absolute top-0 right-0 font-mono text-muted-foreground/40 text-xs leading-none select-none">╮</span>
              <span aria-hidden className="absolute bottom-0 left-0 font-mono text-muted-foreground/40 text-xs leading-none select-none">╰</span>
              <span aria-hidden className="absolute bottom-0 right-0 font-mono text-muted-foreground/40 text-xs leading-none select-none">╯</span>
            </div>

            {/* Bottom caption — city codes */}
            <div className="mt-3 flex items-baseline justify-between font-mono text-[10px] uppercase tracking-wider text-muted-foreground">
              <span className="truncate">sf · nyc · ldn · par · ber · tyo · sgp · hkg · syd · dxb · bom · sao · bue · del</span>
              <span className="font-pixel text-foreground ml-3 shrink-0">GLOBAL</span>
            </div>
          </motion.div>
        </div>
      </div>
    </section>
  )
}

export default Hero
