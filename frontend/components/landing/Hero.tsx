"use client"

import Link from "next/link"
import { motion } from "framer-motion"
import { DottedSphereGlobe } from "@/components/landing/DottedSphereGlobe"
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

/**
 * Landing hero — device-first / passive-income pivot.
 *
 *   left  : chapter marker · wordmark · D.I.C.E. · headline · pillars · CTAs
 *   right : rotating dotted sphere · node strip
 *
 * Marketing direction (2026-04-25): lead with the device, not the protocol.
 * Mining-rig energy: "buy the box, plug in, earn while it lives." No specific
 * numbers in the hero — those drift, invite arguments, and overpromise.
 * Concrete latency / fee / success-rate metrics live in /explorer where they
 * update against the real network.
 */
export function Hero() {
  return (
    <section className="relative overflow-hidden border-b border-border">
      {/* faint blueprint grid bg */}
      <div className="absolute inset-0 bg-grid opacity-60 pointer-events-none" aria-hidden />

      <div className="relative mx-auto w-full max-w-[1400px] px-4 sm:px-6 lg:px-8 py-14 md:py-20">
        {/* Top meta strip */}
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{ duration: 0.4 }}
          className="mb-12 md:mb-20 grid grid-cols-2 md:grid-cols-12 gap-4 items-baseline font-mono text-[11px] uppercase tracking-wider text-muted-foreground border-b border-dashed border-border pb-4"
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
              Mine VRF.
              <br />
              <span className="italic font-light">While you</span>{" "}
              sleep.
            </motion.h1>

            {/* Subline */}
            <motion.p
              custom={2}
              variants={fadeUp}
              initial="hidden"
              animate="visible"
              className="mt-8 max-w-[52ch] font-mono text-[14px] md:text-[15px] text-muted-foreground leading-[1.65]"
            >
              A real <span className="text-foreground">box on your shelf</span> mining verifiable randomness for Solana.
              One-time purchase. No fans. No diminishing returns. No electricity tax.
            </motion.p>

            {/* Pillars */}
            <motion.ul
              custom={3}
              variants={fadeUp}
              initial="hidden"
              animate="visible"
              className="mt-10 max-w-[48ch] space-y-2.5 font-mono text-[13px] text-muted-foreground leading-[1.7]"
            >
              <li className="flex gap-3">
                <span className="font-pixel text-foreground w-6 shrink-0">01</span>
                <span>
                  <span className="text-foreground">Real hardware</span> on your shelf — an ESP32 device drawing
                  true entropy from physical noise, not a software seed.
                </span>
              </li>
              <li className="flex gap-3">
                <span className="font-pixel text-foreground w-6 shrink-0">02</span>
                <span>
                  Your node <span className="text-foreground">earns from every randomness request</span> it helps
                  fulfill — paid out on-chain to a wallet you control.
                </span>
              </li>
              <li className="flex gap-3">
                <span className="font-pixel text-foreground w-6 shrink-0">03</span>
                <span>
                  <span className="text-foreground">Plug in once.</span> No staking, no token gates, no firmware
                  tinkering. Earn until the device dies.
                </span>
              </li>
            </motion.ul>

            {/* CTAs — pre-order leads */}
            <motion.div
              custom={4}
              variants={fadeUp}
              initial="hidden"
              animate="visible"
              className="mt-12 flex flex-wrap items-center gap-3"
            >
              <BracketLink href="/preorder" variant="primary">Pre-order_Your_Node</BracketLink>
              <BracketLink href="/docs" variant="ghost">How_It_Earns</BracketLink>
              <Link
                href="/explorer"
                className="ml-2 font-mono text-[11px] uppercase tracking-wider text-muted-foreground hover:text-foreground transition-colors border-b border-dashed border-border hover:border-foreground"
              >
                or see the network live →
              </Link>
            </motion.div>
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
              <span className="font-pixel text-foreground">LIVE</span>
            </div>

            <div className="relative aspect-square w-full max-w-[560px] mx-auto corner-ticks">
              <DottedSphereGlobe size={560} className="w-full h-auto" />

              {/* Axis labels */}
              <span className="absolute top-3 left-3 font-mono text-[10px] uppercase tracking-wider text-muted-foreground/60">N · 90°</span>
              <span className="absolute bottom-3 left-3 font-mono text-[10px] uppercase tracking-wider text-muted-foreground/60">S · 90°</span>
              <span className="absolute top-1/2 right-3 -translate-y-1/2 font-mono text-[10px] uppercase tracking-wider text-muted-foreground/60">180° E</span>
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
