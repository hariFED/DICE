import type { ReactNode } from "react"
import { cn } from "@/lib/utils"

interface GlowCardProps {
  className?: string
  children: ReactNode
  glowOnHover?: boolean
}

export function GlowCard({
  className,
  children,
  glowOnHover = true,
}: GlowCardProps) {
  return (
    <div
      className={cn(
        "bg-white/[0.03] backdrop-blur-xl border border-white/[0.08] rounded-xl",
        glowOnHover &&
          "hover:shadow-[0_0_30px_rgba(0,255,133,0.1)] hover:border-white/[0.15] transition-all duration-300",
        className
      )}
    >
      {children}
    </div>
  )
}
