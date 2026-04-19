"use client"

/**
 * 20-node "LED strip" visualization. Each cell is a node, color intensity
 * is its recent contribution count. Ref: the Kendall Jackson "Your money,
 * your momentum" layout. Cells pulse subtly if the node is online.
 */

type NodeSummary = {
  id: string
  online: boolean
  rounds: number
}

export function NodeMapStrip({ nodes = [] }: { nodes?: NodeSummary[] }) {
  // Ensure 20 cells — pad with placeholder offline nodes if needed
  const cells: NodeSummary[] = [...nodes]
  while (cells.length < 20) cells.push({ id: `n${cells.length.toString().padStart(2, "0")}`, online: false, rounds: 0 })
  cells.splice(20)
  const maxRounds = Math.max(1, ...cells.map((n) => n.rounds))

  return (
    <div>
      <div className="flex items-baseline justify-between mb-3">
        <p className="ascii-label text-[10px]">// mesh · 20 · nodes</p>
        <span className="font-mono text-[10px] text-muted-foreground">
          online · {cells.filter((c) => c.online).length} / 20
        </span>
      </div>

      <div className="grid grid-cols-10 sm:grid-cols-20 gap-[3px]">
        {cells.map((n, i) => {
          const intensity = n.online ? Math.max(0.15, n.rounds / maxRounds) : 0
          return (
            <div
              key={n.id + i}
              className={`relative aspect-square border border-border/60 ${n.online ? "" : "opacity-30"}`}
              style={{
                backgroundColor: intensity > 0
                  ? `color-mix(in srgb, var(--foreground) ${Math.round(intensity * 100)}%, transparent)`
                  : "transparent",
              }}
              title={`node ${n.id} · ${n.online ? "online" : "offline"} · ${n.rounds} rounds`}
            >
              {n.online && (
                <span className="absolute bottom-0.5 right-0.5 w-1 h-1 rounded-full bg-[var(--status-ok)] animate-pulse" />
              )}
            </div>
          )
        })}
      </div>
    </div>
  )
}

export default NodeMapStrip
