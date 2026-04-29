"use client"

import { useState, useMemo } from "react"
import Link from "next/link"
import { useNodes } from "@/lib/hooks"
import { cn } from "@/lib/utils"
import { CornerBox } from "@/components/shared/CornerBox"
import { AsciiBar } from "@/components/shared/AsciiBar"
import { TableRowSkeleton } from "@/components/shared/Skeleton"

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

const FALLBACK_CITIES = Object.values(CITY_MAP)

function getCity(nodeId: string, index: number): string {
  return CITY_MAP[nodeId] ?? FALLBACK_CITIES[index % FALLBACK_CITIES.length]
}

function truncateId(id: string) {
  if (id.length <= 16) return id
  return id.slice(0, 8) + "…" + id.slice(-4)
}

function formatUptime(secs: number) {
  const days = Math.floor(secs / 86400)
  const hours = Math.floor((secs % 86400) / 3600)
  return `${days}d ${hours}h`
}

type SortKey = "node_id" | "latency_ms" | "uptime_secs" | "jobs_completed"
type SortDir = "asc" | "desc"

export default function NodesPage() {
  const { data: nodesData, isLoading } = useNodes()
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
        return sortDir === "asc" ? av.localeCompare(bv) : bv.localeCompare(av)
      }
      if (typeof av === "number" && typeof bv === "number") {
        return sortDir === "asc" ? av - bv : bv - av
      }
      return 0
    })
    return copy
  }, [nodes, sortKey, sortDir])

  function handleSort(key: SortKey) {
    if (sortKey === key) setSortDir((d) => (d === "asc" ? "desc" : "asc"))
    else { setSortKey(key); setSortDir("asc") }
  }

  function SortHeader({ column, label }: { column: SortKey; label: string }) {
    const active = sortKey === column
    return (
      <button
        onClick={() => handleSort(column)}
        className="ascii-label text-[10px] hover:text-foreground transition-colors inline-flex items-center gap-1"
      >
        <span>{label}</span>
        {active && <span className="text-foreground">{sortDir === "asc" ? "↑" : "↓"}</span>}
      </button>
    )
  }

  const pct = (onlineCount / totalRegistered) * 100

  return (
    <div className="pb-12 space-y-6">
      <div className="flex items-end justify-between gap-4 flex-wrap">
        <h1 className="text-3xl md:text-4xl font-semibold tracking-tight">Nodes</h1>
        <div className="flex gap-2 text-xs font-mono">
          <Link href="/explorer" className="border border-border text-muted-foreground hover:text-foreground hover:border-foreground transition-colors px-3 py-1 uppercase tracking-wider"><span className="mr-1"> </span>overview</Link>
          <Link href="/explorer/rounds" className="border border-border text-muted-foreground hover:text-foreground hover:border-foreground transition-colors px-3 py-1 uppercase tracking-wider"><span className="mr-1"> </span>rounds</Link>
          <Link href="/explorer/nodes" className="border border-foreground bg-foreground text-background px-3 py-1 uppercase tracking-wider"><span className="mr-1">▸</span>nodes</Link>
        </div>
      </div>

      {/* Online / total ASCII bar */}
      <CornerBox title="fleet · uptime" innerClassName="p-5">
        <div className="flex items-center justify-between mb-2 flex-wrap gap-2">
          <p className="font-mono text-sm">
            <span className="text-foreground font-semibold tabular-nums">{onlineCount}</span>
            <span className="text-muted-foreground"> / {totalRegistered} nodes online</span>
          </p>
          <p className="ascii-label text-[10px]">{pct.toFixed(0)} pct</p>
        </div>
        <div className="text-sm">
          <AsciiBar value={pct / 100} width={48} />
        </div>
      </CornerBox>

      {/* Nodes table */}
      <CornerBox title={`fleet · n=${sorted.length}`} innerClassName="p-0">
        <div className="overflow-x-auto">
        <table className="w-full text-sm font-mono">
          <thead>
            <tr className="border-b border-border bg-muted/30">
              <th className="text-left px-3 py-2"><SortHeader column="node_id" label="device_id" /></th>
              <th className="text-left px-3 py-2 ascii-label text-[10px]">location</th>
              <th className="text-left px-3 py-2 ascii-label text-[10px]">status</th>
              <th className="text-left px-3 py-2"><SortHeader column="latency_ms" label="latency" /></th>
              <th className="text-left px-3 py-2"><SortHeader column="uptime_secs" label="uptime" /></th>
              <th className="text-left px-3 py-2"><SortHeader column="jobs_completed" label="jobs" /></th>
            </tr>
          </thead>
          <tbody>
            {isLoading ? (
              Array.from({ length: 8 }).map((_, i) => (
                <TableRowSkeleton key={i} cols={6} />
              ))
            ) : sorted.length === 0 ? (
              <tr><td colSpan={6} className="text-center text-muted-foreground py-10">— no nodes connected —</td></tr>
            ) : (
              sorted.map((node, i) => (
                <tr
                  key={node.node_id}
                  className="border-b border-border last:border-0 hover:bg-muted/30 transition-colors"
                >
                  <td className="px-3 py-2.5 text-foreground">{truncateId(node.node_id)}</td>
                  <td className="px-3 py-2.5 text-muted-foreground">{getCity(node.node_id, i)}</td>
                  <td className="px-3 py-2.5">
                    <span className={cn("inline-flex items-center gap-1.5 rounded-sm px-2 py-0.5 text-[10px] uppercase tracking-wider pill-ok")}>
                      <span>●</span> ONLINE
                    </span>
                  </td>
                  <td className="px-3 py-2.5 text-foreground tabular-nums">{node.latency_ms}ms</td>
                  <td className="px-3 py-2.5 text-muted-foreground">{formatUptime(node.uptime_secs)}</td>
                  <td className="px-3 py-2.5 text-foreground tabular-nums">{node.jobs_completed}</td>
                </tr>
              ))
            )}
          </tbody>
        </table>
        </div>
      </CornerBox>
    </div>
  )
}
