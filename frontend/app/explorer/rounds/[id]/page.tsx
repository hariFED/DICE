"use client"

import { useParams } from "next/navigation"
import Link from "next/link"
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

function getTimelineSteps(status: RoundStatus, commits: number, reveals: number, total: number): TimelineStep[] {
  const steps: TimelineStep[] = [
    { label: "request",   state: "pending" },
    { label: "commits",   detail: `${commits}/${total}`, state: "pending" },
    { label: "reveals",   detail: `${reveals}/${total}`, state: "pending" },
    { label: "finalized", state: "pending" },
  ]
  if (status === "collecting_commits") { steps[0].state = "completed"; steps[1].state = "active" }
  else if (status === "collecting_reveals") { steps[0].state = "completed"; steps[1].state = "completed"; steps[2].state = "active" }
  else if (status === "finalized") steps.forEach((s) => { s.state = "completed" })
  else if (status === "failed") {
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
      <div className="pb-12 space-y-6">
        <Link href="/explorer/rounds" className="ascii-label hover:text-foreground transition-colors">
          ‹ back · to · rounds
        </Link>
        <div className="ascii-box p-12 text-center text-muted-foreground font-mono">
          — round not found · may be loading or id is invalid —
        </div>
      </div>
    )
  }

  const status = round.status as RoundStatus
  const timeline = getTimelineSteps(status, round.commits_received, round.reveals_received, round.node_count)

  return (
    <div className="pb-12 space-y-6">
      <Link href="/explorer/rounds" className="ascii-label hover:text-foreground transition-colors">
        ‹ back · to · rounds
      </Link>

      {/* Header */}
      <div className="ascii-box p-6">
        <div className="flex flex-col md:flex-row md:items-center gap-4 justify-between">
          <div className="space-y-2 min-w-0">
            <p className="ascii-label text-[10px]">request_id</p>
            <div className="flex items-center gap-2 min-w-0">
              <span className="font-mono text-sm text-foreground break-all">{round.request_id}</span>
              <CopyButton text={round.request_id} />
            </div>
          </div>
          <div className="flex items-center gap-4">
            <StatusBadge status={status} />
            <span className="font-mono text-sm text-foreground tabular-nums">
              {(round.elapsed_ms / 1000).toFixed(1)}s
            </span>
          </div>
        </div>
      </div>

      {/* Timeline */}
      <div className="ascii-box p-6">
        <p className="ascii-label text-[10px] mb-6">round · timeline</p>
        <div className="flex items-start justify-between relative font-mono">
          <div className="absolute top-3 left-[5%] right-[5%] h-px bg-border" />
          {timeline.map((step, i) => (
            <div key={step.label} className="relative z-10 flex flex-col items-center flex-1">
              <div
                className={cn(
                  "w-6 h-6 flex items-center justify-center border text-xs leading-none",
                  step.state === "completed" && "bg-foreground text-background border-foreground",
                  step.state === "active" && "border-foreground text-foreground bg-background",
                  step.state === "pending" && "border-border text-muted-foreground bg-background"
                )}
              >
                {step.state === "completed" ? "✓" : step.state === "active" ? "◌" : "·"}
              </div>
              <p className={cn(
                "mt-3 text-xs tracking-wider uppercase",
                step.state === "completed" && "text-foreground",
                step.state === "active" && "text-foreground",
                step.state === "pending" && "text-muted-foreground",
              )}>
                {step.label}
              </p>
              {step.detail && <p className="text-xs text-muted-foreground mt-0.5 tabular-nums">{step.detail}</p>}
            </div>
          ))}
        </div>
      </div>

      {/* Randomness output */}
      {round.randomness && (
        <div className="ascii-box p-6">
          <p className="ascii-label text-[10px] mb-3">randomness · output</p>
          <div className="flex items-start gap-3 border border-border p-4 bg-muted/30">
            <span className="font-mono text-sm text-foreground break-all leading-relaxed flex-1">
              {round.randomness}
            </span>
            <CopyButton text={round.randomness} />
          </div>
        </div>
      )}

      {/* Details grid */}
      <div className="ascii-box">
        <div className="px-6 py-3 border-b border-border"><p className="ascii-label text-[10px]">details</p></div>
        <div className="grid grid-cols-2 md:grid-cols-4 divide-x divide-y divide-border">
          {[
            ["node_count", String(round.node_count)],
            ["commits", `${round.commits_received} / ${round.node_count}`],
            ["reveals", `${round.reveals_received} / ${round.node_count}`],
            ["elapsed", `${(round.elapsed_ms / 1000).toFixed(1)}s`],
          ].map(([k, v]) => (
            <div key={k} className="p-4">
              <p className="ascii-label text-[10px] mb-1">{k}</p>
              <p className="font-mono text-sm text-foreground tabular-nums">{v}</p>
            </div>
          ))}
        </div>
      </div>
    </div>
  )
}
