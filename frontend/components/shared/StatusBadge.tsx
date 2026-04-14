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

const STATUS_CONFIG: Record<Status, { label: string; classes: string }> = {
  finalized: {
    label: "Finalized",
    classes: "bg-green-500/20 text-green-400 border border-green-500/30",
  },
  failed: {
    label: "Failed",
    classes: "bg-red-500/20 text-red-400 border border-red-500/30",
  },
  collecting_commits: {
    label: "Collecting Commits",
    classes:
      "bg-blue-500/20 text-blue-400 border border-blue-500/30 animate-pulse",
  },
  collecting_reveals: {
    label: "Collecting Reveals",
    classes:
      "bg-blue-500/20 text-blue-400 border border-blue-500/30 animate-pulse",
  },
  idle: {
    label: "Idle",
    classes: "bg-zinc-500/20 text-zinc-400 border border-zinc-500/30",
  },
}

export function StatusBadge({ status, className }: StatusBadgeProps) {
  const config = STATUS_CONFIG[status]

  return (
    <span
      className={cn(
        "inline-flex items-center rounded-full px-3 py-1 text-xs font-medium",
        config.classes,
        className
      )}
    >
      {config.label}
    </span>
  )
}
