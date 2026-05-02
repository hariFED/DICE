/**
 * Mono section divider:  ── × ──  ◇  ── × ──
 * Pure CSS via `.ascii-rule` utility. The center label is optional.
 */
export function AsciiDivider({ label, className = "" }: { label?: string; className?: string }) {
  return (
    <div className={`ascii-rule my-16 ${className}`} aria-hidden>
      {label ? <span className="px-2 tracking-[0.2em]">{label}</span> : <span className="px-2">◇</span>}
    </div>
  )
}

export default AsciiDivider
