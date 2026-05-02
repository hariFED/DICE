import type { ReactNode } from "react"
import { cn } from "@/lib/utils"

interface CornerBoxProps {
  /** Optional title rendered into the top edge:  ┌─[ TITLE ]─────────┐ */
  title?: string
  /** Optional bottom-right tag:  └────[ TAG ]──┘ */
  tag?: string
  padded?: boolean
  className?: string
  innerClassName?: string
  children: ReactNode
}

/**
 * TUI-style frame.
 *
 * Rendering trick: a single dashed `border` gives us all four sides for free.
 * Four absolutely-positioned ┌┐└┘ chars sit on top of the corners with the
 * background color "punched through" so the dashed pattern terminates cleanly
 * into the corner glyph. Title / tag chips dock into the top / bottom edges
 * the same way.
 *
 * Pure CSS, no per-side rails to repeat — safe at any height/width.
 */
export function CornerBox({
  title,
  tag,
  padded = true,
  className,
  innerClassName,
  children,
}: CornerBoxProps) {
  return (
    <div
      className={cn(
        "group/corner relative border border-dashed border-muted-foreground/50 transition-colors hover:border-foreground overflow-hidden",
        className,
      )}
    >
      {/* Corner glyphs — sit on top of the border, background-color punches through */}
      <span aria-hidden className="absolute -top-[0.55em] -left-[0.4ch] font-mono text-xs leading-none text-muted-foreground bg-background px-0.5 group-hover/corner:text-foreground transition-colors">┌</span>
      <span aria-hidden className="absolute -top-[0.55em] -right-[0.4ch] font-mono text-xs leading-none text-muted-foreground bg-background px-0.5 group-hover/corner:text-foreground transition-colors">┐</span>
      <span aria-hidden className="absolute -bottom-[0.55em] -left-[0.4ch] font-mono text-xs leading-none text-muted-foreground bg-background px-0.5 group-hover/corner:text-foreground transition-colors">└</span>
      <span aria-hidden className="absolute -bottom-[0.55em] -right-[0.4ch] font-mono text-xs leading-none text-muted-foreground bg-background px-0.5 group-hover/corner:text-foreground transition-colors">┘</span>

      {/* Top edge title */}
      {title && (
        <span className="absolute -top-[0.65em] left-3 font-mono text-[10px] leading-none uppercase tracking-wider bg-background px-1.5 text-foreground">
          [ {title} ]
        </span>
      )}

      {/* Bottom-right tag */}
      {tag && (
        <span className="absolute -bottom-[0.65em] right-3 font-mono text-[10px] leading-none uppercase tracking-wider bg-background px-1.5 text-muted-foreground">
          [ {tag} ]
        </span>
      )}

      <div className={cn(padded && "p-4 sm:p-5", innerClassName)}>
        {children}
      </div>
    </div>
  )
}

export default CornerBox
