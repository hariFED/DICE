"use client"

import { motion } from "framer-motion"
import { GlowCard } from "@/components/shared/GlowCard"
import { StatusBadge } from "@/components/shared/StatusBadge"
import { CopyButton } from "@/components/shared/CopyButton"
import { useNodes } from "@/lib/hooks"
import { useRounds } from "@/lib/hooks"
import { useQueue } from "@/lib/hooks"
import {
  Table,
  TableHeader,
  TableRow,
  TableHead,
  TableBody,
  TableCell,
} from "@/components/ui/table"

function truncateId(id: string) {
  if (id.length <= 16) return id
  return id.slice(0, 8) + "..." + id.slice(-4)
}

function formatElapsed(ms: number) {
  return (ms / 1000).toFixed(1) + "s"
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
  const successRate = total > 0 ? ((finalized / total) * 100).toFixed(1) : "--"
  const avgLatency =
    rounds.length > 0
      ? (
          rounds.reduce((sum, r) => sum + r.elapsed_ms, 0) / rounds.length
        ).toFixed(0)
      : "--"
  const queueDepth = queue?.pending ?? 0

  const recentRounds = rounds.slice(0, 20)

  return (
    <div className="py-8 space-y-8">
      {/* Stats Cards */}
      <div className="grid grid-cols-2 md:grid-cols-5 gap-4">
        <GlowCard className="p-5">
          <p className="text-xs uppercase text-zinc-500 tracking-wider">
            Nodes Online
          </p>
          <div className="flex items-center gap-2 mt-1">
            <span className="relative flex h-2 w-2">
              <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-[#00FF85] opacity-75" />
              <span className="relative inline-flex h-2 w-2 rounded-full bg-[#00FF85]" />
            </span>
            <span className="text-2xl font-bold text-[#00FF85]">
              {nodesOnline}
            </span>
          </div>
        </GlowCard>

        <GlowCard className="p-5">
          <p className="text-xs uppercase text-zinc-500 tracking-wider">
            Rounds Completed
          </p>
          <p className="text-2xl font-bold text-white mt-1">
            {roundsCompleted}
          </p>
        </GlowCard>

        <GlowCard className="p-5">
          <p className="text-xs uppercase text-zinc-500 tracking-wider">
            Success Rate
          </p>
          <p className="text-2xl font-bold text-white mt-1">
            {successRate === "--" ? "--" : `${successRate}%`}
          </p>
        </GlowCard>

        <GlowCard className="p-5">
          <p className="text-xs uppercase text-zinc-500 tracking-wider">
            Avg Latency
          </p>
          <p className="text-2xl font-bold text-white mt-1">
            {avgLatency === "--" ? "--" : `${avgLatency}ms`}
          </p>
        </GlowCard>

        <GlowCard className="p-5">
          <p className="text-xs uppercase text-zinc-500 tracking-wider">
            Queue Depth
          </p>
          <p className="text-2xl font-bold text-white mt-1">{queueDepth}</p>
        </GlowCard>
      </div>

      {/* Live Rounds Feed */}
      <div className="space-y-4">
        <div className="flex items-center gap-3">
          <h2 className="text-xl font-bold text-white">Recent Rounds</h2>
          <span className="relative flex h-2 w-2">
            <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-[#00FF85] opacity-75" />
            <span className="relative inline-flex h-2 w-2 rounded-full bg-[#00FF85]" />
          </span>
        </div>

        <GlowCard className="overflow-hidden">
          <Table className="bg-transparent">
            <TableHeader>
              <TableRow className="border-white/[0.06] hover:bg-transparent">
                <TableHead className="text-zinc-500">Request ID</TableHead>
                <TableHead className="text-zinc-500">Status</TableHead>
                <TableHead className="text-zinc-500">Nodes</TableHead>
                <TableHead className="text-zinc-500">Progress</TableHead>
                <TableHead className="text-zinc-500">Elapsed</TableHead>
                <TableHead className="text-zinc-500">Randomness</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {recentRounds.length === 0 ? (
                <TableRow className="border-white/[0.06]">
                  <TableCell
                    colSpan={6}
                    className="text-center text-zinc-600 py-8"
                  >
                    No rounds yet
                  </TableCell>
                </TableRow>
              ) : (
                recentRounds.map((round, i) => (
                  <motion.tr
                    key={round.request_id}
                    initial={{ opacity: 0, y: 8 }}
                    animate={{ opacity: 1, y: 0 }}
                    transition={{ delay: i * 0.03 }}
                    className="border-b border-white/[0.06] hover:bg-white/[0.02] transition-colors"
                  >
                    <TableCell>
                      <div className="flex items-center gap-1">
                        <span className="font-mono text-sm text-zinc-300">
                          {truncateId(round.request_id)}
                        </span>
                        <CopyButton text={round.request_id} />
                      </div>
                    </TableCell>
                    <TableCell>
                      <StatusBadge
                        status={
                          round.status as
                            | "finalized"
                            | "failed"
                            | "collecting_commits"
                            | "collecting_reveals"
                        }
                      />
                    </TableCell>
                    <TableCell className="text-zinc-300">
                      {round.node_count}
                    </TableCell>
                    <TableCell className="text-zinc-400 font-mono text-sm">
                      {round.commits_received}/{round.node_count} |{" "}
                      {round.reveals_received}/{round.node_count}
                    </TableCell>
                    <TableCell className="text-zinc-300">
                      {formatElapsed(round.elapsed_ms)}
                    </TableCell>
                    <TableCell>
                      {round.randomness ? (
                        <div className="flex items-center gap-1">
                          <span className="font-mono text-sm text-zinc-400">
                            {truncateId(round.randomness)}
                          </span>
                          <CopyButton text={round.randomness} />
                        </div>
                      ) : (
                        <span className="text-zinc-600">&mdash;</span>
                      )}
                    </TableCell>
                  </motion.tr>
                ))
              )}
            </TableBody>
          </Table>
        </GlowCard>
      </div>
    </div>
  )
}
