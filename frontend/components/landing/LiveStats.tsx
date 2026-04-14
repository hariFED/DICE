"use client"

import { motion } from "framer-motion"
import { useStats } from "@/lib/hooks"
import { GlowCard } from "@/components/shared/GlowCard"
import { AnimatedCounter } from "@/components/shared/AnimatedCounter"

const fadeUp = {
  hidden: { opacity: 0, y: 30 },
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

interface StatConfig {
  label: string
  getValue: (stats: ReturnType<typeof useStats>["data"]) => number
  suffix?: string
  prefix?: string
  decimals?: number
  colorClass: string
}

// Fallbacks are 0 (not fake demo numbers) so when the coordinator is
// unreachable the parent shows an em-dash instead of a credibility-breaking
// "20 nodes / 2847 rounds / 98.7%" that has no relationship to reality.
// Cost Per Request stays at 0.002 because that's the real protocol constant,
// not a live stat.
const STATS: StatConfig[] = [
  {
    label: "Nodes Online",
    getValue: (s) => s?.nodes_online ?? 0,
    colorClass: "text-[#00FF85]",
  },
  {
    label: "Total Rounds",
    getValue: (s) => s?.total_rounds ?? 0,
    colorClass: "text-white",
  },
  {
    label: "Success Rate",
    getValue: (s) => {
      const rate = s?.success_rate ?? 0
      return rate <= 1 ? rate * 100 : rate
    },
    suffix: "%",
    decimals: 1,
    colorClass: "text-white",
  },
  {
    label: "Avg Latency",
    getValue: (s) => s?.avg_latency_ms ?? 0,
    suffix: "ms",
    colorClass: "text-white",
  },
  {
    label: "Cost Per Request",
    getValue: () => 0.002,
    suffix: " SOL",
    decimals: 3,
    colorClass: "text-[#00FF85]",
  },
]

export function LiveStats() {
  const { data: stats } = useStats()

  return (
    <section className="py-32">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        {/* Section header */}
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true, margin: "-50px" }}
          transition={{ duration: 0.6 }}
          className="text-center mb-16"
        >
          <h2 className="text-3xl md:text-5xl font-bold text-metallic">
            Live Network
          </h2>
          <p className="mt-4 text-zinc-400 text-lg">
            Real-time metrics from the DICE oracle network
          </p>
        </motion.div>

        {/* Offline indicator — shown when the coordinator API is unreachable.
            Replaces the old "fabricated fallback numbers" behavior that used
            to silently render hardcoded demo values. */}
        {!stats && (
          <div className="mb-8 flex items-center justify-center gap-2 text-sm text-zinc-500">
            <span className="inline-flex h-2 w-2 rounded-full bg-zinc-600" />
            <span>Coordinator offline — live metrics unavailable</span>
          </div>
        )}

        {/* Stats grid. When stats is undefined we render an em-dash in place
            of the animated counter for the four live metrics (Cost Per Request
            is a protocol constant and stays visible). */}
        <div className="grid grid-cols-2 md:grid-cols-5 gap-8">
          {STATS.map((stat, i) => {
            const isLive = stat.label !== "Cost Per Request"
            const showDash = isLive && !stats
            return (
              <motion.div
                key={stat.label}
                custom={i}
                variants={fadeUp}
                initial="hidden"
                whileInView="visible"
                viewport={{ once: true, margin: "-50px" }}
              >
                <GlowCard className="p-6 text-center">
                  {showDash ? (
                    <div
                      className={`text-4xl md:text-6xl font-bold ${stat.colorClass} tabular-nums`}
                    >
                      —
                    </div>
                  ) : (
                    <AnimatedCounter
                      value={stat.getValue(stats)}
                      suffix={stat.suffix}
                      prefix={stat.prefix}
                      decimals={stat.decimals}
                      className={`text-4xl md:text-6xl font-bold ${stat.colorClass}`}
                    />
                  )}
                  <p className="mt-3 text-sm uppercase tracking-wider text-zinc-500">
                    {stat.label}
                  </p>
                </GlowCard>
              </motion.div>
            )
          })}
        </div>
      </div>
    </section>
  )
}

export default LiveStats
