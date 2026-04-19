"use client"

/**
 * Hi-resolution rotating ASCII sphere — 128×64 grid for a crisp centerpiece.
 * Markers pulse on a subtle phase offset per city so the fleet "breathes".
 *
 * Algorithm: classic shaded-sphere raymarch (see small version for comments).
 * Key differences from the small version:
 *   - Bigger grid for legibility at display sizes.
 *   - Wider char ramp with Unicode block chars → smoother gradient.
 *   - Markers use a 3-frame pulse: '●' → '◉' → '○' → '◉' so they blink at ~1.2s.
 *   - Exposes `className` so the parent controls sizing (responsive text-*).
 */
import { useEffect, useRef } from "react"

type Marker = [number, number] // [lat, lon] in degrees

const MARKERS: Marker[] = [
  [37.77, -122.42],  // SF
  [40.71, -74.01],   // NYC
  [51.51, -0.13],    // London
  [48.86, 2.35],     // Paris
  [52.52, 13.41],    // Berlin
  [35.68, 139.65],   // Tokyo
  [1.35, 103.82],    // Singapore
  [22.32, 114.17],   // Hong Kong
  [-33.87, 151.21],  // Sydney
  [55.76, 37.62],    // Moscow
  [25.20, 55.27],    // Dubai
  [19.08, 72.88],    // Mumbai
  [-23.55, -46.63],  // São Paulo
  [-34.60, -58.38],  // Buenos Aires
  [28.61, 77.21],    // New Delhi
  [47.61, -122.33],  // Seattle
  [43.65, -79.38],   // Toronto
  [50.08, 14.44],    // Prague
  [59.33, 18.07],    // Stockholm
  [34.05, -118.24],  // LA
]

// Wider shade ramp — unicode block chars give a smoother gradient.
const RAMP = " .·:-=+*xo#▒▓█"
const COLS = 110
const ROWS = 60
const PULSE_FRAMES = ["●", "◉", "○", "◉"]

/**
 * Monospace char aspect = (char_width / line_height).
 * For Geist Mono at our `leading-[0.95]`, ≈ 0.55. We use this to stretch
 * the sphere's horizontal range so the rendered disc is an actual circle,
 * not the crescent-looking oval you get with the naive 1:1 assumption.
 */
const CHAR_ASPECT = 0.55

function latLonToVec3(lat: number, lon: number): [number, number, number] {
  const phi = (lat * Math.PI) / 180
  const lambda = (lon * Math.PI) / 180
  return [
    Math.cos(phi) * Math.cos(lambda),
    Math.sin(phi),
    Math.cos(phi) * Math.sin(lambda),
  ]
}

function rotateY([x, y, z]: [number, number, number], a: number): [number, number, number] {
  const c = Math.cos(a), s = Math.sin(a)
  return [c * x + s * z, y, -s * x + c * z]
}

function rotateX([x, y, z]: [number, number, number], a: number): [number, number, number] {
  const c = Math.cos(a), s = Math.sin(a)
  return [x, c * y - s * z, s * y + c * z]
}

export function AsciiGlobe({ className = "" }: { className?: string }) {
  const ref = useRef<HTMLPreElement>(null)
  const angleRef = useRef(0)

  useEffect(() => {
    let rafId: number
    let lastTime = performance.now()
    const tilt = 0.45
    const light: [number, number, number] = [0.6, 0.55, 0.7]
    const lightLen = Math.hypot(...light)
    const lN: [number, number, number] = [light[0] / lightLen, light[1] / lightLen, light[2] / lightLen]

    const grid: string[] = new Array(ROWS)

    // Horizontal u-range so disc ends up visually circular on screen.
    // u ∈ [-u_max, u_max]; the sphere (r ≤ 1) is centered inside that range.
    const u_max = (COLS * CHAR_ASPECT) / ROWS

    function frame(now: number) {
      const dt = (now - lastTime) / 1000
      lastTime = now
      angleRef.current += dt * 0.3 // ~21s per revolution — slower, more hypnotic

      const angle = angleRef.current
      const pulsePhase = Math.floor((now / 300) % PULSE_FRAMES.length)

      // 1. Terrain.
      for (let r = 0; r < ROWS; r++) {
        let line = ""
        const v = (r / (ROWS - 1)) * 2 - 1
        for (let c = 0; c < COLS; c++) {
          const u = ((c / (COLS - 1)) * 2 - 1) * u_max
          const x0 = u
          const y0 = v
          const r2 = x0 * x0 + y0 * y0
          if (r2 > 1) {
            line += " "
            continue
          }
          const z0 = Math.sqrt(1 - r2)
          const rotated = rotateX(rotateY([x0, y0, z0], angle), tilt)
          const dot = rotated[0] * lN[0] + rotated[1] * lN[1] + rotated[2] * lN[2]
          const shade = Math.max(0, Math.min(1, dot))
          const idx = Math.floor(shade * (RAMP.length - 1))
          line += RAMP[idx]
        }
        grid[r] = line
      }

      // 2. Markers — per-city phase offset so they don't all pulse together.
      MARKERS.forEach(([lat, lon], i) => {
        const v3 = rotateX(rotateY(latLonToVec3(lat, lon), angle), tilt)
        const [mx, my, mz] = v3
        if (mz < 0.05) return
        // Inverse of the projection in the terrain loop.
        const sc = Math.round(((mx / u_max + 1) / 2) * (COLS - 1))
        const sr = Math.round(((my + 1) / 2) * (ROWS - 1))
        if (sc < 0 || sc >= COLS || sr < 0 || sr >= ROWS) return
        const row = grid[sr]
        const glyph = PULSE_FRAMES[(pulsePhase + i) % PULSE_FRAMES.length]
        grid[sr] = row.slice(0, sc) + glyph + row.slice(sc + 1)
      })

      const out = grid.join("\n")
      if (ref.current) ref.current.textContent = out
      rafId = requestAnimationFrame(frame)
    }

    rafId = requestAnimationFrame(frame)
    return () => cancelAnimationFrame(rafId)
  }, [])

  return (
    <pre
      ref={ref}
      aria-hidden
      className={`font-mono leading-[0.95] tracking-[0.02em] text-foreground select-none whitespace-pre ${className}`}
      style={{ fontFamily: "var(--font-mono), ui-monospace, monospace" }}
    >
      {Array(ROWS).fill(" ".repeat(COLS)).join("\n")}
    </pre>
  )
}

export default AsciiGlobe
