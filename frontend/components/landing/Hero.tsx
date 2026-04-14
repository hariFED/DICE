"use client"

import { motion } from "framer-motion"
import dynamic from "next/dynamic"
import Link from "next/link"
import { useStats } from "@/lib/hooks"

const Globe = dynamic(() => import("@/components/three/Globe"), { ssr: false })

// 20 node locations distributed globally — simulating the DICE network
const NODE_LOCATIONS: Array<{ location: [number, number]; size: number }> = [
  { location: [37.7749, -122.4194], size: 0.06 },   // San Francisco
  { location: [40.7128, -74.006], size: 0.06 },      // New York
  { location: [51.5074, -0.1278], size: 0.06 },      // London
  { location: [48.8566, 2.3522], size: 0.05 },       // Paris
  { location: [52.52, 13.405], size: 0.05 },         // Berlin
  { location: [35.6762, 139.6503], size: 0.06 },     // Tokyo
  { location: [1.3521, 103.8198], size: 0.06 },      // Singapore
  { location: [22.3193, 114.1694], size: 0.05 },     // Hong Kong
  { location: [-33.8688, 151.2093], size: 0.05 },    // Sydney
  { location: [55.7558, 37.6173], size: 0.05 },      // Moscow
  { location: [25.2048, 55.2708], size: 0.05 },      // Dubai
  { location: [19.076, 72.8777], size: 0.05 },       // Mumbai
  { location: [-23.5505, -46.6333], size: 0.05 },    // Sao Paulo
  { location: [43.6532, -79.3832], size: 0.05 },     // Toronto
  { location: [47.6062, -122.3321], size: 0.04 },    // Seattle
  { location: [34.0522, -118.2437], size: 0.04 },    // Los Angeles
  { location: [50.0755, 14.4378], size: 0.04 },      // Prague
  { location: [59.3293, 18.0686], size: 0.04 },      // Stockholm
  { location: [-34.6037, -58.3816], size: 0.04 },    // Buenos Aires
  { location: [28.6139, 77.209], size: 0.04 },       // New Delhi
]

const fadeUp = {
  hidden: { opacity: 0, y: 30 },
  visible: (i: number) => ({
    opacity: 1,
    y: 0,
    transition: {
      delay: 0.15 * i,
      duration: 0.6,
      ease: [0.25, 0.4, 0.25, 1] as [number, number, number, number],
    },
  }),
}

export function Hero() {
  const { data: stats } = useStats()

  // Show an em-dash when the coordinator is unreachable rather than fabricated
  // numbers. Hardcoded demo values would be a credibility bug — visitors who
  // know how many nodes are actually online would catch it immediately.
  const hasStats = stats !== undefined
  const nodesOnlineDisplay = hasStats ? String(stats.nodes_online) : "—"
  const totalRoundsDisplay = hasStats ? stats.total_rounds.toLocaleString() : "—"
  const successRateDisplay = (() => {
    if (!hasStats) return "—"
    const rate = stats.success_rate
    if (typeof rate !== "number") return "—"
    const pct = rate <= 1 ? rate * 100 : rate
    return `${pct.toFixed(1)}%`
  })()

  return (
    <section className="relative min-h-screen overflow-hidden flex items-center">
      {/* Background globe — right side, oversized */}
      <div className="absolute inset-0 flex items-center justify-end pointer-events-none select-none">
        <div
          className="relative opacity-60"
          style={{
            width: "min(800px, 90vw)",
            height: "min(800px, 90vw)",
            marginRight: "-10%",
          }}
        >
          <Globe markers={NODE_LOCATIONS} />
        </div>
      </div>

      {/* Subtle radial gradient overlay for depth */}
      <div className="absolute inset-0 bg-[radial-gradient(ellipse_at_70%_50%,rgba(0,255,133,0.04)_0%,transparent_60%)] pointer-events-none" />

      {/* Foreground content */}
      <div className="relative z-10 mx-auto w-full max-w-7xl px-4 sm:px-6 lg:px-8 py-32 md:py-40">
        <div className="max-w-2xl">
          {/* Badge */}
          <motion.div
            custom={0}
            variants={fadeUp}
            initial="hidden"
            animate="visible"
          >
            <span className="inline-flex items-center gap-2 rounded-full border border-white/10 bg-white/[0.03] px-4 py-1.5 text-sm text-zinc-300 backdrop-blur-sm">
              <span className="relative flex h-2 w-2">
                <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-[#00FF85] opacity-75" />
                <span className="relative inline-flex h-2 w-2 rounded-full bg-[#00FF85]" />
              </span>
              Live on Solana Devnet
            </span>
          </motion.div>

          {/* Headline */}
          <motion.h1
            custom={1}
            variants={fadeUp}
            initial="hidden"
            animate="visible"
            className="mt-8 text-5xl md:text-7xl font-bold tracking-tight leading-[1.05]"
          >
            <span className="text-metallic block">Hardware-Backed</span>
            <span className="text-metallic block">Verifiable Randomness</span>
          </motion.h1>

          {/* Subtitle */}
          <motion.p
            custom={2}
            variants={fadeUp}
            initial="hidden"
            animate="visible"
            className="mt-6 max-w-lg text-lg text-zinc-400 leading-relaxed"
          >
            20 physical ESP32 devices generating true entropy via commit-reveal
            protocol. 0.002 SOL per request.
          </motion.p>

          {/* Stats row */}
          <motion.div
            custom={3}
            variants={fadeUp}
            initial="hidden"
            animate="visible"
            className="mt-8 flex flex-wrap items-center gap-3"
          >
            <div className="flex items-center gap-2 rounded-lg border border-white/[0.08] bg-white/[0.03] px-4 py-2">
              <span className="text-sm font-semibold text-[#00FF85] tabular-nums">
                {nodesOnlineDisplay}
              </span>
              <span className="text-sm text-zinc-500">Nodes Online</span>
            </div>
            <div className="flex items-center gap-2 rounded-lg border border-white/[0.08] bg-white/[0.03] px-4 py-2">
              <span className="text-sm font-semibold text-white tabular-nums">
                {totalRoundsDisplay}
              </span>
              <span className="text-sm text-zinc-500">Rounds Completed</span>
            </div>
            <div className="flex items-center gap-2 rounded-lg border border-white/[0.08] bg-white/[0.03] px-4 py-2">
              <span className="text-sm font-semibold text-white tabular-nums">
                {successRateDisplay}
              </span>
              <span className="text-sm text-zinc-500">Success Rate</span>
            </div>
          </motion.div>

          {/* CTA buttons. "Read Docs" button removed until a public docs
              site exists — shipping a "#" link would be a visible bug. */}
          <motion.div
            custom={4}
            variants={fadeUp}
            initial="hidden"
            animate="visible"
            className="mt-10 flex flex-wrap items-center gap-4"
          >
            <Link
              href="/explorer"
              className="inline-flex items-center justify-center rounded-full bg-[#00FF85] px-6 py-3 text-sm font-semibold text-black transition-all duration-200 hover:bg-[#00cc6a] hover:shadow-[0_0_30px_rgba(0,255,133,0.25)]"
            >
              Explore Network
            </Link>
          </motion.div>
        </div>
      </div>
    </section>
  )
}

export default Hero
