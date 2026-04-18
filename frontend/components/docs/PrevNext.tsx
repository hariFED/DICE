import Link from "next/link"
import { prevNext } from "@/lib/docs-nav"

/**
 * Two-card prev/next navigation for the bottom of each docs page.
 * Feeds from `flatDocs()` so order matches the sidebar.
 */
export function DocsPrevNext({ href }: { href: string }) {
  const { prev, next } = prevNext(href)
  return (
    <div className="mt-16 grid grid-cols-1 gap-3 sm:grid-cols-2">
      {prev ? (
        <Link
          href={prev.href}
          className="group flex flex-col rounded-xl border border-white/[0.08] bg-white/[0.02] px-4 py-3 transition-colors hover:border-[#00FF85]/40 hover:bg-white/[0.04]"
        >
          <span className="flex items-center gap-1 text-[11px] uppercase tracking-[0.14em] text-zinc-500">
            <svg
              width="10"
              height="10"
              viewBox="0 0 10 10"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.5"
            >
              <path d="M6 2L3 5l3 3" strokeLinecap="round" strokeLinejoin="round" />
            </svg>
            Previous
          </span>
          <span className="mt-1 text-sm font-medium text-zinc-200 group-hover:text-white">
            {prev.title}
          </span>
        </Link>
      ) : (
        <span />
      )}

      {next ? (
        <Link
          href={next.href}
          className="group flex flex-col items-end rounded-xl border border-white/[0.08] bg-white/[0.02] px-4 py-3 text-right transition-colors hover:border-[#00FF85]/40 hover:bg-white/[0.04] sm:col-start-2"
        >
          <span className="flex items-center gap-1 text-[11px] uppercase tracking-[0.14em] text-zinc-500">
            Next
            <svg
              width="10"
              height="10"
              viewBox="0 0 10 10"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.5"
            >
              <path d="M4 2l3 3-3 3" strokeLinecap="round" strokeLinejoin="round" />
            </svg>
          </span>
          <span className="mt-1 text-sm font-medium text-zinc-200 group-hover:text-white">
            {next.title}
          </span>
        </Link>
      ) : null}
    </div>
  )
}
