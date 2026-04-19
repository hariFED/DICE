"use client"

import { motion } from "framer-motion"

/**
 * Roadmap — editorial curve timeline. Less words, more visual.
 * Four milestones plotted along a smooth sine-ish curve. Mark completed
 * vs. upcoming via filled vs. ring-only dots.
 *
 * Design ref: Fotiron + OnRamp editorial roadmap.
 */

type Milestone = {
  quarter: string
  label: string
  body: string
  x: number
  y: number
  done: boolean
}

const MILESTONES: Milestone[] = [
  {
    quarter: "Q4 25",
    label: "Devnet launch",
    body: "20-node mesh, commit-reveal, 4s rounds.",
    x: 80,
    y: 180,
    done: true,
  },
  {
    quarter: "Q1 26",
    label: "Anchor 1.0 + SDK v1",
    body: "Stable program, TS SDK, hosted explorer.",
    x: 330,
    y: 110,
    done: true,
  },
  {
    quarter: "Q2 26",
    label: "Mainnet beta",
    body: "Invite-only beta. Streaming VRF primitive.",
    x: 580,
    y: 150,
    done: false,
  },
  {
    quarter: "Q3 26",
    label: "Operator rollout",
    body: "Public pre-order. 100-node target, staking.",
    x: 830,
    y: 90,
    done: false,
  },
  {
    quarter: "Q4 26",
    label: "Permissionless",
    body: "Open operator onboarding. DAO governance.",
    x: 1060,
    y: 160,
    done: false,
  },
]

export function Roadmap() {
  // Build a smooth curve through all milestones
  const pathD = MILESTONES.map((m, i) => {
    if (i === 0) return `M ${m.x} ${m.y}`
    const prev = MILESTONES[i - 1]
    const cpX = (prev.x + m.x) / 2
    return `C ${cpX} ${prev.y}, ${cpX} ${m.y}, ${m.x} ${m.y}`
  }).join(" ")

  // Where the "completed" portion of the curve ends (between last done and first upcoming)
  const lastDoneIdx = [...MILESTONES].map((m, i) => (m.done ? i : -1)).filter((i) => i >= 0).pop() ?? -1
  const doneCutoffX = lastDoneIdx >= 0 ? MILESTONES[lastDoneIdx].x + 30 : 0

  return (
    <section className="relative border-b border-border overflow-hidden">
      <div className="relative mx-auto w-full max-w-[1400px] px-4 sm:px-6 lg:px-8 py-20 md:py-28">
        {/* Section header */}
        <div className="grid grid-cols-1 lg:grid-cols-12 gap-8 mb-12">
          <div className="lg:col-span-3">
            <span className="chapter-num">05</span>
            <p className="mt-2 font-mono text-[11px] uppercase tracking-[0.2em] text-muted-foreground">
              roadmap · trajectory
            </p>
          </div>
          <div className="lg:col-span-6 lg:col-start-5">
            <h2 className="font-sans text-3xl md:text-4xl lg:text-5xl tracking-[-0.02em] leading-[1.05] text-foreground">
              From devnet to <span className="italic font-light text-muted-foreground">permissionless.</span>
            </h2>
            <p className="mt-5 font-mono text-sm text-muted-foreground max-w-md leading-relaxed">
              Five milestones along a single curve. Shipped first, then
              the next one — no ten-year fog plan.
            </p>
          </div>
        </div>

        <div className="relative bg-grid border border-border overflow-hidden py-8">
          <svg viewBox="0 0 1140 280" className="w-full h-auto" style={{ color: "var(--foreground)" }}>
            {/* Faint full curve */}
            <path d={pathD} fill="none" stroke="currentColor" strokeWidth="1" strokeDasharray="3 3" opacity="0.3" />
            {/* Solid overlay up to last-done */}
            <defs>
              <clipPath id="doneClip">
                <rect x="0" y="0" width={doneCutoffX} height="280" />
              </clipPath>
            </defs>
            <path d={pathD} fill="none" stroke="currentColor" strokeWidth="1.8" clipPath="url(#doneClip)" />

            {/* Milestones */}
            {MILESTONES.map((m, i) => (
              <g key={m.quarter}>
                {m.done ? (
                  <circle cx={m.x} cy={m.y} r="8" fill="currentColor" />
                ) : (
                  <>
                    <circle cx={m.x} cy={m.y} r="8" fill="var(--background)" stroke="currentColor" strokeWidth="1.5" />
                    <circle cx={m.x} cy={m.y} r="2.5" fill="currentColor" opacity="0.7" />
                  </>
                )}

                {/* Leader line up to label */}
                <line
                  x1={m.x}
                  y1={m.y - 8}
                  x2={m.x}
                  y2={m.y - 40}
                  stroke="currentColor"
                  strokeWidth="0.8"
                  strokeDasharray="2 2"
                  opacity="0.5"
                />

                {/* Quarter label */}
                <text
                  x={m.x}
                  y={m.y - 46}
                  textAnchor="middle"
                  fontFamily="var(--font-pixel)"
                  fontSize="11"
                  fill="currentColor"
                >
                  {m.quarter}
                </text>
                <text
                  x={m.x}
                  y={m.y + 24}
                  textAnchor="middle"
                  fontFamily="var(--font-mono)"
                  fontSize="10"
                  fill="currentColor"
                  fontWeight="bold"
                >
                  {m.label}
                </text>

                {/* Status tag */}
                {m.done && (
                  <text
                    x={m.x}
                    y={m.y + 38}
                    textAnchor="middle"
                    fontFamily="var(--font-mono)"
                    fontSize="8"
                    fill="var(--status-ok)"
                    letterSpacing="1"
                  >
                    SHIPPED
                  </text>
                )}
              </g>
            ))}
          </svg>

          {/* Body copy row below chart */}
          <div className="grid grid-cols-2 md:grid-cols-5 gap-4 px-6 pb-4 pt-8 border-t border-dashed border-border font-mono text-[11px] leading-relaxed">
            {MILESTONES.map((m) => (
              <div key={m.quarter} className={m.done ? "text-foreground" : "text-muted-foreground/80"}>
                <p className="font-pixel text-[10px] mb-1 text-muted-foreground">{m.quarter}</p>
                <p>{m.body}</p>
              </div>
            ))}
          </div>
        </div>
      </div>
    </section>
  )
}

export default Roadmap
