"use client"

import { useEffect, useState } from "react"
import { useTheme } from "next-themes"
import { cn } from "@/lib/utils"

/**
 * Two-glyph toggle: [ ☼ / ◐ ] — uses ASCII-friendly characters that read
 * cleanly in monospace. Hydration-safe: shows skeleton on first paint.
 */
export function ThemeToggle({ className }: { className?: string }) {
  const { resolvedTheme, setTheme } = useTheme()
  const [mounted, setMounted] = useState(false)
  useEffect(() => setMounted(true), [])

  const isDark = mounted && resolvedTheme === "dark"

  return (
    <button
      type="button"
      aria-label={`Switch to ${isDark ? "light" : "dark"} mode`}
      onClick={() => setTheme(isDark ? "light" : "dark")}
      className={cn(
        "inline-flex items-center gap-2 rounded-sm border border-border px-2.5 py-1 text-xs font-mono",
        "text-muted-foreground hover:text-foreground hover:border-foreground transition-colors",
        className
      )}
    >
      {/* glyph */}
      <span className="text-sm leading-none">{mounted ? (isDark ? "◐" : "☀") : "·"}</span>
      <span className="hidden sm:inline">{mounted ? (isDark ? "DARK" : "LIGHT") : "····"}</span>
    </button>
  )
}
