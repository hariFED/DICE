"use client"

import { useStats } from "@/lib/hooks"

/**
 * Full-width live ticker that scrolls stats across the top of the page.
 * Modeled after financial-exchange tickers + magazine headline marquees.
 * Duplicates content 3× so the animation can loop seamlessly at any width.
 */
export function StatsTicker() {
  const { data: stats } = useStats()

  const items: [string, string][] = [
    ["solana", "devnet"],
    ["dice", "v7.7"],
    ["nodes", stats ? String(stats.nodes_online) : "—"],
    ["rounds", stats ? stats.total_rounds.toLocaleString() : "—"],
    [
      "success",
      stats && typeof stats.success_rate === "number"
        ? `${(stats.success_rate <= 1 ? stats.success_rate * 100 : stats.success_rate).toFixed(1)}%`
        : "—",
    ],
    ["latency", stats ? `${stats.avg_latency_ms}ms` : "—"],
    ["cost", "0.002 SOL"],
    ["protocol", "commit-reveal"],
    ["entropy", "hardware · trng"],
  ]

  const block = (
    <span className="flex shrink-0 items-center">
      {items.map(([k, v], i) => (
        <span key={i} className="flex items-center shrink-0">
          <span className="px-4 py-2 flex items-center gap-2">
            <span className="text-muted-foreground/60 font-mono text-[10px] uppercase tracking-wider">{k}</span>
            <span className="text-foreground font-mono text-xs">{v}</span>
          </span>
          <span aria-hidden className="text-muted-foreground/30 select-none font-mono">▓▒░</span>
        </span>
      ))}
    </span>
  )

  return (
    <div className="sticky top-[3.5rem] z-40 border-y border-dashed border-border bg-background overflow-hidden">
      <div className="relative flex whitespace-nowrap">
        <div className="flex shrink-0" style={{ animation: "ticker 60s linear infinite" }}>
          {block}
          {block}
        </div>
        <div className="flex shrink-0" style={{ animation: "ticker 60s linear infinite" }} aria-hidden>
          {block}
          {block}
        </div>
      </div>
      <style jsx>{`
        @keyframes ticker {
          from { transform: translateX(0); }
          to { transform: translateX(-50%); }
        }
      `}</style>
    </div>
  )
}
