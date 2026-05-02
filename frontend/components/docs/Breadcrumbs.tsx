import Link from "next/link"
import { breadcrumbsFor } from "@/lib/docs-nav"

export function DocsBreadcrumbs({ href }: { href: string }) {
  const crumbs = breadcrumbsFor(href)
  return (
    <nav
      aria-label="Breadcrumb"
      className="mb-4 flex items-center gap-1.5 text-xs font-mono text-muted-foreground"
    >
      {crumbs.map((c, i) => {
        const last = i === crumbs.length - 1
        return (
          <span key={`${c.label}-${i}`} className="flex items-center gap-1.5">
            {c.href && !last ? (
              <Link href={c.href} className="hover:text-foreground transition-colors">
                {c.label}
              </Link>
            ) : (
              <span className={last ? "text-foreground" : ""}>{c.label}</span>
            )}
            {!last && <span className="text-muted-foreground/50">/</span>}
          </span>
        )
      })}
    </nav>
  )
}
