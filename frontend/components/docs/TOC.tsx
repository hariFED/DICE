"use client"

import { useEffect, useState } from "react"
import { cn } from "@/lib/utils"

export type TocItem = { id: string; label: string; level: 2 | 3 }

/**
 * Sticky right-rail "on this page" TOC. Scroll-spy activates the nearest
 * heading above the viewport top. Hidden on mobile; caller handles layout.
 */
export function DocsTOC({ items }: { items: TocItem[] }) {
  const [activeId, setActiveId] = useState<string>(items[0]?.id ?? "")

  useEffect(() => {
    if (items.length === 0) return
    const handler = () => {
      const y = window.scrollY + 96
      let current = items[0]?.id ?? ""
      for (const item of items) {
        const el = document.getElementById(item.id)
        if (el && el.offsetTop <= y) current = item.id
      }
      setActiveId(current)
    }
    handler()
    window.addEventListener("scroll", handler, { passive: true })
    return () => window.removeEventListener("scroll", handler)
  }, [items])

  if (items.length === 0) return null

  return (
    <aside
      aria-label="On this page"
      className="hidden xl:block w-60 shrink-0"
    >
      <div className="sticky top-24 flex flex-col gap-2 text-[13px]">
        <p className="text-[11px] font-semibold uppercase tracking-[0.14em] text-zinc-500">
          On this page
        </p>
        <ul className="flex flex-col gap-1">
          {items.map((item) => (
            <li key={item.id}>
              <a
                href={`#${item.id}`}
                className={cn(
                  "block border-l py-1 transition-colors",
                  item.level === 3 ? "pl-6" : "pl-3",
                  activeId === item.id
                    ? "border-[#00FF85] text-white"
                    : "border-white/10 text-zinc-500 hover:text-zinc-200"
                )}
              >
                {item.label}
              </a>
            </li>
          ))}
        </ul>
      </div>
    </aside>
  )
}
