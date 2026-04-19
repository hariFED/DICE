"use client"

import { useEffect, useRef, useState } from "react"
import { useRouter } from "next/navigation"
import { flatDocs } from "@/lib/docs-nav"

/**
 * Keyboard-driven quick-nav. Pressing `/` focuses the input; typing filters
 * the docs tree; Enter jumps to the first hit. No backend — searches page
 * titles + section labels from the nav manifest.
 */
export function DocsSearchStub() {
  const [q, setQ] = useState("")
  const [open, setOpen] = useState(false)
  const inputRef = useRef<HTMLInputElement>(null)
  const router = useRouter()

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (
        e.key === "/" &&
        document.activeElement?.tagName !== "INPUT" &&
        document.activeElement?.tagName !== "TEXTAREA"
      ) {
        e.preventDefault()
        inputRef.current?.focus()
        setOpen(true)
      }
      if (e.key === "Escape") {
        setOpen(false)
        inputRef.current?.blur()
      }
    }
    window.addEventListener("keydown", onKey)
    return () => window.removeEventListener("keydown", onKey)
  }, [])

  const all = flatDocs()
  const ql = q.trim().toLowerCase()
  const results = ql
    ? all.filter(
        (d) =>
          d.title.toLowerCase().includes(ql) ||
          d.sectionTitle.toLowerCase().includes(ql),
      )
    : []

  return (
    <div className="relative">
      <div className="relative flex items-center gap-2 rounded-sm border border-border bg-background px-2.5 py-1.5">
        <span className="text-muted-foreground font-mono text-xs">›</span>
        <input
          ref={inputRef}
          type="text"
          value={q}
          onChange={(e) => {
            setQ(e.target.value)
            setOpen(true)
          }}
          onFocus={() => setOpen(true)}
          onBlur={() => setTimeout(() => setOpen(false), 150)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && results[0]) {
              router.push(results[0].href)
              setOpen(false)
              setQ("")
            }
          }}
          placeholder="search docs"
          className="flex-1 bg-transparent font-mono text-[13px] text-foreground placeholder:text-muted-foreground/60 focus:outline-none"
          aria-label="Search documentation"
        />
        <kbd className="rounded-sm border border-border bg-muted px-1.5 py-0.5 font-mono text-[10px] text-muted-foreground">
          /
        </kbd>
      </div>

      {open && results.length > 0 && (
        <div className="absolute top-full left-0 right-0 z-50 mt-1 max-h-80 overflow-y-auto border border-border bg-background py-1 shadow-lg">
          {results.slice(0, 10).map((d) => (
            <button
              key={d.href}
              type="button"
              onMouseDown={() => {
                router.push(d.href)
                setOpen(false)
                setQ("")
              }}
              className="block w-full px-3 py-2 text-left text-[13px] font-mono hover:bg-muted/40"
            >
              <span className="ascii-label text-[10px]">{d.sectionTitle}</span>
              <span className="block text-foreground">{d.title}</span>
            </button>
          ))}
        </div>
      )}
    </div>
  )
}
