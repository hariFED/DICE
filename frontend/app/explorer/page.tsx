"use client"

import { motion } from "framer-motion"
import Link from "next/link"
import { StatusBadge } from "@/components/shared/StatusBadge"
import { CopyButton } from "@/components/shared/CopyButton"
import { useNodes, useRounds, useQueue } from "@/lib/hooks"
import { CornerBox } from "@/components/shared/CornerBox"
import { cn } from "@/lib/utils"

function truncateId(id: string) {
  if (id.length <= 16) return id
  return id.slice(0, 8) + "…" + id.slice(-4)
}

function formatElapsed(ms: number) {
  return (ms / 1000).toFixed(1) + "s"
}

interface StatCellProps {
  label: string
  value: string
  live?: boolean
  className?: string
}

function StatCell({ label, value, live, className = "" }: StatCellProps) {
  return (
    <div className={`p-5 ${className}`}>
      <p className="ascii-label text-[10px] mb-3 flex items-center gap-2">
        {live && <span className="inline-block h-1.5 w-1.5 rounded-full bg-[var(--status-ok)] animate-pulse" />}
        {label}
      </p>
      <p className="font-mono text-2xl text-foreground tabular-nums">{value}</p>
    </div>
  )
}

function TabLink({ href, label, active }: { href: string; label: string; active?: boolean }) {
  return (
    <Link
      href={href}
      className={cn(
        "border px-3 py-1 font-mono text-xs uppercase tracking-wider transition-colors",
        active
          ? "border-foreground bg-foreground text-background"
          : "border-border text-muted-foreground hover:text-foreground hover:border-foreground"
      )}
    >
      <span className="mr-1">{active ? "▸" : " "}</span>{label}
    </Link>
  )
}

export default function ExplorerPage() {
  const { data: nodesData } = useNodes()
  const { data: roundsData } = useRounds()
  const { data: queueData } = useQueue()

  const nodes = nodesData?.nodes ?? []
  const rounds = roundsData?.rounds ?? []
  const queue = queueData

  const nodesOnline = nodes.length
  const roundsCompleted = rounds.length
  const finalized = rounds.filter((r) => r.status === "finalized").length
  const total = rounds.filter(
    (r) => r.status === "finalized" || r.status === "failed"
  ).length
  const successRate = total > 0 ? ((finalized / total) * 100).toFixed(1) : "—"
  const avgLatency =
    rounds.length > 0
      ? (rounds.reduce((sum, r) => sum + r.elapsed_ms, 0) / rounds.length).toFixed(0)
      : "—"
  const queueDepth = queue?.pending ?? 0

  const recentRounds = rounds.slice(0, 20)

  return (
    <div className="pb-12 space-y-10">
      {/* Page header */}
      <div className="flex items-end justify-between gap-4 flex-wrap">
        <h1 className="text-3xl md:text-4xl font-semibold tracking-tight">Explorer</h1>
        <div className="flex gap-2 text-xs font-mono">
          <TabLink href="/explorer" label="overview" active />
          <TabLink href="/explorer/rounds" label="rounds" />
          <TabLink href="/explorer/nodes" label="nodes" />
        </div>
      </div>

      {/* Stats — CornerBox grid */}
      <CornerBox title="readout · live · 5s_poll" tag="overview" innerClassName="p-0">
        <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-5 divide-x divide-y divide-border">
          <StatCell label="Nodes Online" value={String(nodesOnline)} live />
          <StatCell label="Rounds Completed" value={String(roundsCompleted)} />
          <StatCell label="Success Rate" value={successRate === "—" ? "—" : `${successRate}%`} />
          <StatCell label="Avg Latency" value={avgLatency === "—" ? "—" : `${avgLatency} ms`} />
          <StatCell label="Queue Depth" value={String(queueDepth)} />
        </div>
      </CornerBox>

      {/* Recent rounds */}
      <div className="space-y-4">
        <div className="flex items-center justify-between">
          <h2 className="ascii-label">// recent · rounds</h2>
          <Link href="/explorer/rounds" className="text-xs font-mono text-muted-foreground hover:text-foreground transition-colors">
            view all <span className="ml-1">›</span>
          </Link>
        </div>

        <CornerBox title="rounds · recent · n=20" innerClassName="p-0">
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
                {recentRounds.length === 0 ? (
                  <tr>
                    <td colSpan={6} className="text-center text-muted-foreground py-10">— no rounds yet —</td>
                  </tr>
                ) : (
                  recentRounds.map((round, i) => (
                    <motion.tr
                      key={`${round.request_id}-${round.timestamp}-${i}`}
                      initial={{ opacity: 0, y: 4 }}
                      animate={{ opacity: 1, y: 0 }}
                      transition={{ delay: i * 0.02 }}
                      className="border-b border-border last:border-0 hover:bg-muted/30 transition-colors"
                    >
                      <td className="px-3 py-2.5">
                        <div className="flex items-center gap-1.5">
                          <span className="text-foreground">{truncateId(round.request_id)}</span>
                          <CopyButton text={round.request_id} />
                        </div>
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
    </div>
  )
}
