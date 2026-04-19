"use client"

import { motion } from "framer-motion"
import { useStats } from "@/lib/hooks"
import { AnimatedCounter } from "@/components/shared/AnimatedCounter"
import { CornerBox } from "@/components/shared/CornerBox"
import { AsciiBar } from "@/components/shared/AsciiBar"
import { SectionHeader } from "@/components/landing/HowItWorks"

const fadeUp = {
  hidden: { opacity: 0, y: 12 },
  visible: (i: number) => ({
    opacity: 1, y: 0,
    transition: { delay: 0.08 * i, duration: 0.4, ease: [0.25, 0.4, 0.25, 1] as [number, number, number, number] },
  }),
}

interface StatConfig {
  label: string
  getValue: (stats: ReturnType<typeof useStats>["data"]) => number
  suffix?: string
  prefix?: string
  decimals?: number
}

const STATS: StatConfig[] = [
  { label: "Nodes Online",     getValue: (s) => s?.nodes_online ?? 0 },
  { label: "Total Rounds",     getValue: (s) => s?.total_rounds ?? 0 },
  { label: "Success Rate",     getValue: (s) => { const r = s?.success_rate ?? 0; return r <= 1 ? r * 100 : r }, suffix: "%", decimals: 1 },
  { label: "Avg Latency",      getValue: (s) => s?.avg_latency_ms ?? 0, suffix: "ms" },
  { label: "Cost / Request",   getValue: () => 0.002, suffix: " SOL", decimals: 3 },
]

export function LiveStats() {
  const { data: stats } = useStats()

  // Compute success rate ratio for the ASCII bar.
  const successRatio = (() => {
    if (!stats) return 0
    const r = stats.success_rate
    if (typeof r !== "number") return 0
    return r <= 1 ? r : r / 100
  })()

  return (
    <section className="relative py-28 border-b border-border">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <SectionHeader
          index="04"
          name="network"
          title="Live network metrics."
          lead="Read directly from the coordinator API. Polls every five seconds. When the coordinator is unreachable, fields show — instead of fabricated demo numbers."
        />

        {/* Offline indicator */}
        {!stats && (
          <div className="mt-10 ascii-label">⊘ coordinator · offline · all live values dashed</div>
        )}

        {/* Stats — single CornerBox grid */}
        <div className="mt-10">
          <CornerBox title="readout · live · 5s_poll" tag={stats ? "online" : "offline"} innerClassName="p-0">
            <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-5 divide-x divide-y divide-border">
              {STATS.map((stat, i) => {
                const isLive = stat.label !== "Cost / Request"
                const showDash = isLive && !stats
                return (
                  <motion.div
                    key={stat.label}
                    custom={i}
                    variants={fadeUp}
                    initial="hidden"
                    whileInView="visible"
                    viewport={{ once: true, margin: "-40px" }}
                    className="p-5 md:p-6"
                  >
                    <p className="ascii-label text-[10px] mb-3">{stat.label}</p>
                    {showDash ? (
                      <span className="font-mono text-3xl md:text-4xl text-foreground tabular-nums">—</span>
                    ) : (
                      <AnimatedCounter
                        value={stat.getValue(stats)}
                        suffix={stat.suffix}
                        prefix={stat.prefix}
                        decimals={stat.decimals}
                        className="font-mono text-3xl md:text-4xl text-foreground tabular-nums"
                      />
                    )}
                  </motion.div>
                )
              })}
            </div>
          </CornerBox>

          {/* ASCII success-rate bar */}
          {stats && (
            <motion.div
              initial={{ opacity: 0 }}
              whileInView={{ opacity: 1 }}
              viewport={{ once: true }}
              transition={{ duration: 0.5, delay: 0.3 }}
              className="mt-6 font-mono text-sm flex flex-wrap items-center gap-3"
            >
              <span className="ascii-label">success</span>
              <AsciiBar value={successRatio} width={32} />
              <span className="text-foreground tabular-nums">{(successRatio * 100).toFixed(1)} %</span>
            </motion.div>
          )}
        </div>
      </div>
    </section>
  )
}

export default LiveStats
