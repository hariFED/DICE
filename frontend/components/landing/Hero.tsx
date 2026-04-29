"use client"

import { motion } from "framer-motion"
import { NetworkGlobe } from "@/components/landing/NetworkGlobe"
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

/**
 * Landing hero — editorial two-column.
 *
 *   left  : chapter marker · wordmark · D.I.C.E. · headline · pillars
 *   right : rotating network globe with arcs + data transfer
 */
export function Hero() {
  return (
    <section className="relative overflow-clip border-b border-border">
      {/* faint blueprint grid bg */}
      <div className="absolute inset-0 bg-grid opacity-60 pointer-events-none" aria-hidden />

      <div className="relative mx-auto w-full max-w-[1400px] px-4 sm:px-6 lg:px-8 py-14 md:pt-30 pb-14">
        {/* Main — copy left, globe right */}
        <div className="grid grid-cols-1 lg:grid-cols-12 gap-8 lg:gap-14 items-start">
          {/* LEFT — copy */}
          <div className="lg:col-span-7">
            {/* Chapter + D.I.C.E. acronym row */}
            <motion.div
              initial={{ opacity: 0, y: 8 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ duration: 0.6 }}
              className="mb-6 flex items-end gap-6"
            >
              <span className="chapter-num">01</span>
              <div className="mb-2">
                <p className="font-mono text-[10px] uppercase tracking-[0.2em] text-muted-foreground mb-1">
                  The DICE Network
                </p>
                <div className="grid grid-cols-4 gap-x-3 max-w-md font-mono text-[10px] uppercase tracking-wider">
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
                      <p className="text-muted-foreground/70 mt-0.5 text-[9px]">{word}</p>
                    </div>
                  ))}
                </div>
              </div>
            </motion.div>

            {/* Headline */}
            <motion.h1
              custom={1}
              variants={fadeUp}
              initial="hidden"
              animate="visible"
              className="font-sans text-[13vw] sm:text-[9vw] md:text-[6.5vw] lg:text-[5.2vw] xl:text-[4.6vw] font-medium tracking-[-0.03em] leading-[0.9] text-foreground"
              style={{ wordBreak: "keep-all" }}
            >
              Hardware-
              <br />
              <span className="italic font-light">backed</span>{" "}
              randomness
              <br />
              <span className="text-muted-foreground">for Solana.</span>
            </motion.h1>

            {/* Pillars */}
            <motion.ul
              custom={2}
              variants={fadeUp}
              initial="hidden"
              animate="visible"
              className="mt-10 max-w-[48ch] space-y-2.5 font-mono text-[13px] text-muted-foreground leading-[1.7]"
            >
              <li className="flex gap-3">
                <span className="font-pixel text-foreground w-6 shrink-0">01</span>
                <span>Physical ESP32 devices producing <span className="text-foreground">true entropy</span> via commit-reveal.</span>
              </li>
              <li className="flex gap-3">
                <span className="font-pixel text-foreground w-6 shrink-0">02</span>
                <span>Sub-four-second VRF rounds on <span className="text-foreground">Solana mainnet</span>.</span>
              </li>
              <li className="flex gap-3">
                <span className="font-pixel text-foreground w-6 shrink-0">03</span>
                <span><span className="text-foreground">0.002 SOL</span> per request — no token, no gatekeeping.</span>
              </li>
            </motion.ul>
          </div>

          {/* RIGHT — globe */}
          <motion.div
            initial={{ opacity: 0, scale: 0.96 }}
            animate={{ opacity: 1, scale: 1 }}
            transition={{ delay: 0.3, duration: 1, ease: [0.25, 0.4, 0.25, 1] }}
            className="relative lg:col-span-5 lg:pt-2"
          >
            <div className="flex items-baseline justify-between font-mono text-[10px] uppercase tracking-wider text-muted-foreground mb-4">
              <span>[ entropy · mesh ]</span>
              <span className="font-pixel text-foreground">20 NODES</span>
            </div>

            <div className="relative aspect-square w-full mx-auto" style={{ maxWidth: "640px" }}>
              <NetworkGlobe className="absolute inset-[-10%] w-[120%] h-[120%]" />

              {/* Axis labels */}
              <span className="absolute top-3 left-3 font-mono text-[10px] uppercase tracking-wider text-muted-foreground/60 z-10">N · 90°</span>
              <span className="absolute bottom-3 left-3 font-mono text-[10px] uppercase tracking-wider text-muted-foreground/60 z-10">S · 90°</span>
              <span className="absolute top-1/2 right-3 -translate-y-1/2 font-mono text-[10px] uppercase tracking-wider text-muted-foreground/60 z-10">180° E</span>
            </div>

            <div className="mt-4 flex items-baseline justify-between font-mono text-[10px] uppercase tracking-wider text-muted-foreground">
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
