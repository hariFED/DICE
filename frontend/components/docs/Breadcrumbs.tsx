import Link from "next/link"
import { breadcrumbsFor } from "@/lib/docs-nav"

export function DocsBreadcrumbs({ href }: { href: string }) {
  const crumbs = breadcrumbsFor(href)
  return (
    <nav
      aria-label="Breadcrumb"
      className="mb-4 flex items-center gap-1.5 text-[12.5px] text-zinc-500"
    >
      {crumbs.map((c, i) => {
        const last = i === crumbs.length - 1
        return (
          <span key={`${c.label}-${i}`} className="flex items-center gap-1.5">
            {c.href && !last ? (
              <Link
                href={c.href}
                className="hover:text-zinc-200 transition-colors"
              >
                {c.label}
              </Link>
            ) : (
              <span className={last ? "text-zinc-300" : ""}>{c.label}</span>
            )}
            {!last && <span className="text-zinc-700">/</span>}
          </span>
        )
      })}
    </nav>
  )
}
