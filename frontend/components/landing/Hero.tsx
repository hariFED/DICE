"use client"

import { motion } from "framer-motion"
import dynamic from "next/dynamic"
import Link from "next/link"
import { BRAND } from "@/lib/constants"

const NetworkGlobe = dynamic(
  () =>
    import("@/components/landing/NetworkGlobe").then((mod) => ({
      default: mod.NetworkGlobe,
    })),
  {
    ssr: false,
    loading: () => <GlobePlaceholder />,
  },
)

/** CSS-only shimmer placeholder shown while the globe chunk loads. */
function GlobePlaceholder() {
  return (
    <div className="absolute inset-[-10%] w-[120%] h-[120%] flex items-center justify-center">
      <div
        className="w-[70%] aspect-square rounded-full opacity-40 animate-pulse"
        style={{
          background:
            "radial-gradient(circle at 40% 40%, rgba(255,255,255,0.08) 0%, rgba(255,255,255,0.02) 50%, transparent 70%)",
        }}
      />
    </div>
  )
}

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
 * Landing hero — device-first / passive-income pivot (Story-C framing).
 *
 *   left  : chapter marker · D.I.C.E. acronym · headline · subline · pillars · CTAs
 *   right : 3D NetworkGlobe with arcs + data transfer
 *
 * Copy direction (per `frontend/PRELAUNCH_NARRATIVE.md`):
 *   - Mine VRF. While you sleep. (mining metaphor, no specific numbers)
 *   - Pre-register CTA (no payment yet — captures intent, not money)
 *   - Pillars describe character, not metrics; concrete numbers live in /explorer
 *
 * Visual direction (merged from `ui-revamp`):
 *   - NetworkGlobe replaces DottedSphereGlobe — richer entropy-mesh story
 *   - Liquid-glass CTAs match the global glass design system
 *   - Gradient on "Solana" (Solana brand colors) anchors the chain origin
 */
export function Hero() {
  return (
    <section className="relative overflow-clip border-b border-border">
      {/* faint blueprint grid bg */}
      <div className="absolute inset-0 bg-grid opacity-60 pointer-events-none" aria-hidden />

      <div className="relative mx-auto w-full max-w-[1400px] px-4 sm:px-6 lg:px-8 pt-20 md:pt-30 pb-14">
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

            {/* Headline — Story-C: "Mine VRF. While you sleep." with a small
                Solana-gradient sub-line so the chain origin still anchors the
                hero without diluting the mining metaphor in the main line. */}
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
              <br />
              <span className="text-muted-foreground text-[60%] font-light">on </span>
              <span
                className="text-[60%] font-light bg-clip-text text-transparent"
                style={{
                  backgroundImage:
                    "linear-gradient(to right, #9945FF, #14F195)",
                }}
              >
                Solana.
              </span>
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
              No fans. No diminishing returns. No electricity tax.
            </motion.p>

            {/* Pillars — no specific numbers; concrete metrics live in /explorer */}
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

            {/* CTAs — desktop only (mobile CTAs appear below globe). Glass
                styling from ui-revamp; labels per PRELAUNCH_NARRATIVE.md
                (pre-REGISTER, not pre-order — no payment yet). */}
            <motion.div
              custom={4}
              variants={fadeUp}
              initial="hidden"
              animate="visible"
              className="mt-10 hidden lg:flex flex-wrap items-center gap-3"
            >
              <Link
                href="/preorder"
                className="liquid-glass-strong rounded-full px-5 py-2.5 font-mono text-[12px] uppercase tracking-wider text-foreground inline-flex items-center gap-2 hover:bg-white/5 transition-colors"
              >
                [ › Pre-register_your_node ]
              </Link>
              <Link
                href="/docs"
                className="liquid-glass rounded-full px-5 py-2.5 font-mono text-[12px] uppercase tracking-wider text-foreground inline-flex items-center gap-2 hover:bg-white/5 transition-colors"
              >
                [ › How_it_earns ]
              </Link>
              <Link
                href="/explorer"
                className="font-mono text-[12px] uppercase tracking-wider text-muted-foreground hover:text-foreground transition-colors ml-2"
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

          {/* CTAs — mobile only (below globe), same labels as desktop */}
          <motion.div
            custom={4}
            variants={fadeUp}
            initial="hidden"
            animate="visible"
            className="mt-6 flex lg:hidden flex-wrap items-center gap-3"
          >
            <Link
              href="/preorder"
              className="liquid-glass-strong rounded-full px-5 py-2.5 font-mono text-[12px] uppercase tracking-wider text-foreground inline-flex items-center gap-2 hover:bg-white/5 transition-colors"
            >
              [ › Pre-register_your_node ]
            </Link>
            <Link
              href="/docs"
              className="liquid-glass rounded-full px-5 py-2.5 font-mono text-[12px] uppercase tracking-wider text-foreground inline-flex items-center gap-2 hover:bg-white/5 transition-colors"
            >
              [ › How_it_earns ]
            </Link>
          </motion.div>
        </div>
      </div>
    </section>
  )
}

export default Hero
