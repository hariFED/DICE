import { cn } from "@/lib/utils"

interface AsciiBarProps {
  /** Value in [0, 1] (or [0, 100] if `percentage`). */
  value: number
  /** Treat `value` as a 0-100 percentage rather than 0-1 ratio. */
  percentage?: boolean
  /** Total cells in the bar. Default 24. */
  width?: number
  /** Filled char (default █). */
  fill?: string
  /** Empty char (default ░). */
  empty?: string
  /** Wrap with brackets:  [████████░░░░░░░░] */
  brackets?: boolean
  className?: string
}

/**
 * Pure-text bar chart:  [████████░░░░░░░░]  67%
 * Renders inline; no SVG, no images.
 */
export function AsciiBar({
  value,
  percentage,
  width = 24,
  fill = "█",
  empty = "░",
  brackets = true,
  className,
}: AsciiBarProps) {
  const ratio = Math.max(0, Math.min(1, percentage ? value / 100 : value))
  const filled = Math.round(ratio * width)
  const bar = fill.repeat(filled) + empty.repeat(Math.max(0, width - filled))
  return (
    <span className={cn("font-mono text-foreground tabular-nums select-none", className)}>
      {brackets && <span className="text-muted-foreground">[ </span>}
      {bar}
      {brackets && <span className="text-muted-foreground"> ]</span>}
    </span>
  )
}

export default AsciiBar
