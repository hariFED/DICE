"use client"

import { useState, useMemo } from "react"
import Link from "next/link"
import { motion } from "framer-motion"
import { StatusBadge } from "@/components/shared/StatusBadge"
import { CopyButton } from "@/components/shared/CopyButton"
import { useRounds } from "@/lib/hooks"
import type { Round } from "@/lib/types"
import { cn } from "@/lib/utils"
import { CornerBox } from "@/components/shared/CornerBox"

const FILTERS = [
  { key: "all", label: "all" },
  { key: "finalized", label: "finalized" },
  { key: "failed", label: "failed" },
  { key: "in_progress", label: "in-progress" },
] as const

type FilterKey = (typeof FILTERS)[number]["key"]

function truncateId(id: string) {
  if (id.length <= 16) return id
  return id.slice(0, 8) + "…" + id.slice(-4)
}

function formatElapsed(ms: number) {
  return (ms / 1000).toFixed(1) + "s"
}

function matchesFilter(round: Round, filter: FilterKey): boolean {
  if (filter === "all") return true
  if (filter === "finalized") return round.status === "finalized"
  if (filter === "failed") return round.status === "failed"
  if (filter === "in_progress")
    return (
      round.status === "collecting_commits" ||
      round.status === "collecting_reveals"
    )
  return true
}

export default function RoundsPage() {
  const { data: roundsData } = useRounds()
  const [filter, setFilter] = useState<FilterKey>("all")

  const rounds = roundsData?.rounds ?? []

  const filtered = useMemo(
    () => rounds.filter((r) => matchesFilter(r, filter)),
    [rounds, filter]
  )

  return (
    <div className="pb-12 space-y-6">
      <div className="flex items-end justify-between gap-4 flex-wrap">
        <h1 className="text-3xl md:text-4xl font-semibold tracking-tight">Rounds</h1>
        <div className="flex gap-2 text-xs font-mono">
          <Link href="/explorer" className="border border-border text-muted-foreground hover:text-foreground hover:border-foreground transition-colors px-3 py-1 uppercase tracking-wider"><span className="mr-1"> </span>overview</Link>
          <Link href="/explorer/rounds" className="border border-foreground bg-foreground text-background px-3 py-1 uppercase tracking-wider"><span className="mr-1">▸</span>rounds</Link>
          <Link href="/explorer/nodes" className="border border-border text-muted-foreground hover:text-foreground hover:border-foreground transition-colors px-3 py-1 uppercase tracking-wider"><span className="mr-1"> </span>nodes</Link>
        </div>
      </div>

      {/* Filter row */}
      <div className="flex items-center gap-2 flex-wrap">
        <span className="ascii-label mr-2">filter</span>
        {FILTERS.map((f) => (
          <button
            key={f.key}
            onClick={() => setFilter(f.key)}
            className={cn(
              "border px-3 py-1 text-xs font-mono uppercase tracking-wider transition-colors",
              filter === f.key
                ? "border-foreground bg-foreground text-background"
                : "border-border text-muted-foreground hover:text-foreground hover:border-foreground"
            )}
          >
            {f.label}
          </button>
        ))}
        <span className="ml-auto font-mono text-xs text-muted-foreground">
          {filtered.length} / {rounds.length}
        </span>
      </div>

      {/* Rounds table */}
      <CornerBox title={`rounds · n=${filtered.length}`} tag={filter} innerClassName="p-0">
        <div className="overflow-x-auto">
        <table className="w-full text-sm font-mono">
          <thead>
            <tr className="border-b border-border bg-muted/30">
              {["request_id", "status", "nodes", "commits/reveals", "elapsed", "randomness"].map((h) => (
                <th key={h} className="text-left px-3 py-2 ascii-label text-[10px]">{h}</th>
              ))}
            </tr>
          </thead>
          <tbody>
            {filtered.length === 0 ? (
              <tr>
                <td colSpan={6} className="text-center text-muted-foreground py-10">— no rounds found —</td>
              </tr>
            ) : (
              filtered.map((round, i) => (
                <motion.tr
                  key={`${round.request_id}-${round.timestamp}-${i}`}
                  initial={{ opacity: 0, y: 4 }}
                  animate={{ opacity: 1, y: 0 }}
                  transition={{ delay: i * 0.015 }}
                  className="border-b border-border last:border-0 hover:bg-muted/30 transition-colors"
                >
                  <td className="px-3 py-2.5">
                    <Link href={`/explorer/rounds/${round.request_id}`} className="inline-flex items-center gap-1.5 group">
                      <span className="text-foreground group-hover:underline underline-offset-4">{truncateId(round.request_id)}</span>
                      <CopyButton text={round.request_id} />
                    </Link>
                  </td>
                  <td className="px-3 py-2.5">
                    <StatusBadge
                      status={round.status as "finalized" | "failed" | "collecting_commits" | "collecting_reveals"}
                    />
                  </td>
                  <td className="px-3 py-2.5 text-muted-foreground">{round.node_count}</td>
                  <td className="px-3 py-2.5 text-muted-foreground tabular-nums">
                    {round.commits_received}/{round.node_count} · {round.reveals_received}/{round.node_count}
                  </td>
                  <td className="px-3 py-2.5 text-foreground tabular-nums">{formatElapsed(round.elapsed_ms)}</td>
                  <td className="px-3 py-2.5">
                    {round.randomness ? (
                      <div className="flex items-center gap-1.5">
                        <span className="text-muted-foreground">{truncateId(round.randomness)}</span>
                        <CopyButton text={round.randomness} />
                      </div>
                    ) : (
                      <span className="text-muted-foreground/50">—</span>
                    )}
                  </td>
                </motion.tr>
              ))
            )}
          </tbody>
        </table>
        </div>
      </CornerBox>
    </div>
  )
}
