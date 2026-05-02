"use client"

import { useEffect, useRef } from "react"

/**
 * Drifting low-opacity ASCII rain — pure CSS char animation, no DOM updates.
 * Renders a fixed grid of monospace characters and slowly translates the whole
 * thing diagonally with `transform`. Replaces what would normally be a
 * particle field or noise SVG. Cheap (one rAF, no per-char redraws).
 *
 * Uses `var(--ascii-bg)` so it adapts to light/dark theme automatically.
 */

const CHARS = "·:-+*#░▒▓·"
const COLS = 120
const ROWS = 50

function genGrid() {
  let out = ""
  for (let r = 0; r < ROWS; r++) {
    let line = ""
    for (let c = 0; c < COLS; c++) {
      // Sparse: ~75% spaces. Bias toward lighter chars.
      const roll = Math.random()
      if (roll > 0.25) line += " "
      else line += CHARS[Math.floor(Math.random() * CHARS.length)]
    }
    out += line + "\n"
  }
  return out
}

export function AsciiBackground({ className = "" }: { className?: string }) {
  const ref = useRef<HTMLPreElement>(null)
  const offset = useRef(0)

  useEffect(() => {
    if (!ref.current) return
    ref.current.textContent = genGrid()

    let rafId: number
    let last = performance.now()

    function tick(now: number) {
      const dt = (now - last) / 1000
      last = now
      offset.current += dt * 8 // px/sec drift

      if (ref.current) {
        ref.current.style.transform = `translate3d(${-offset.current % 200}px, ${-offset.current % 200}px, 0)`
      }
      rafId = requestAnimationFrame(tick)
    }

    rafId = requestAnimationFrame(tick)
    return () => cancelAnimationFrame(rafId)
  }, [])

  return (
    <div
      aria-hidden
      className={`pointer-events-none absolute inset-0 overflow-hidden select-none ${className}`}
    >
      <pre
        ref={ref}
        className="absolute -inset-[200px] font-mono text-[10px] leading-[1.1] whitespace-pre"
        style={{
          color: "var(--ascii-bg)",
          fontFamily: "var(--font-mono), ui-monospace, monospace",
          willChange: "transform",
        }}
      />
    </div>
  )
}

export default AsciiBackground
