import Link from "next/link"
import type { ReactNode, ButtonHTMLAttributes, AnchorHTMLAttributes } from "react"
import { cn } from "@/lib/utils"

type Variant = "primary" | "ghost"

interface CommonProps {
  children: ReactNode
  variant?: Variant
  glyph?: string
  className?: string
}

const VARIANTS: Record<Variant, string> = {
  primary: "border-foreground bg-foreground text-background hover:bg-background hover:text-foreground",
  ghost:   "border-border text-muted-foreground hover:text-foreground hover:border-foreground",
}

const COMMON =
  "inline-flex items-center gap-2 rounded-none border px-3 py-1.5 font-mono text-xs uppercase tracking-wider transition-colors disabled:opacity-60 disabled:cursor-not-allowed select-none"

interface ButtonProps
  extends Omit<ButtonHTMLAttributes<HTMLButtonElement>, "children" | "className">,
    CommonProps {}

export function BracketButton({
  children,
  variant = "primary",
  glyph = "›",
  className,
  ...rest
}: ButtonProps) {
  return (
    <button {...rest} className={cn(COMMON, VARIANTS[variant], className)}>
      <span aria-hidden>[</span>
      <span aria-hidden>{glyph}</span>
      <span>{children}</span>
      <span aria-hidden>]</span>
    </button>
  )
}

interface LinkProps extends CommonProps {
  href: string
  external?: boolean
}

export function BracketLink({
  children,
  variant = "primary",
  glyph = "›",
  className,
  href,
  external,
}: LinkProps) {
  const cls = cn(COMMON, VARIANTS[variant], className)
  const inner = (
    <>
      <span aria-hidden>[</span>
      <span aria-hidden>{glyph}</span>
      <span>{children}</span>
      <span aria-hidden>]</span>
    </>
  )

  if (external) {
    return (
      <a href={href} target="_blank" rel="noopener noreferrer" className={cls}>
        {inner}
      </a>
    )
  }
  return (
    <Link href={href} className={cls}>
      {inner}
    </Link>
  )
}

// Suppress unused-import warning in light mode.
export type _AnchorAttrs = AnchorHTMLAttributes<HTMLAnchorElement>
