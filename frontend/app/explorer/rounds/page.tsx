"use client"

import { useState, useMemo } from "react"
import Link from "next/link"
import { motion } from "framer-motion"
import { GlowCard } from "@/components/shared/GlowCard"
import { StatusBadge } from "@/components/shared/StatusBadge"
import { CopyButton } from "@/components/shared/CopyButton"
import { useRounds } from "@/lib/hooks"
import {
  Table,
  TableHeader,
  TableRow,
  TableHead,
  TableBody,
  TableCell,
} from "@/components/ui/table"
import type { Round } from "@/lib/types"

const FILTERS = [
  { key: "all", label: "All" },
  { key: "finalized", label: "Finalized" },
  { key: "failed", label: "Failed" },
  { key: "in_progress", label: "In Progress" },
] as const

type FilterKey = (typeof FILTERS)[number]["key"]

function truncateId(id: string) {
  if (id.length <= 16) return id
  return id.slice(0, 8) + "..." + id.slice(-4)
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
    <div className="py-8 space-y-6">
      <h1 className="text-3xl font-bold text-white">Rounds</h1>

      {/* Filter tabs */}
      <div className="flex items-center gap-2">
        {FILTERS.map((f) => (
          <button
            key={f.key}
            onClick={() => setFilter(f.key)}
            className={`px-4 py-1.5 rounded-full text-sm font-medium transition-all duration-200 ${
              filter === f.key
                ? "bg-[#00FF85]/15 text-[#00FF85] border border-[#00FF85]/30"
                : "bg-white/[0.04] text-zinc-400 border border-white/[0.08] hover:bg-white/[0.06] hover:text-zinc-300"
            }`}
          >
            {f.label}
          </button>
        ))}
      </div>

      {/* Rounds table */}
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
            {filtered.length === 0 ? (
              <TableRow className="border-white/[0.06]">
                <TableCell
                  colSpan={6}
                  className="text-center text-zinc-600 py-8"
                >
                  No rounds found
                </TableCell>
              </TableRow>
            ) : (
              filtered.map((round, i) => (
                <motion.tr
                  // v2 channel path: request_id (=channel pubkey) repeats
                  // across rounds on one channel — compose with timestamp
                  // + index for a stable per-row identity.
                  key={`${round.request_id}-${round.timestamp}-${i}`}
                  initial={{ opacity: 0, y: 6 }}
                  animate={{ opacity: 1, y: 0 }}
                  transition={{ delay: i * 0.02 }}
                  className="border-b border-white/[0.06] hover:bg-white/[0.02] transition-colors"
                >
                  <TableCell>
                    <Link
                      href={`/explorer/rounds/${round.request_id}`}
                      className="flex items-center gap-1 group"
                    >
                      <span className="font-mono text-sm text-zinc-300 group-hover:text-[#00FF85] transition-colors">
                        {truncateId(round.request_id)}
                      </span>
                      <CopyButton text={round.request_id} />
                    </Link>
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
  )
}
