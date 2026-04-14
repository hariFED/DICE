"use client"

import { motion } from "framer-motion"
import { GlowCard } from "@/components/shared/GlowCard"

const metrics = [
  {
    value: "2 Lines of Code",
    description: "Integrate DICE VRF with just two instructions",
  },
  {
    value: "< 15 Minutes",
    description: "From zero to first randomness request",
  },
  {
    value: "0.002 SOL",
    description: "Per request. No tokens. No staking.",
  },
]

const comparisonRows = [
  {
    feature: "Integration",
    dice: "2 lines",
    switchboard: "~50 lines",
    chainlink: "~100 lines",
  },
  {
    feature: "Cost",
    dice: "0.002 SOL",
    switchboard: "Variable + token",
    chainlink: "LINK token",
  },
  {
    feature: "Latency",
    dice: "< 2s",
    switchboard: "2-5s",
    chainlink: "10-30s",
  },
  {
    feature: "Token Required",
    dice: "No",
    switchboard: "SBV2",
    chainlink: "LINK",
  },
  {
    feature: "Hardware Backed",
    dice: "Yes",
    switchboard: "No",
    chainlink: "No",
  },
]

function CodeBlock() {
  return (
    <div className="rounded-xl border border-white/[0.08] bg-[#0a0a0a] overflow-hidden">
      {/* Header bar */}
      <div className="flex items-center gap-2 border-b border-white/[0.08] px-4 py-3">
        <div className="flex gap-1.5">
          <span className="h-3 w-3 rounded-full bg-white/10" />
          <span className="h-3 w-3 rounded-full bg-white/10" />
          <span className="h-3 w-3 rounded-full bg-white/10" />
        </div>
        <span className="ml-2 text-xs font-medium text-zinc-500 tracking-wide uppercase">
          DICE VRF Integration
        </span>
      </div>

      {/* Code content */}
      <div className="p-5 overflow-x-auto">
        <pre className="text-sm leading-relaxed font-mono">
          <code>
            <span className="text-[#00FF85]">use</span>
            <span className="text-zinc-200"> dice_vrf</span>
            <span className="text-zinc-200">::</span>
            <span className="text-zinc-200">cpi</span>
            <span className="text-zinc-200">::</span>
            <span className="text-zinc-200">request_randomness;</span>
            {"\n\n"}
            <span className="text-[#00FF85]">pub</span>
            <span className="text-zinc-200"> </span>
            <span className="text-[#00FF85]">fn</span>
            <span className="text-zinc-200"> </span>
            <span className="text-blue-400">roll_dice</span>
            <span className="text-zinc-200">(</span>
            <span className="text-[#00FF85]">ctx</span>
            <span className="text-zinc-200">: </span>
            <span className="text-blue-400">Context</span>
            <span className="text-zinc-200">{"<"}</span>
            <span className="text-blue-400">RollDice</span>
            <span className="text-zinc-200">{">"}</span>
            <span className="text-zinc-200">) -{">"} </span>
            <span className="text-blue-400">Result</span>
            <span className="text-zinc-200">{"<()>"} {"{"}</span>
            {"\n"}
            <span className="text-zinc-500">{"    "}// Request verifiable randomness — that&apos;s it</span>
            {"\n"}
            <span className="text-zinc-200">{"    "}</span>
            <span className="text-zinc-200">request_randomness(</span>
            <span className="text-[#00FF85]">ctx</span>
            <span className="text-zinc-200">.accounts.dice_ctx(), </span>
            {"\n"}
            <span className="text-zinc-200">{"        "}sequence, &amp;crate::ID)?;</span>
            {"\n"}
            <span className="text-zinc-200">{"    "}</span>
            <span className="text-[#00FF85]">Ok</span>
            <span className="text-zinc-200">(())</span>
            {"\n"}
            <span className="text-zinc-200">{"}"}</span>
            {"\n\n"}
            <span className="text-zinc-500">// DICE calls back with the result</span>
            {"\n"}
            <span className="text-[#00FF85]">pub</span>
            <span className="text-zinc-200"> </span>
            <span className="text-[#00FF85]">fn</span>
            <span className="text-zinc-200"> </span>
            <span className="text-blue-400">dice_callback</span>
            <span className="text-zinc-200">(</span>
            <span className="text-[#00FF85]">ctx</span>
            <span className="text-zinc-200">: </span>
            <span className="text-blue-400">Context</span>
            <span className="text-zinc-200">{"<"}</span>
            <span className="text-blue-400">Callback</span>
            <span className="text-zinc-200">{">"}, </span>
            {"\n"}
            <span className="text-zinc-200">{"    "}randomness: [</span>
            <span className="text-blue-400">u8</span>
            <span className="text-zinc-200">; 32]) -{">"} </span>
            <span className="text-blue-400">Result</span>
            <span className="text-zinc-200">{"<()>"} {"{"}</span>
            {"\n"}
            <span className="text-zinc-200">{"    "}</span>
            <span className="text-[#00FF85]">let</span>
            <span className="text-zinc-200"> roll = (randomness[0] % 6) + 1;</span>
            {"\n"}
            <span className="text-zinc-200">{"    "}msg!(</span>
            <span className="text-amber-300">&quot;You rolled: {"{}"}&quot;</span>
            <span className="text-zinc-200">, roll);</span>
            {"\n"}
            <span className="text-zinc-200">{"    "}</span>
            <span className="text-[#00FF85]">Ok</span>
            <span className="text-zinc-200">(())</span>
            {"\n"}
            <span className="text-zinc-200">{"}"}</span>
          </code>
        </pre>
      </div>
    </div>
  )
}

function ComparisonTable() {
  return (
    <GlowCard className="overflow-hidden">
      <div className="p-4 border-b border-white/[0.08]">
        <h4 className="text-sm font-semibold text-zinc-300 uppercase tracking-wider">
          Comparison
        </h4>
      </div>
      <div className="overflow-x-auto">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-zinc-800">
              <th className="text-left px-4 py-3 text-zinc-500 font-medium">
                Feature
              </th>
              <th className="text-left px-4 py-3 text-[#00FF85] font-semibold">
                DICE
              </th>
              <th className="text-left px-4 py-3 text-zinc-400 font-medium">
                Switchboard
              </th>
              <th className="text-left px-4 py-3 text-zinc-400 font-medium">
                Chainlink VRF
              </th>
            </tr>
          </thead>
          <tbody>
            {comparisonRows.map((row) => (
              <tr key={row.feature} className="border-b border-zinc-800/60 last:border-0">
                <td className="px-4 py-3 text-zinc-400">{row.feature}</td>
                <td className="px-4 py-3 text-[#00FF85] font-medium">
                  {row.dice}
                </td>
                <td className="px-4 py-3 text-zinc-500">{row.switchboard}</td>
                <td className="px-4 py-3 text-zinc-500">{row.chainlink}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </GlowCard>
  )
}

const fadeLeft = {
  hidden: { opacity: 0, x: -40 },
  visible: {
    opacity: 1,
    x: 0,
    transition: {
      duration: 0.7,
      ease: [0.25, 0.4, 0.25, 1] as [number, number, number, number],
    },
  },
}

const fadeRight = {
  hidden: { opacity: 0, x: 40 },
  visible: {
    opacity: 1,
    x: 0,
    transition: {
      duration: 0.7,
      ease: [0.25, 0.4, 0.25, 1] as [number, number, number, number],
    },
  },
}

const metricVariants = {
  hidden: { opacity: 0, y: 20 },
  visible: (i: number) => ({
    opacity: 1,
    y: 0,
    transition: {
      delay: 0.15 * i,
      duration: 0.5,
      ease: [0.25, 0.4, 0.25, 1] as [number, number, number, number],
    },
  }),
}

export function ForDevelopers() {
  return (
    <section className="relative py-32 overflow-hidden">
      {/* Subtle gradient */}
      <div className="absolute inset-0 bg-[radial-gradient(ellipse_at_30%_50%,rgba(0,255,133,0.02)_0%,transparent_60%)] pointer-events-none" />

      <div className="relative mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        {/* Section header */}
        <motion.div
          initial={{ opacity: 0, y: 30 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true, margin: "-100px" }}
          transition={{ duration: 0.6, ease: [0.25, 0.4, 0.25, 1] }}
          className="text-center mb-16"
        >
          <h2 className="text-4xl md:text-5xl font-bold text-metallic">
            For Developers
          </h2>
          <p className="mt-4 text-lg text-zinc-400 max-w-xl mx-auto">
            Two instructions. Full verifiability. Ship in minutes.
          </p>
        </motion.div>

        {/* Two column layout */}
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-10 lg:gap-14 items-start">
          {/* Left — Code block */}
          <motion.div
            variants={fadeLeft}
            initial="hidden"
            whileInView="visible"
            viewport={{ once: true, margin: "-80px" }}
          >
            <CodeBlock />
          </motion.div>

          {/* Right — Metrics + Comparison */}
          <div>
            {/* Metric cards */}
            <div className="space-y-4">
              {metrics.map((metric, i) => (
                <motion.div
                  key={metric.value}
                  custom={i}
                  variants={metricVariants}
                  initial="hidden"
                  whileInView="visible"
                  viewport={{ once: true, margin: "-60px" }}
                >
                  <GlowCard className="p-5">
                    <p className="text-xl font-bold text-[#00FF85]">
                      {metric.value}
                    </p>
                    <p className="mt-1 text-sm text-zinc-400">
                      {metric.description}
                    </p>
                  </GlowCard>
                </motion.div>
              ))}
            </div>

            {/* Comparison table */}
            <motion.div
              variants={fadeRight}
              initial="hidden"
              whileInView="visible"
              viewport={{ once: true, margin: "-80px" }}
              className="mt-6"
            >
              <ComparisonTable />
            </motion.div>

            {/* CTA */}
            <motion.div
              initial={{ opacity: 0 }}
              whileInView={{ opacity: 1 }}
              viewport={{ once: true }}
              transition={{ delay: 0.6, duration: 0.5 }}
              className="mt-6"
            >
              <a
                href="#"
                className="inline-flex items-center text-[#00FF85] font-medium text-sm hover:underline underline-offset-4 transition-all duration-200"
              >
                Read the Docs
                <span className="ml-1">{"\u2192"}</span>
              </a>
            </motion.div>
          </div>
        </div>
      </div>
    </section>
  )
}

export default ForDevelopers
