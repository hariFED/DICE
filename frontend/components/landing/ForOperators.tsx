"use client"

import dynamic from "next/dynamic"
import { motion } from "framer-motion"
import { GlowCard } from "@/components/shared/GlowCard"

const ESP32Scene = dynamic(
  () =>
    import("@/components/three/ESP32Scene").then((m) => ({
      default: m.ESP32Scene,
    })),
  { ssr: false },
)

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

const FEATURES = [
  {
    title: "$8 Device",
    description:
      "ESP32-S3 microcontroller. Plug in, connect WiFi, start earning.",
  },
  {
    title: "70% Revenue Share",
    description:
      "Node operators receive 70% of every 0.002 SOL request fee.",
  },
  {
    title: "Zero Maintenance",
    description:
      "Once set up, your node runs autonomously. No staking, no tokens required.",
  },
]

export function ForOperators() {
  return (
    <section>
      {/* ── Part 1: Scroll-driven 3D ESP32 showcase ── */}
      <ESP32Scene />

      {/* ── Part 2: Operator CTA section ── */}
      <div className="py-32">
        <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-16 items-start">
            {/* Left column: Text + features */}
            <div>
              <motion.p
                custom={0}
                variants={fadeUp}
                initial="hidden"
                whileInView="visible"
                viewport={{ once: true, margin: "-50px" }}
                className="text-xs uppercase tracking-[0.2em] text-[#00FF85]"
              >
                For Node Operators
              </motion.p>

              <motion.h2
                custom={1}
                variants={fadeUp}
                initial="hidden"
                whileInView="visible"
                viewport={{ once: true, margin: "-50px" }}
                className="mt-4 text-4xl md:text-5xl font-bold text-metallic"
              >
                Earn Passive Income
              </motion.h2>

              <motion.p
                custom={2}
                variants={fadeUp}
                initial="hidden"
                whileInView="visible"
                viewport={{ once: true, margin: "-50px" }}
                className="mt-4 text-zinc-400 text-lg leading-relaxed max-w-lg"
              >
                Run a DICE node on commodity hardware and earn SOL for every
                randomness request your device fulfills. No cloud servers, no
                staking minimums, no lock-ups.
              </motion.p>

              {/* Feature cards */}
              <div className="mt-10 flex flex-col gap-4">
                {FEATURES.map((feature, i) => (
                  <motion.div
                    key={feature.title}
                    custom={i + 3}
                    variants={fadeUp}
                    initial="hidden"
                    whileInView="visible"
                    viewport={{ once: true, margin: "-50px" }}
                  >
                    <GlowCard className="p-5">
                      <h3 className="text-white font-semibold">
                        {feature.title}
                      </h3>
                      <p className="mt-1 text-sm text-zinc-400">
                        {feature.description}
                      </p>
                    </GlowCard>
                  </motion.div>
                ))}
              </div>

              {/* CTA */}
              <motion.div
                custom={6}
                variants={fadeUp}
                initial="hidden"
                whileInView="visible"
                viewport={{ once: true, margin: "-50px" }}
                className="mt-10"
              >
                <button className="inline-flex items-center justify-center rounded-full border border-[#00FF85] px-6 py-3 text-sm font-semibold text-[#00FF85] transition-all duration-200 hover:bg-[#00FF85]/10 hover:shadow-[0_0_30px_rgba(0,255,133,0.15)]">
                  Buy a DICE Node&nbsp;&rarr;
                </button>
              </motion.div>
            </div>

            {/* Right column: summary visual or empty — the 3D showcase above replaces the old placeholder */}
            <motion.div
              initial={{ opacity: 0, scale: 0.95 }}
              whileInView={{ opacity: 1, scale: 1 }}
              viewport={{ once: true, margin: "-50px" }}
              transition={{
                duration: 0.8,
                ease: [0.25, 0.4, 0.25, 1],
              }}
              className="flex items-center justify-center"
            >
              <div className="relative w-full aspect-square max-w-md bg-zinc-900/50 border border-white/[0.06] rounded-2xl flex flex-col items-center justify-center p-8">
                <div className="absolute inset-0 bg-[radial-gradient(ellipse_at_center,rgba(0,255,133,0.08)_0%,transparent_70%)] pointer-events-none rounded-2xl" />
                <div className="text-center">
                  <p className="text-6xl font-bold text-metallic">$8</p>
                  <p className="mt-2 text-zinc-400 text-sm">per node</p>
                  <div className="mt-6 h-px w-16 mx-auto bg-[#00FF85]/30" />
                  <p className="mt-6 text-2xl font-bold text-metallic">70%</p>
                  <p className="mt-1 text-zinc-400 text-sm">revenue share</p>
                  <div className="mt-6 h-px w-16 mx-auto bg-[#00FF85]/30" />
                  <p className="mt-6 text-2xl font-bold text-[#00FF85]">0.002 SOL</p>
                  <p className="mt-1 text-zinc-400 text-sm">per request fee</p>
                </div>
              </div>
            </motion.div>
          </div>
        </div>
      </div>
    </section>
  )
}

export default ForOperators
