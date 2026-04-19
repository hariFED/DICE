"use client"

import Link from "next/link"
import { usePathname } from "next/navigation"
import { useState } from "react"
import { DOCS_NAV, type DocsSection } from "@/lib/docs-nav"
import { cn } from "@/lib/utils"

const SECTIONS: DocsSection[] = DOCS_NAV.filter(
  (n): n is DocsSection => "pages" in n,
)

/** Sticky left sidebar — mono palette, ASCII chrome. */
export function DocsSidebar({ className }: { className?: string }) {
  const pathname = usePathname()

  const initial = (): Record<string, boolean> => {
    const map: Record<string, boolean> = {}
    for (const node of SECTIONS) {
      const containsCurrent = node.pages.some((p) => p.href === pathname)
      map[node.title] = containsCurrent || node.title === "Overview"
    }
    return map
  }
  const [open, setOpen] = useState<Record<string, boolean>>(initial)

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
      className={cn("flex flex-col gap-5 text-sm font-mono", className)}
    >
      {SECTIONS.map((node) => {
        const isOpen = open[node.title] ?? false
        return (
          <div key={node.title} className="flex flex-col">
            <button
              type="button"
              onClick={() => setOpen((prev) => ({ ...prev, [node.title]: !prev[node.title] }))}
              className="flex items-center justify-between px-1 py-1 ascii-label hover:text-foreground transition-colors"
            >
              <span>{node.title}</span>
              <span className="text-xs">{isOpen ? "−" : "+"}</span>
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
                          "block border-l py-1.5 pl-3 pr-2 text-[13px] transition-colors",
                          isActive
                            ? "border-foreground text-foreground bg-muted/40"
                            : "border-border text-muted-foreground hover:text-foreground hover:border-foreground"
                        )}
                      >
                        <span className={cn("mr-1 transition-opacity", isActive ? "opacity-100" : "opacity-0")}>›</span>
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
