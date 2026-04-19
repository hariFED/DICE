"use client"

import { useState, useEffect } from "react"
import { DocsSidebar } from "./Sidebar"
import { cn } from "@/lib/utils"
import { usePathname } from "next/navigation"

/**
 * Mobile-only hamburger that slides the sidebar in from the left.
 * Auto-closes on route change by resetting state when pathname changes —
 * derived-during-render rather than synced via useEffect to avoid a
 * cascading render.
 */
export function DocsMobileNav() {
  const pathname = usePathname()
  const [open, setOpen] = useState(false)
  const [lastPath, setLastPath] = useState(pathname)
  if (pathname !== lastPath) {
    // Route changed — auto-close, done during render (React pattern).
    setLastPath(pathname)
    setOpen(false)
  }

  // Lock body scroll while drawer is open.
  useEffect(() => {
    if (open) {
      const prev = document.body.style.overflow
      document.body.style.overflow = "hidden"
      return () => {
        document.body.style.overflow = prev
      }
    }
  }, [open])

  return (
    <>
      <button
        type="button"
        onClick={() => setOpen(true)}
        aria-label="Open docs navigation"
        className="lg:hidden inline-flex items-center gap-2 rounded-sm border border-border bg-background px-3 py-1.5 text-xs font-mono uppercase tracking-wider text-muted-foreground hover:text-foreground hover:border-foreground transition-colors"
      >
        <span>≡</span>
        Menu
      </button>

      {open && (
        <div className="fixed inset-0 z-[60] lg:hidden">
          <div
            className="absolute inset-0 bg-foreground/40 backdrop-blur-sm"
            onClick={() => setOpen(false)}
          />
          <div
            className={cn(
              "absolute left-0 top-0 h-full w-72 overflow-y-auto border-r border-border bg-background p-6",
              "animate-in slide-in-from-left",
            )}
          >
            <div className="mb-4 flex items-center justify-between">
              <span className="ascii-label">navigation</span>
              <button
                type="button"
                onClick={() => setOpen(false)}
                aria-label="Close"
                className="rounded-sm p-1 font-mono text-muted-foreground hover:text-foreground"
              >
                ✕
              </button>
            </div>
            <DocsSidebar />
          </div>
        </div>
      )}
    </>
  )
}
