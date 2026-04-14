"use client"

import { useState, useMemo } from "react"
import { motion } from "framer-motion"
import { GlowCard } from "@/components/shared/GlowCard"
import { useNodes } from "@/lib/hooks"
import { cn } from "@/lib/utils"
import {
  Table,
  TableHeader,
  TableRow,
  TableHead,
  TableBody,
  TableCell,
} from "@/components/ui/table"
import type { Node } from "@/lib/types"

const CITY_MAP: Record<string, string> = {
  "node-01": "San Francisco",
  "node-02": "New York",
  "node-03": "London",
  "node-04": "Tokyo",
  "node-05": "Singapore",
  "node-06": "Sydney",
  "node-07": "Berlin",
  "node-08": "Toronto",
  "node-09": "Seoul",
  "node-10": "Mumbai",
  "node-11": "Sao Paulo",
  "node-12": "Dubai",
  "node-13": "Amsterdam",
  "node-14": "Paris",
  "node-15": "Hong Kong",
  "node-16": "Chicago",
  "node-17": "Zurich",
  "node-18": "Stockholm",
  "node-19": "Jakarta",
  "node-20": "Cape Town",
}

const FALLBACK_CITIES = [
  "San Francisco",
  "New York",
  "London",
  "Tokyo",
  "Singapore",
  "Sydney",
  "Berlin",
  "Toronto",
  "Seoul",
  "Mumbai",
  "Sao Paulo",
  "Dubai",
  "Amsterdam",
  "Paris",
  "Hong Kong",
  "Chicago",
  "Zurich",
  "Stockholm",
  "Jakarta",
  "Cape Town",
]

function getCity(nodeId: string, index: number): string {
  return CITY_MAP[nodeId] ?? FALLBACK_CITIES[index % FALLBACK_CITIES.length]
}

function truncateId(id: string) {
  if (id.length <= 16) return id
  return id.slice(0, 8) + "..." + id.slice(-4)
}

function formatUptime(secs: number) {
  const days = Math.floor(secs / 86400)
  const hours = Math.floor((secs % 86400) / 3600)
  return `${days}d ${hours}h`
}

type SortKey = "node_id" | "latency_ms" | "uptime_secs" | "jobs_completed"
type SortDir = "asc" | "desc"

export default function NodesPage() {
  const { data: nodesData } = useNodes()
  const [sortKey, setSortKey] = useState<SortKey>("node_id")
  const [sortDir, setSortDir] = useState<SortDir>("asc")

  const nodes = nodesData?.nodes ?? []
  const totalRegistered = 20
  const onlineCount = nodes.length

  const sorted = useMemo(() => {
    const copy = [...nodes]
    copy.sort((a, b) => {
      const av = a[sortKey]
      const bv = b[sortKey]
      if (typeof av === "string" && typeof bv === "string") {
        return sortDir === "asc"
          ? av.localeCompare(bv)
          : bv.localeCompare(av)
      }
      if (typeof av === "number" && typeof bv === "number") {
        return sortDir === "asc" ? av - bv : bv - av
      }
      return 0
    })
    return copy
  }, [nodes, sortKey, sortDir])

  function handleSort(key: SortKey) {
    if (sortKey === key) {
      setSortDir((d) => (d === "asc" ? "desc" : "asc"))
    } else {
      setSortKey(key)
      setSortDir("asc")
    }
  }

  function SortIndicator({ column }: { column: SortKey }) {
    if (sortKey !== column) return null
    return (
      <span className="ml-1 text-[#00FF85]">
        {sortDir === "asc" ? "\u2191" : "\u2193"}
      </span>
    )
  }

  return (
    <div className="py-8 space-y-6">
      <h1 className="text-3xl font-bold text-white">Nodes</h1>

      {/* Status summary */}
      <GlowCard className="p-5 space-y-3">
        <div className="flex items-center justify-between">
          <p className="text-sm text-zinc-400">
            <span className="text-[#00FF85] font-semibold">{onlineCount}</span>{" "}
            of {totalRegistered} nodes online
          </p>
          <p className="text-xs text-zinc-600">
            {((onlineCount / totalRegistered) * 100).toFixed(0)}%
          </p>
        </div>
        <div className="w-full h-2 bg-white/[0.06] rounded-full overflow-hidden">
          <motion.div
            className="h-full bg-[#00FF85] rounded-full"
            initial={{ width: 0 }}
            animate={{
              width: `${(onlineCount / totalRegistered) * 100}%`,
            }}
            transition={{ duration: 0.8, ease: "easeOut" }}
          />
        </div>
      </GlowCard>

      {/* Nodes table */}
      <GlowCard className="overflow-hidden">
        <Table className="bg-transparent">
          <TableHeader>
            <TableRow className="border-white/[0.06] hover:bg-transparent">
              <TableHead
                className="text-zinc-500 cursor-pointer select-none"
                onClick={() => handleSort("node_id")}
              >
                Device ID
                <SortIndicator column="node_id" />
              </TableHead>
              <TableHead className="text-zinc-500">Location</TableHead>
              <TableHead className="text-zinc-500">Status</TableHead>
              <TableHead
                className="text-zinc-500 cursor-pointer select-none"
                onClick={() => handleSort("latency_ms")}
              >
                Latency
                <SortIndicator column="latency_ms" />
              </TableHead>
              <TableHead
                className="text-zinc-500 cursor-pointer select-none"
                onClick={() => handleSort("uptime_secs")}
              >
                Uptime
                <SortIndicator column="uptime_secs" />
              </TableHead>
              <TableHead
                className="text-zinc-500 cursor-pointer select-none"
                onClick={() => handleSort("jobs_completed")}
              >
                Jobs Completed
                <SortIndicator column="jobs_completed" />
              </TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {sorted.length === 0 ? (
              <TableRow className="border-white/[0.06]">
                <TableCell
                  colSpan={6}
                  className="text-center text-zinc-600 py-8"
                >
                  No nodes connected
                </TableCell>
              </TableRow>
            ) : (
              sorted.map((node, i) => (
                <motion.tr
                  key={node.node_id}
                  initial={{ opacity: 0, y: 6 }}
                  animate={{ opacity: 1, y: 0 }}
                  transition={{ delay: i * 0.02 }}
                  className="border-b border-white/[0.06] hover:bg-white/[0.02] transition-colors"
                >
                  <TableCell>
                    <span className="font-mono text-sm text-zinc-300">
                      {truncateId(node.node_id)}
                    </span>
                  </TableCell>
                  <TableCell className="text-zinc-400 text-sm">
                    {getCity(node.node_id, i)}
                  </TableCell>
                  <TableCell>
                    <div className="flex items-center gap-2">
                      <span className="relative flex h-2 w-2">
                        <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-[#00FF85] opacity-75" />
                        <span className="relative inline-flex h-2 w-2 rounded-full bg-[#00FF85]" />
                      </span>
                      <span className="text-sm text-[#00FF85]">Online</span>
                    </div>
                  </TableCell>
                  <TableCell className="text-zinc-300 text-sm">
                    {node.latency_ms}ms
                  </TableCell>
                  <TableCell className="text-zinc-300 text-sm">
                    {formatUptime(node.uptime_secs)}
                  </TableCell>
                  <TableCell className="text-zinc-300 text-sm">
                    {node.jobs_completed}
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
