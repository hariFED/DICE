"use client"

import { motion } from "framer-motion"

/**
 * ESP32-S3 board art — every line is exactly 59 chars wide so the box aligns
 * regardless of what renders at each cell. No colored glyphs, no proportional
 * tricks; just plain mono.
 *
 * Layout (top-down silkscreen view):
 *   - antenna block at top-right
 *   - USB-C at left
 *   - SoC block in center with pin indicators above + below
 *   - LEDs at right
 *   - GPIO header row beneath
 *   - passives (R1 R2 R3 C1 XTAL)
 *   - BOOT / RST buttons
 *   - corner rev label
 */

const BOARD_LINES = [
  "┌─────────────────────────────────────────────────────────┐",
  "│                                                         │",
  "│                                ≈ ≈ ≈ ≈ ≈ ≈              │",
  "│                                  WIFI·ANT               │",
  "│                                                         │",
  "│   ┌─────┐          ┌───────────────────┐       ◉ PWR    │",
  "│   │ USB │══════════│    ESP32-S3       │       ◉ STAT   │",
  "│   │  C  │          │    N16R8  · QFN   │       ◉ ENT    │",
  "│   └─────┘          └───────────────────┘       ◉ NET    │",
  "│                                                         │",
  "│   GPIO   01  02  03  04  05  06  07  08  09  10         │",
  "│          │   │   │   │   │   │   │   │   │   │          │",
  "│                                                         │",
  "│   [R1]   [R2]   [R3]   [C1]   [XTAL]                    │",
  "│                                                         │",
  "│   ◇ BOOT        ◇ RST                 DICE·S3·v1        │",
  "│                                                         │",
  "└─────────────────────────────────────────────────────────┘",
]

export function AsciiEsp32({ className = "" }: { className?: string }) {
  return (
    <pre
      className={`font-mono text-[8px] xs:text-[10px] sm:text-[11px] md:text-xs leading-[1.25] text-foreground select-none whitespace-pre ${className}`}
      style={{ fontFamily: "var(--font-mono), ui-monospace, monospace" }}
      aria-label="ASCII illustration of the DICE ESP32-S3 entropy node"
    >
      {BOARD_LINES.map((line, i) => (
        <motion.span
          key={i}
          className="block"
          initial={{ opacity: 0, x: -4 }}
          whileInView={{ opacity: 1, x: 0 }}
          viewport={{ once: true, margin: "-80px" }}
          transition={{ delay: i * 0.025, duration: 0.3, ease: "easeOut" }}
        >
          {line}
        </motion.span>
      ))}
    </pre>
  )
}

export default AsciiEsp32
