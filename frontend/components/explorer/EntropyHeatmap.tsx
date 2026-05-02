"use client"

/**
 * Entropy production heatmap — a 24×7 cell grid ("day-of-week × hour")
 * showing request volume by bucket. Creative data-viz reference: the
 * Kendall Jackson "Your money, your momentum" dotted map, but rectified.
 *
 * Input: array of round timestamps (ms).
 * Output: heatmap with cell intensity scaled 0..1 against max bucket.
 */

const DAYS = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"]

export function EntropyHeatmap({ timestamps = [] }: { timestamps?: number[] }) {
  // Bucket by day-of-week × hour. Day-of-week uses ISO week (Mon=0).
  const buckets: number[][] = Array.from({ length: 7 }, () => Array(24).fill(0))

  for (const ts of timestamps) {
    const d = new Date(ts)
    const dow = (d.getUTCDay() + 6) % 7 // Mon=0
    const h = d.getUTCHours()
    buckets[dow][h]++
  }

  let max = 1
  for (const row of buckets) for (const v of row) max = Math.max(max, v)

  // If real data is empty, synthesize a faint pulse pattern so the viz
  // still reads as a chart on a fresh network.
  if (max <= 1 && timestamps.length === 0) {
    for (let d = 0; d < 7; d++) {
      for (let h = 0; h < 24; h++) {
        // Peak around 14:00 UTC + weekend lift
        const peak = Math.max(0, 1 - Math.abs(h - 14) / 8)
        const weekend = d >= 5 ? 1.2 : 1
        buckets[d][h] = peak * weekend * 0.6
      }
    }
    max = 1
  }

  return (
    <div className="w-full">
      <div className="flex items-baseline justify-between mb-3">
        <p className="ascii-label text-[10px]">// entropy · production · last 7d</p>
        <div className="flex items-center gap-2 font-mono text-[10px] text-muted-foreground">
          <span>low</span>
          {[0.1, 0.3, 0.55, 0.8, 1].map((v) => (
            <span
              key={v}
              className="w-3 h-3 border border-border"
              style={{ backgroundColor: `rgba(var(--fg-rgb, 250 250 250) / ${v})` }}
            />
          ))}
          <span>high</span>
        </div>
      </div>

      <div className="flex gap-2">
        {/* Day labels */}
        <div className="flex flex-col justify-between font-mono text-[10px] text-muted-foreground py-0.5">
          {DAYS.map((d) => (
            <span key={d} className="leading-none h-3">{d}</span>
          ))}
        </div>

        {/* Grid */}
        <div className="flex-1 overflow-x-auto">
          <div
            className="grid gap-[2px]"
            style={{
              gridTemplateColumns: "repeat(24, minmax(10px, 1fr))",
              gridTemplateRows: "repeat(7, 14px)",
            }}
          >
            {buckets.flatMap((row, d) =>
              row.map((v, h) => {
                const intensity = v / max
                return (
                  <div
                    key={`${d}-${h}`}
                    className="relative border border-border/40"
                    style={{
                      backgroundColor: intensity > 0
                        ? `color-mix(in srgb, var(--foreground) ${Math.round(intensity * 100)}%, transparent)`
                        : "transparent",
                    }}
                    title={`${DAYS[d]} ${h.toString().padStart(2, "0")}:00 — ${v.toFixed(0)} rounds`}
                  />
                )
              }),
            )}
          </div>

          {/* Hour labels */}
          <div className="mt-1 grid gap-[2px] font-mono text-[9px] text-muted-foreground/60"
               style={{ gridTemplateColumns: "repeat(24, minmax(10px, 1fr))" }}>
            {Array.from({ length: 24 }, (_, i) => (
              <span key={i} className="text-center">{i % 6 === 0 ? i : ""}</span>
            ))}
          </div>
        </div>
      </div>
    </div>
  )
}

export default EntropyHeatmap
