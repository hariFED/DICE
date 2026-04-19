/**
 * Docs navigation tree.
 *
 * Single source of truth for:
 *   - the sidebar
 *   - prev / next footer buttons
 *   - breadcrumb trail lookup
 *
 * Pages are authored in `app/docs/**` as plain TSX. This tree mirrors that
 * file layout. Flatten() walks it in reading order so prev/next is just an
 * index lookup.
 */
export type DocsLeaf = {
  title: string
  href: string
  /** Short label shown under breadcrumbs / in meta tags. */
  summary?: string
}

export type DocsSection = {
  title: string
  /** Landing page for the section (optional). */
  href?: string
  pages: DocsLeaf[]
}

export type DocsNode = DocsLeaf | DocsSection

export const DOCS_NAV: DocsNode[] = [
  {
    title: "For Beginners",
    pages: [
      { title: "Start here · what is VRF?", href: "/docs/getting-started" },
      { title: "Your first request", href: "/docs/getting-started/first-request" },
      { title: "Glossary", href: "/docs/getting-started/glossary" },
    ],
  },
  {
    title: "Overview",
    pages: [
      { title: "Welcome", href: "/docs" },
      { title: "Introduction", href: "/docs/introduction" },
      { title: "Quickstart", href: "/docs/quickstart" },
      { title: "Architecture", href: "/docs/architecture" },
    ],
  },
  {
    title: "Integration",
    href: "/docs/integration",
    pages: [
      { title: "Overview", href: "/docs/integration" },
      { title: "Channel setup", href: "/docs/integration/setup" },
      { title: "Request randomness", href: "/docs/integration/request" },
      { title: "Callback handler", href: "/docs/integration/callback" },
      { title: "Outcome formulas", href: "/docs/integration/formulas" },
    ],
  },
  {
    title: "Reference",
    href: "/docs/reference",
    pages: [
      { title: "Overview", href: "/docs/reference" },
      { title: "Callback payload", href: "/docs/reference/payload" },
      { title: "Selection modes", href: "/docs/reference/modes" },
      { title: "Pricing", href: "/docs/reference/pricing" },
      { title: "Errors", href: "/docs/reference/errors" },
    ],
  },
  {
    title: "Advanced",
    href: "/docs/advanced",
    pages: [
      { title: "Overview", href: "/docs/advanced" },
      { title: "Streaming feed", href: "/docs/advanced/streaming" },
      { title: "Verifying randomness", href: "/docs/advanced/verify" },
      { title: "On-chain selection", href: "/docs/advanced/selection" },
    ],
  },
  {
    title: "Help",
    pages: [{ title: "FAQ", href: "/docs/faq" }],
  },
]

/* ---------- helpers ----------------------------------------------------- */

export type FlatDoc = {
  title: string
  href: string
  sectionTitle: string
}

/** Flatten the tree into reading order (depth-first). */
export function flatDocs(): FlatDoc[] {
  const out: FlatDoc[] = []
  for (const node of DOCS_NAV) {
    if ("pages" in node) {
      for (const p of node.pages) {
        out.push({ title: p.title, href: p.href, sectionTitle: node.title })
      }
    } else {
      out.push({ title: node.title, href: node.href, sectionTitle: node.title })
    }
  }
  // De-duplicate by href, keeping first occurrence.
  const seen = new Set<string>()
  return out.filter((d) => {
    if (seen.has(d.href)) return false
    seen.add(d.href)
    return true
  })
}

export function findDoc(href: string): FlatDoc | undefined {
  return flatDocs().find((d) => d.href === href)
}

export function prevNext(href: string): {
  prev: FlatDoc | null
  next: FlatDoc | null
} {
  const all = flatDocs()
  const i = all.findIndex((d) => d.href === href)
  if (i === -1) return { prev: null, next: null }
  return {
    prev: i > 0 ? all[i - 1] : null,
    next: i < all.length - 1 ? all[i + 1] : null,
  }
}

/** Build breadcrumbs for a doc href. */
export function breadcrumbsFor(href: string): { label: string; href?: string }[] {
  const crumbs: { label: string; href?: string }[] = [
    { label: "Docs", href: "/docs" },
  ]
  if (href === "/docs") return crumbs

  // Section + page lookup
  for (const node of DOCS_NAV) {
    if (!("pages" in node)) continue
    const match = node.pages.find((p) => p.href === href)
    if (match) {
      if (node.href && node.href !== href) {
        crumbs.push({ label: node.title, href: node.href })
      } else if (!node.href) {
        // non-landing section — just show label, no link
        crumbs.push({ label: node.title })
      }
      if (match.href !== node.href) {
        crumbs.push({ label: match.title })
      }
      return crumbs
    }
  }
  return crumbs
}
