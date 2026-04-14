"use client"

import { useParams } from "next/navigation"
import Link from "next/link"
import { motion } from "framer-motion"
import { GlowCard } from "@/components/shared/GlowCard"
import { StatusBadge } from "@/components/shared/StatusBadge"
import { CopyButton } from "@/components/shared/CopyButton"
import { useRounds } from "@/lib/hooks"
import { cn } from "@/lib/utils"

type RoundStatus =
  | "collecting_commits"
  | "collecting_reveals"
  | "finalized"
  | "failed"

interface TimelineStep {
  label: string
  detail?: string
  state: "completed" | "active" | "pending"
}

function getTimelineSteps(
  status: RoundStatus,
  commits: number,
  reveals: number,
  total: number
): TimelineStep[] {
  const steps: TimelineStep[] = [
    { label: "Request", detail: "", state: "pending" },
    {
      label: "Commits",
      detail: `${commits}/${total}`,
      state: "pending",
    },
    {
      label: "Reveals",
      detail: `${reveals}/${total}`,
      state: "pending",
    },
    { label: "Finalized", detail: "", state: "pending" },
  ]

  if (status === "collecting_commits") {
    steps[0].state = "completed"
    steps[1].state = "active"
  } else if (status === "collecting_reveals") {
    steps[0].state = "completed"
    steps[1].state = "completed"
    steps[2].state = "active"
  } else if (status === "finalized") {
    steps[0].state = "completed"
    steps[1].state = "completed"
    steps[2].state = "completed"
    steps[3].state = "completed"
  } else if (status === "failed") {
    steps[0].state = "completed"
    if (commits > 0) steps[1].state = "completed"
    if (reveals > 0) steps[2].state = "completed"
  }

  return steps
}

export default function RoundDetailPage() {
  const params = useParams<{ id: string }>()
  const { data: roundsData } = useRounds()

  const rounds = roundsData?.rounds ?? []
  const round = rounds.find((r) => r.request_id === params.id)

  if (!round) {
    return (
      <div className="py-8 space-y-6">
        <Link
          href="/explorer/rounds"
          className="text-sm text-zinc-500 hover:text-zinc-300 transition-colors"
        >
          &larr; Back to Rounds
        </Link>
        <GlowCard className="p-12 text-center">
          <p className="text-zinc-500">
            Round not found. It may still be loading or the ID is invalid.
          </p>
        </GlowCard>
      </div>
    )
  }

  const status = round.status as RoundStatus
  const timeline = getTimelineSteps(
    status,
    round.commits_received,
    round.reveals_received,
    round.node_count
  )

  return (
    <div className="py-8 space-y-6">
      {/* Back link */}
      <Link
        href="/explorer/rounds"
        className="inline-flex items-center gap-1 text-sm text-zinc-500 hover:text-zinc-300 transition-colors"
      >
        &larr; Back to Rounds
      </Link>

      {/* Round header card */}
      <GlowCard className="p-6 space-y-4">
        <div className="flex flex-col md:flex-row md:items-center gap-4 justify-between">
          <div className="space-y-2">
            <p className="text-xs uppercase text-zinc-500 tracking-wider">
              Request ID
            </p>
            <div className="flex items-center gap-2">
              <span className="font-mono text-sm text-zinc-200 break-all">
                {round.request_id}
              </span>
              <CopyButton text={round.request_id} />
            </div>
          </div>
          <div className="flex items-center gap-4">
            <StatusBadge status={status} />
            <span className="text-sm text-zinc-400">
              {(round.elapsed_ms / 1000).toFixed(1)}s
            </span>
          </div>
        </div>
      </GlowCard>

      {/* Timeline visualization */}
      <GlowCard className="p-6">
        <p className="text-xs uppercase text-zinc-500 tracking-wider mb-6">
          Round Timeline
        </p>
        <div className="flex items-center justify-between relative">
          {/* Background line */}
          <div className="absolute top-5 left-0 right-0 h-0.5 bg-white/[0.06]" />

          {timeline.map((step, i) => (
            <div key={step.label} className="flex flex-col items-center z-10">
              {/* Step circle */}
              <motion.div
                initial={{ scale: 0 }}
                animate={{ scale: 1 }}
                transition={{ delay: i * 0.1 }}
                className={cn(
                  "w-10 h-10 rounded-full flex items-center justify-center border-2 relative",
                  step.state === "completed" &&
                    "bg-[#00FF85]/20 border-[#00FF85]",
                  step.state === "active" &&
                    "bg-blue-500/20 border-blue-400 animate-pulse",
                  step.state === "pending" && "bg-zinc-800 border-zinc-700"
                )}
              >
                {step.state === "completed" ? (
                  <svg
                    width="16"
                    height="16"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="#00FF85"
                    strokeWidth="3"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                  >
                    <polyline points="20 6 9 17 4 12" />
                  </svg>
                ) : step.state === "active" ? (
                  <div className="w-3 h-3 rounded-full bg-blue-400" />
                ) : (
                  <div className="w-3 h-3 rounded-full bg-zinc-600" />
                )}
              </motion.div>

              {/* Connecting line (between circles) */}
              {i < timeline.length - 1 && (
                <div
                  className={cn(
                    "absolute top-5 h-0.5",
                    step.state === "completed" ? "bg-[#00FF85]" : "bg-transparent"
                  )}
                  style={{
                    left: `${(i / (timeline.length - 1)) * 100}%`,
                    width: `${(1 / (timeline.length - 1)) * 100}%`,
                  }}
                />
              )}

              {/* Label */}
              <p
                className={cn(
                  "mt-3 text-xs font-medium",
                  step.state === "completed" && "text-[#00FF85]",
                  step.state === "active" && "text-blue-400",
                  step.state === "pending" && "text-zinc-600"
                )}
              >
                {step.label}
              </p>
              {step.detail && (
                <p className="text-xs text-zinc-500 mt-0.5">{step.detail}</p>
              )}
            </div>
          ))}
        </div>
      </GlowCard>

      {/* Randomness output */}
      {round.randomness && (
        <GlowCard className="p-6">
          <p className="text-xs uppercase text-zinc-500 tracking-wider mb-3">
            Randomness Output
          </p>
          <div className="flex items-start gap-3 bg-black/50 rounded-lg p-4 border border-white/[0.06]">
            <span className="font-mono text-sm text-[#00FF85] break-all leading-relaxed flex-1">
              {round.randomness}
            </span>
            <CopyButton text={round.randomness} />
          </div>
        </GlowCard>
      )}

      {/* Round details */}
      <GlowCard className="p-6">
        <p className="text-xs uppercase text-zinc-500 tracking-wider mb-4">
          Details
        </p>
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
          <div>
            <p className="text-xs text-zinc-500">Node Count</p>
            <p className="text-sm text-white font-medium mt-1">
              {round.node_count}
            </p>
          </div>
          <div>
            <p className="text-xs text-zinc-500">Commits</p>
            <p className="text-sm text-white font-medium mt-1">
              {round.commits_received} / {round.node_count}
            </p>
          </div>
          <div>
            <p className="text-xs text-zinc-500">Reveals</p>
            <p className="text-sm text-white font-medium mt-1">
              {round.reveals_received} / {round.node_count}
            </p>
          </div>
          <div>
            <p className="text-xs text-zinc-500">Elapsed</p>
            <p className="text-sm text-white font-medium mt-1">
              {(round.elapsed_ms / 1000).toFixed(1)}s
            </p>
          </div>
        </div>
      </GlowCard>
    </div>
  )
}
