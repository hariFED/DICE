"use client"

import Link from "next/link"
import { usePathname } from "next/navigation"
import { useState } from "react"
import { DOCS_NAV, type DocsSection } from "@/lib/docs-nav"
import { cn } from "@/lib/utils"

/**
 * The sidebar only renders sections — any top-level leaf we might add in
 * the future would be surfaced elsewhere. Keeping the surface explicit
 * makes the TS narrowing painless.
 */
const SECTIONS: DocsSection[] = DOCS_NAV.filter(
  (n): n is DocsSection => "pages" in n,
)

/**
 * Sticky left sidebar with collapsible sections. Highlights the current
 * page with a left-border accent and keeps the containing section expanded.
 */
export function DocsSidebar({ className }: { className?: string }) {
  const pathname = usePathname()

  // Track which sections are open. Default: all open when the current page
  // lives inside them; closed otherwise. Users can toggle.
  const initial = (): Record<string, boolean> => {
    const map: Record<string, boolean> = {}
    for (const node of SECTIONS) {
      const containsCurrent = node.pages.some((p) => p.href === pathname)
      map[node.title] = containsCurrent || node.title === "Overview"
    }
    return map
  }
  const [open, setOpen] = useState<Record<string, boolean>>(initial)

  // Navigation expansion on route change: derive during render rather than
  // reaching for useEffect — React 19 prefers reset-via-state-comparison.
  const [lastPath, setLastPath] = useState(pathname)
  if (pathname !== lastPath) {
    setLastPath(pathname)
    setOpen((prev) => {
      const next = { ...prev }
      for (const node of SECTIONS) {
        if (node.pages.some((p) => p.href === pathname)) {
          next[node.title] = true
        }
      }
      return next
    })
  }

  return (
    <nav
      aria-label="Docs navigation"
      className={cn(
        "flex flex-col gap-6 text-sm",
        className
      )}
    >
      {SECTIONS.map((node) => {
        const isOpen = open[node.title] ?? false
        return (
          <div key={node.title} className="flex flex-col">
            <button
              type="button"
              onClick={() =>
                setOpen((prev) => ({ ...prev, [node.title]: !prev[node.title] }))
              }
              className="flex items-center justify-between px-2 py-1 text-[11px] font-semibold uppercase tracking-[0.14em] text-zinc-400 hover:text-white transition-colors"
            >
              <span>{node.title}</span>
              <svg
                width="10"
                height="10"
                viewBox="0 0 10 10"
                className={cn(
                  "transition-transform duration-200",
                  isOpen ? "rotate-0" : "-rotate-90"
                )}
              >
                <path
                  d="M2 4l3 3 3-3"
                  stroke="currentColor"
                  strokeWidth="1.5"
                  fill="none"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                />
              </svg>
            </button>

            {isOpen && (
              <ul className="mt-1 flex flex-col">
                {node.pages.map((page) => {
                  const isActive = pathname === page.href
                  return (
                    <li key={page.href}>
                      <Link
                        href={page.href}
                        className={cn(
                          "block border-l-2 py-1.5 pl-4 pr-2 text-[13.5px] transition-colors",
                          isActive
                            ? "border-[#00FF85] text-white font-medium bg-white/[0.03]"
                            : "border-transparent text-zinc-500 hover:text-zinc-200 hover:border-white/20"
                        )}
                      >
                        {page.title}
                      </Link>
                    </li>
                  )
                })}
              </ul>
            )}
          </div>
        )
      })}
    </nav>
  )
}
