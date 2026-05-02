import { cn } from "@/lib/utils"

type Status =
  | "finalized"
  | "failed"
  | "collecting_commits"
  | "collecting_reveals"
  | "idle"

interface StatusBadgeProps {
  status: Status
  className?: string
}

/**
 * ASCII-style status pill. The ONLY place in the app that uses semantic color
 * (var(--status-*)). Everything else stays mono.
 *
 *   [ ● FINALIZED ]   [ ◌ COMMITS ]   [ ✕ FAILED ]
 */
const STATUS_CONFIG: Record<Status, { label: string; glyph: string; pillClass: string; pulse?: boolean }> = {
  finalized:          { label: "FINALIZED", glyph: "●", pillClass: "pill-ok" },
  failed:             { label: "FAILED",    glyph: "✕", pillClass: "pill-err" },
  collecting_commits: { label: "COMMITS",   glyph: "◌", pillClass: "pill-warn", pulse: true },
  collecting_reveals: { label: "REVEALS",   glyph: "◔", pillClass: "pill-warn", pulse: true },
  idle:               { label: "IDLE",      glyph: "·", pillClass: "" },
}

export function StatusBadge({ status, className }: StatusBadgeProps) {
  const cfg = STATUS_CONFIG[status]
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1.5 rounded-sm px-2 py-0.5 font-mono text-[10px] uppercase tracking-wider",
        cfg.pillClass || "border border-border text-muted-foreground",
        cfg.pulse && "animate-pulse",
        className
      )}
    >
      <span className="leading-none">{cfg.glyph}</span>
      <span>{cfg.label}</span>
    </span>
  )
}
