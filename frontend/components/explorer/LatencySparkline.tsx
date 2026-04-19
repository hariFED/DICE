"use client"

/**
 * Latency sparkline — draw last N rounds' elapsed_ms as a tiny line
 * chart with fill. Markers for the min/max data points.
 */

export function LatencySparkline({
  points = [],
  width = 620,
  height = 120,
}: {
  points?: number[]
  width?: number
  height?: number
}) {
  const data = points.length > 0 ? points : syntheticFill(50)
  const min = Math.min(...data)
  const max = Math.max(...data)
  const span = Math.max(1, max - min)
  const stepX = data.length > 1 ? width / (data.length - 1) : 0

  const pts = data.map((v, i) => {
    const x = i * stepX
    const y = height - 8 - ((v - min) / span) * (height - 16)
    return [x, y] as [number, number]
  })

  const linePath = pts.map((p, i) => `${i === 0 ? "M" : "L"} ${p[0].toFixed(1)} ${p[1].toFixed(1)}`).join(" ")
  const areaPath = `${linePath} L ${width} ${height} L 0 ${height} Z`

  const minIdx = data.indexOf(min)
  const maxIdx = data.indexOf(max)
  const avg = data.reduce((s, v) => s + v, 0) / data.length

  return (
    <div className="w-full">
      <div className="flex items-baseline justify-between mb-3">
        <p className="ascii-label text-[10px]">// latency · last {data.length} rounds</p>
        <div className="flex items-center gap-4 font-mono text-[10px] text-muted-foreground">
          <span>min <span className="text-foreground tabular-nums">{min.toFixed(0)} ms</span></span>
          <span>avg <span className="text-foreground tabular-nums">{avg.toFixed(0)} ms</span></span>
          <span>max <span className="text-foreground tabular-nums">{max.toFixed(0)} ms</span></span>
        </div>
      </div>

      <svg viewBox={`0 0 ${width} ${height}`} className="w-full h-auto" style={{ color: "var(--foreground)" }}>
        {/* y grid */}
        {[0.25, 0.5, 0.75].map((f, i) => (
          <line
            key={i}
            x1="0"
            x2={width}
            y1={height * f}
            y2={height * f}
            stroke="currentColor"
            strokeWidth="0.4"
            strokeDasharray="3 3"
            opacity="0.15"
          />
        ))}

        {/* area */}
        <path d={areaPath} fill="currentColor" opacity="0.08" />
        {/* line */}
        <path d={linePath} fill="none" stroke="currentColor" strokeWidth="1.3" />

        {/* min / max markers */}
        <circle cx={pts[minIdx][0]} cy={pts[minIdx][1]} r="3" fill="var(--background)" stroke="currentColor" strokeWidth="1.2" />
        <text x={pts[minIdx][0] + 6} y={pts[minIdx][1] + 3} fontFamily="var(--font-mono)" fontSize="9" fill="currentColor" opacity="0.7">↓ {min.toFixed(0)}</text>
        <circle cx={pts[maxIdx][0]} cy={pts[maxIdx][1]} r="3" fill="currentColor" />
        <text x={pts[maxIdx][0] + 6} y={pts[maxIdx][1] + 3} fontFamily="var(--font-mono)" fontSize="9" fill="currentColor" opacity="0.85">↑ {max.toFixed(0)}</text>
      </svg>
    </div>
  )
}

function syntheticFill(n: number): number[] {
  const out: number[] = []
  for (let i = 0; i < n; i++) {
    const base = 3800 + Math.sin(i / 5) * 200
    const noise = (Math.random() - 0.5) * 400
    out.push(Math.max(2500, base + noise))
  }
  return out
}

export default LatencySparkline
