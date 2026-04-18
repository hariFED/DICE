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
      <div className="relative flex items-center gap-2 rounded-lg border border-white/[0.08] bg-white/[0.02] px-2.5 py-1.5">
        <svg
          width="13"
          height="13"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          className="text-zinc-500"
        >
          <circle cx="11" cy="11" r="7" />
          <line x1="21" y1="21" x2="16.65" y2="16.65" />
        </svg>
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
          placeholder="Search docs"
          className="flex-1 bg-transparent text-[13px] text-zinc-200 placeholder:text-zinc-600 focus:outline-none"
          aria-label="Search documentation"
        />
        <kbd className="rounded border border-white/[0.1] bg-white/[0.04] px-1.5 py-0.5 font-mono text-[10px] text-zinc-500">
          /
        </kbd>
      </div>

      {open && results.length > 0 && (
        <div className="absolute top-full left-0 right-0 z-50 mt-1 max-h-80 overflow-y-auto rounded-lg border border-white/[0.08] bg-[#060606] py-1 shadow-2xl">
          {results.slice(0, 10).map((d) => (
            <button
              key={d.href}
              type="button"
              onMouseDown={() => {
                router.push(d.href)
                setOpen(false)
                setQ("")
              }}
              className="block w-full px-3 py-2 text-left text-[13px] hover:bg-white/[0.04]"
            >
              <span className="text-[10px] uppercase tracking-[0.14em] text-zinc-600">
                {d.sectionTitle}
              </span>
              <span className="block text-zinc-200">{d.title}</span>
            </button>
          ))}
        </div>
      )}
    </div>
  )
}
