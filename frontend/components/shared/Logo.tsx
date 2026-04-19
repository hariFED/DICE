import { SITE } from "@/lib/constants"

/**
 * DICE logo — isometric cube glyph + wordmark.
 *
 * The SVG uses currentColor for the cube fill + CSS var for seams so the
 * cube reads correctly in both themes without needing two files.
 */
export function Logo({
  size = 28,
  showWordmark = true,
  className = "",
}: {
  size?: number
  showWordmark?: boolean
  className?: string
}) {
  return (
    <span className={`inline-flex items-center gap-2.5 ${className}`}>
      <svg
        width={size}
        height={size}
        viewBox="0 0 64 64"
        fill="none"
        xmlns="http://www.w3.org/2000/svg"
        aria-hidden="true"
      >
        <path d="M32 4 L60 19 L60 45 L32 60 L4 45 L4 19 Z" fill="currentColor" />
        <path d="M4 19 L32 34 L60 19" stroke="var(--background)" strokeWidth="2" fill="none" />
        <path d="M32 4 L32 34" stroke="var(--background)" strokeWidth="2" fill="none" />
        <path d="M18 11.5 L46 26.5 M46 11.5 L18 26.5" stroke="var(--background)" strokeWidth="1.25" opacity="0.9" fill="none" />
        <path d="M32 34 L32 60" stroke="var(--background)" strokeWidth="2" fill="none" />
        <path d="M18 26.5 L18 52.5 M46 26.5 L46 52.5" stroke="var(--background)" strokeWidth="1.25" opacity="0.9" fill="none" />
        <path d="M4 32 L32 47 M60 32 L32 47" stroke="var(--background)" strokeWidth="1.25" opacity="0.9" fill="none" />
        <path d="M46 11.5 L53 15 L53 22 L46 18.5 Z" fill="currentColor" stroke="var(--background)" strokeWidth="1.25" />
        <path d="M46 11.5 L46 18.5 M53 15 L46 18.5" stroke="var(--background)" strokeWidth="1.25" fill="none" />
      </svg>
      {showWordmark && (
        <span className="font-pixel text-[15px] tracking-tight leading-none text-foreground">
          {SITE.name}
        </span>
      )}
    </span>
  )
}

export default Logo
