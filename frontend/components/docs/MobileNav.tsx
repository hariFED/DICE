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
        className="lg:hidden inline-flex items-center gap-2 rounded-lg border border-white/[0.1] bg-white/[0.02] px-3 py-1.5 text-[13px] text-zinc-300 hover:bg-white/[0.06]"
      >
        <svg
          width="14"
          height="14"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
        >
          <line x1="3" y1="6" x2="21" y2="6" />
          <line x1="3" y1="12" x2="21" y2="12" />
          <line x1="3" y1="18" x2="15" y2="18" />
        </svg>
        Menu
      </button>

      {open && (
        <div className="fixed inset-0 z-[60] lg:hidden">
          <div
            className="absolute inset-0 bg-black/60 backdrop-blur-sm"
            onClick={() => setOpen(false)}
          />
          <div
            className={cn(
              "absolute left-0 top-0 h-full w-72 overflow-y-auto border-r border-white/[0.08] bg-[#060606] p-6",
              "animate-in slide-in-from-left",
            )}
          >
            <div className="mb-4 flex items-center justify-between">
              <span className="text-[11px] font-semibold uppercase tracking-[0.14em] text-zinc-400">
                Navigation
              </span>
              <button
                type="button"
                onClick={() => setOpen(false)}
                aria-label="Close"
                className="rounded-md p-1.5 text-zinc-400 hover:bg-white/[0.06] hover:text-white"
              >
                <svg
                  width="16"
                  height="16"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="2"
                  strokeLinecap="round"
                >
                  <line x1="18" y1="6" x2="6" y2="18" />
                  <line x1="6" y1="6" x2="18" y2="18" />
                </svg>
              </button>
            </div>
            <DocsSidebar />
          </div>
        </div>
      )}
    </>
  )
}
