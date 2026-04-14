"use client"

import { motion } from "framer-motion"
import { GlowCard } from "@/components/shared/GlowCard"

const steps = [
  {
    number: "01",
    title: "Request Randomness",
    description:
      "Developer calls request_randomness() on-chain. 0.002 SOL fee auto-deducted from channel.",
    icon: (
      <svg
        viewBox="0 0 24 24"
        fill="none"
        className="h-8 w-8"
        stroke="currentColor"
        strokeWidth={1.5}
      >
        <path
          strokeLinecap="round"
          strokeLinejoin="round"
          d="M21 7.5l-2.25-1.313M21 7.5v2.25m0-2.25l-2.25 1.313M3 7.5l2.25-1.313M3 7.5l2.25 1.313M3 7.5v2.25m9 3l2.25-1.313M12 12.75l-2.25-1.313M12 12.75V15m0 6.75l2.25-1.313M12 21.75V19.5m0 2.25l-2.25-1.313m0-16.875L12 2.25l2.25 1.313M21 14.25v2.25l-2.25 1.313m-13.5 0L3 16.5v-2.25"
        />
      </svg>
    ),
  },
  {
    number: "02",
    title: "Node Selection",
    description:
      "4-7 nodes trustlessly selected using Solana's SlotHashes. No coordinator manipulation possible.",
    icon: (
      <svg
        viewBox="0 0 24 24"
        fill="none"
        className="h-8 w-8"
        stroke="currentColor"
        strokeWidth={1.5}
      >
        <path
          strokeLinecap="round"
          strokeLinejoin="round"
          d="M7.5 21L3 16.5m0 0L7.5 12M3 16.5h13.5m0-13.5L21 7.5m0 0L16.5 12M21 7.5H7.5"
        />
        <circle cx="12" cy="12" r="2" />
        <circle cx="5" cy="5" r="1.5" />
        <circle cx="19" cy="5" r="1.5" />
        <circle cx="5" cy="19" r="1.5" />
        <circle cx="19" cy="19" r="1.5" />
      </svg>
    ),
  },
  {
    number: "03",
    title: "Commit-Reveal",
    description:
      "Each node generates entropy from hardware TRNG, commits hash, then reveals. ECDSA signatures verify authenticity.",
    icon: (
      <svg
        viewBox="0 0 24 24"
        fill="none"
        className="h-8 w-8"
        stroke="currentColor"
        strokeWidth={1.5}
      >
        <path
          strokeLinecap="round"
          strokeLinejoin="round"
          d="M9 12.75L11.25 15 15 9.75m-3-7.036A11.959 11.959 0 013.598 6 11.99 11.99 0 003 9.749c0 5.592 3.824 10.29 9 11.623 5.176-1.332 9-6.03 9-11.622 0-1.31-.21-2.571-.598-3.751h-.152c-3.196 0-6.1-1.248-8.25-3.285z"
        />
      </svg>
    ),
  },
  {
    number: "04",
    title: "Delivery",
    description:
      "Combined entropy delivered via CPI callback to your program. Fully verifiable on-chain.",
    icon: (
      <svg
        viewBox="0 0 24 24"
        fill="none"
        className="h-8 w-8"
        stroke="currentColor"
        strokeWidth={1.5}
      >
        <path
          strokeLinecap="round"
          strokeLinejoin="round"
          d="M3.75 13.5l10.5-11.25L12 10.5h8.25L9.75 21.75 12 13.5H3.75z"
        />
      </svg>
    ),
  },
]

const cardVariants = {
  hidden: { opacity: 0, y: 40 },
  visible: (i: number) => ({
    opacity: 1,
    y: 0,
    transition: {
      delay: 0.2 * i,
      duration: 0.6,
      ease: [0.25, 0.4, 0.25, 1] as [number, number, number, number],
    },
  }),
}

export function HowItWorks() {
  return (
    <section className="relative py-32 overflow-hidden">
      {/* Subtle background glow */}
      <div className="absolute inset-0 bg-[radial-gradient(ellipse_at_50%_0%,rgba(0,255,133,0.03)_0%,transparent_50%)] pointer-events-none" />

      <div className="relative mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        {/* Section header */}
        <motion.div
          initial={{ opacity: 0, y: 30 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true, margin: "-100px" }}
          transition={{ duration: 0.6, ease: [0.25, 0.4, 0.25, 1] }}
          className="text-center"
        >
          <h2 className="text-4xl md:text-5xl font-bold text-metallic">
            How It Works
          </h2>
          <p className="mt-4 text-lg text-zinc-400 max-w-xl mx-auto">
            From request to verified randomness in under 2 seconds
          </p>
        </motion.div>

        {/* Timeline connector line — desktop only */}
        <div className="hidden lg:block relative mt-24 mb-0">
          <div className="absolute top-1/2 left-[12.5%] right-[12.5%] h-px -translate-y-1/2">
            <div className="h-full w-full border-t border-dashed border-[#00FF85]/30" />
            {/* Animated glow along the line */}
            <motion.div
              initial={{ scaleX: 0 }}
              whileInView={{ scaleX: 1 }}
              viewport={{ once: true, margin: "-100px" }}
              transition={{ duration: 1.5, delay: 0.5, ease: "easeOut" }}
              className="absolute inset-0 origin-left h-px bg-gradient-to-r from-[#00FF85]/60 via-[#00FF85]/20 to-[#00FF85]/60"
            />
          </div>
        </div>

        {/* Steps grid */}
        <div className="mt-16 lg:mt-0 grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-6 lg:gap-5">
          {steps.map((step, i) => (
            <motion.div
              key={step.number}
              custom={i}
              variants={cardVariants}
              initial="hidden"
              whileInView="visible"
              viewport={{ once: true, margin: "-80px" }}
              className="relative"
            >
              {/* Mobile/tablet vertical connector */}
              {i < steps.length - 1 && (
                <div className="lg:hidden absolute left-1/2 -bottom-6 w-px h-6 border-l border-dashed border-[#00FF85]/30" />
              )}

              <GlowCard className="relative p-6 h-full flex flex-col items-center text-center">
                {/* Step number badge */}
                <span className="absolute -top-3 left-6 inline-flex items-center justify-center rounded-full bg-black border border-[#00FF85]/40 px-3 py-0.5 text-xs font-mono font-semibold text-[#00FF85]">
                  {step.number}
                </span>

                {/* Icon */}
                <div className="mt-4 flex items-center justify-center w-14 h-14 rounded-xl bg-[#00FF85]/[0.08] border border-[#00FF85]/20 text-[#00FF85]">
                  {step.icon}
                </div>

                {/* Title */}
                <h3 className="mt-5 text-lg font-semibold text-white">
                  {step.title}
                </h3>

                {/* Description */}
                <p className="mt-3 text-sm leading-relaxed text-zinc-400">
                  {step.description}
                </p>
              </GlowCard>
            </motion.div>
          ))}
        </div>
      </div>
    </section>
  )
}

export default HowItWorks
