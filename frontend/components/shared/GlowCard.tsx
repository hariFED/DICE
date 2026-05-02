import type { ReactNode } from "react"
import { cn } from "@/lib/utils"

/**
 * Repurposed: kept the export name so existing pages (docs / explorer / preorder)
 * compile, but visually it's now an `ascii-box` — 1px border, theme-aware bg,
 * no glow shadow. The ASCII redesign dropped all gradient/glow chrome.
 */
interface GlowCardProps {
  className?: string
  children: ReactNode
  /** kept for back-compat; ignored. */
  glowOnHover?: boolean
}

export function GlowCard({ className, children }: GlowCardProps) {
  return <div className={cn("ascii-box", className)}>{children}</div>
}
