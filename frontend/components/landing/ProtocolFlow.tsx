"use client"

import { useRef } from "react"
import { motion, useInView } from "framer-motion"

/**
 * Protocol flow diagram — "how a round works" in pure visual form.
 * Less words, more UI (per user brief).
 *
 * Left to right: dApp → coordinator → mesh (6 nodes) → chain.
 * Two-pass loop illustrates the commit → reveal cycle.
 *
 * Each stage has a short cap label + time estimate. Node cluster pulses.
 */

type Stage = {
  x: number
  y: number
  r: number
  label: string
  time: string
}

const STAGES: Stage[] = [
  { x: 60,  y: 160, r: 22, label: "dApp",        time: "t=0" },
  { x: 260, y: 160, r: 26, label: "coord",       time: "+80ms" },
  { x: 500, y: 160, r: 32, label: "mesh × 6",    time: "+1.2s" },
  { x: 740, y: 160, r: 26, label: "coord",       time: "+2.8s" },
  { x: 940, y: 160, r: 22, label: "chain",       time: "+3.9s" },
]

const NODE_CLUSTER = [
  { x: 460, y: 100 },
  { x: 500, y: 80 },
  { x: 540, y: 100 },
  { x: 460, y: 220 },
  { x: 500, y: 240 },
  { x: 540, y: 220 },
]

// Packet route — dApp → coord → (up commit arc) → mesh → (down reveal arc) → coord → chain.
// Times are normalized 0..1 over the 6-second loop. The apex of each arc
// lands at y=40, matching the Q-control-point in the SVG paths above.
const LOOP_S = 6
const PACKET_FRAMES = {
  cx: [60, 260, 380, 500, 620, 740, 940, 940],
  cy: [160, 160, 40, 160, 40, 160, 160, 160],
  // slower in the arcs (commit/reveal is where the work happens)
  times: [0, 0.12, 0.25, 0.38, 0.52, 0.65, 0.82, 1],
}
// Seconds at which the packet lands on each stage — used to stagger
// the pulse rings so they fire in sync with the moving dot.
const STAGE_ARRIVAL_S = [0, 0.12, 0.38, 0.65, 0.82].map((t) => t * LOOP_S)

export function ProtocolFlow() {
  const sectionRef = useRef<HTMLElement>(null)
  const isInView = useInView(sectionRef, { margin: "-100px", once: false })

  return (
    <section ref={sectionRef} className="relative border-b border-border overflow-hidden">
      <div className="relative mx-auto w-full max-w-[1400px] px-4 sm:px-6 lg:px-8 py-20 md:py-28">
        {/* Section header */}
        <div className="grid grid-cols-1 lg:grid-cols-12 gap-8 mb-12">
          <div className="lg:col-span-3">
            <span className="chapter-num">02</span>
            <p className="mt-2 font-mono text-[11px] uppercase tracking-[0.2em] text-muted-foreground">
              protocol · trace
            </p>
          </div>
          <div className="lg:col-span-6 lg:col-start-5">
            <h2 className="font-sans text-3xl md:text-4xl lg:text-5xl tracking-[-0.02em] leading-[1.05] text-foreground">
              One request. <span className="italic font-light text-muted-foreground">Four seconds.</span>
            </h2>
            <p className="mt-5 font-mono text-sm text-muted-foreground max-w-md leading-relaxed">
              Commit-reveal runs across six hardware nodes in parallel.
              The coordinator bundles the result into a single Solana
              transaction. No stake, no relayer, no token.
            </p>
          </div>
        </div>

        {/* Diagram */}
        <div className="relative bg-grid border border-border overflow-hidden">
          <span className="absolute top-3 left-3 font-mono text-[10px] uppercase tracking-wider text-muted-foreground/50">FIG_002 · commit-reveal · single round</span>
          <span className="absolute top-3 right-3 font-mono text-[10px] uppercase tracking-wider text-muted-foreground/50">wall clock : 3.9 s</span>

          <svg viewBox="0 0 1000 320" className="w-full h-auto" style={{ color: "var(--foreground)" }}>
            {/* Horizontal spine */}
            <line x1="60" y1="160" x2="940" y2="160" stroke="currentColor" strokeWidth="1" strokeDasharray="4 3" opacity="0.35" />

            {/* Commit arc (top) and Reveal arc (bottom) — only on mesh hop */}
            <path
              d="M 260 160 Q 380 40 500 160"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.2"
              strokeDasharray="3 3"
              opacity="0.55"
            />
            <path
              d="M 500 160 Q 620 40 740 160"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.2"
              opacity="0.85"
            />
            {/* commit label */}
            <text x="380" y="48" textAnchor="middle" fontFamily="var(--font-mono)" fontSize="10" fill="currentColor" opacity="0.7">
              commit
            </text>
            <text x="620" y="48" textAnchor="middle" fontFamily="var(--font-mono)" fontSize="10" fill="currentColor">
              reveal
            </text>

            {/* Node cluster — small filled circles */}
            {NODE_CLUSTER.map((n, i) => (
              <motion.circle
                key={i}
                cx={n.x}
                cy={n.y}
                r="5"
                fill="currentColor"
                initial={{ opacity: 0.3 }}
                animate={isInView ? { opacity: [0.3, 1, 0.3] } : { opacity: 0.3 }}
                transition={{ duration: 2, delay: i * 0.15, repeat: isInView ? Infinity : 0 }}
              />
            ))}

            {/* Stage nodes */}
            {STAGES.map((s, i) => (
              <g key={i}>
                {/* expanding pulse ring — fires when the packet arrives */}
                <motion.circle
                  cx={s.x}
                  cy={s.y}
                  r={s.r}
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="1.2"
                  initial={{ scale: 1, opacity: 0 }}
                  animate={isInView ? { scale: [1, 1.9, 1], opacity: [0.9, 0, 0] } : { scale: 1, opacity: 0 }}
                  transition={{
                    duration: 0.9,
                    delay: STAGE_ARRIVAL_S[i],
                    repeat: isInView ? Infinity : 0,
                    repeatDelay: LOOP_S - 0.9,
                    ease: "easeOut",
                  }}
                  style={{ transformOrigin: `${s.x}px ${s.y}px` }}
                />
                <circle
                  cx={s.x}
                  cy={s.y}
                  r={s.r}
                  fill="var(--background)"
                  stroke="currentColor"
                  strokeWidth="1.5"
                />
                <circle
                  cx={s.x}
                  cy={s.y}
                  r={s.r - 6}
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="0.6"
                  opacity="0.5"
                  strokeDasharray="2 2"
                />
                {/* center dot */}
                <circle cx={s.x} cy={s.y} r="2.5" fill="currentColor" />

                {/* Stage label below */}
                <text
                  x={s.x}
                  y={s.y + s.r + 20}
                  textAnchor="middle"
                  fontFamily="var(--font-pixel)"
                  fontSize="12"
                  fill="currentColor"
                >
                  {s.label}
                </text>
                <text
                  x={s.x}
                  y={s.y + s.r + 34}
                  textAnchor="middle"
                  fontFamily="var(--font-mono)"
                  fontSize="9"
                  fill="currentColor"
                  opacity="0.55"
                >
                  {s.time}
                </text>

                {/* stage index above */}
                <text
                  x={s.x}
                  y={s.y - s.r - 10}
                  textAnchor="middle"
                  fontFamily="var(--font-pixel)"
                  fontSize="10"
                  fill="currentColor"
                  opacity="0.4"
                >
                  0{i + 1}
                </text>
              </g>
            ))}

            {/* Arrow heads between stages */}
            {STAGES.slice(0, -1).map((s, i) => {
              const next = STAGES[i + 1]
              const ax = (s.x + next.x) / 2 + 10
              return (
                <polygon
                  key={`arr-${i}`}
                  points={`${ax - 4},155 ${ax + 2},160 ${ax - 4},165`}
                  fill="currentColor"
                  opacity="0.6"
                />
              )
            })}

            {/* The moving packet — traces request → commit → reveal → chain.
                Trail dot lags behind for a subtle comet effect. */}
            <motion.circle
              r="6"
              fill="currentColor"
              initial={{ cx: 60, cy: 160, opacity: 0 }}
              animate={
                isInView
                  ? { cx: PACKET_FRAMES.cx, cy: PACKET_FRAMES.cy, opacity: [0, 1, 1, 1, 1, 1, 1, 0] }
                  : { cx: 60, cy: 160, opacity: 0 }
              }
              transition={{
                duration: LOOP_S,
                times: PACKET_FRAMES.times,
                repeat: isInView ? Infinity : 0,
                ease: "linear",
              }}
            />
            <motion.circle
              r="4"
              fill="currentColor"
              opacity="0.4"
              initial={{ cx: 60, cy: 160 }}
              animate={
                isInView
                  ? { cx: PACKET_FRAMES.cx, cy: PACKET_FRAMES.cy }
                  : { cx: 60, cy: 160 }
              }
              transition={{
                duration: LOOP_S,
                times: PACKET_FRAMES.times,
                repeat: isInView ? Infinity : 0,
                ease: "linear",
                delay: 0.15,
              }}
            />
          </svg>

          <span className="absolute bottom-3 left-3 font-mono text-[10px] uppercase tracking-wider text-muted-foreground/50">each hop verified · no stake · no token</span>
          <span className="absolute bottom-3 right-3 font-mono text-[10px] uppercase tracking-wider text-muted-foreground/50">0.002 SOL / request</span>
        </div>
      </div>
    </section>
  )
}

export default ProtocolFlow
