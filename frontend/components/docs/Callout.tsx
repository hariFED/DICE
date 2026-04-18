import { cn } from "@/lib/utils"
import type { ReactNode } from "react"

type Variant = "info" | "tip" | "warn" | "danger"

const STYLES: Record<
  Variant,
  { border: string; bg: string; label: string; icon: string; title: string }
> = {
  info: {
    border: "border-l-sky-400",
    bg: "bg-sky-400/[0.06]",
    label: "text-sky-300",
    icon: "i",
    title: "Note",
  },
  tip: {
    border: "border-l-[#00FF85]",
    bg: "bg-[#00FF85]/[0.06]",
    label: "text-[#00FF85]",
    icon: "✓",
    title: "Tip",
  },
  warn: {
    border: "border-l-amber-400",
    bg: "bg-amber-400/[0.06]",
    label: "text-amber-300",
    icon: "!",
    title: "Warning",
  },
  danger: {
    border: "border-l-red-500",
    bg: "bg-red-500/[0.06]",
    label: "text-red-400",
    icon: "×",
    title: "Danger",
  },
}

export function Callout({
  type = "info",
  title,
  children,
}: {
  type?: Variant
  title?: string
  children: ReactNode
}) {
  const s = STYLES[type]
  return (
    <div
      className={cn(
        "my-5 rounded-r-lg border-l-2 py-3 pl-4 pr-4 text-[14.5px] text-zinc-300 leading-[1.65]",
        s.border,
        s.bg,
      )}
    >
      <p
        className={cn(
          "mb-1 flex items-center gap-2 text-[11px] font-semibold uppercase tracking-[0.14em]",
          s.label,
        )}
      >
        <span
          className={cn(
            "inline-flex h-4 w-4 items-center justify-center rounded-full text-[10px] font-bold",
            "bg-white/10 text-white",
          )}
        >
          {s.icon}
        </span>
        {title ?? s.title}
      </p>
      <div className="[&_p]:my-1 [&_code]:text-[13.5px]">{children}</div>
    </div>
  )
}
