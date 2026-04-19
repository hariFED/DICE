import { cn } from "@/lib/utils"
import type { ReactNode } from "react"

type Variant = "info" | "tip" | "warn" | "danger"

const STYLES: Record<Variant, { glyph: string; title: string; pillClass: string }> = {
  info:   { glyph: "ℹ", title: "Note",    pillClass: "" },
  tip:    { glyph: "✓", title: "Tip",     pillClass: "pill-ok" },
  warn:   { glyph: "!", title: "Warning", pillClass: "pill-warn" },
  danger: { glyph: "✕", title: "Danger",  pillClass: "pill-err" },
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
    <div className="my-5 ascii-box border-l-2 border-l-foreground py-3 pl-4 pr-4 text-[14.5px] text-foreground leading-[1.65] font-mono">
      <p className="mb-2 flex items-center gap-2 text-[11px] uppercase tracking-[0.14em]">
        <span
          className={cn(
            "inline-flex items-center justify-center px-1.5 py-0 text-[10px] font-mono leading-none rounded-sm",
            s.pillClass || "border border-border text-muted-foreground",
          )}
        >
          {s.glyph} {title ?? s.title}
        </span>
      </p>
      <div className="[&_p]:my-1 [&_code]:text-[13.5px]">{children}</div>
    </div>
  )
}
