import type { ReactNode } from "react"
import { DocsBreadcrumbs } from "./Breadcrumbs"
import { DocsPrevNext } from "./PrevNext"
import { DocsTOC, type TocItem } from "./TOC"
import { H1, Lead } from "./Prose"

/**
 * Consistent wrapper each docs page uses. Passes `href` down to
 * breadcrumbs and prev/next so there's exactly one source of truth
 * (the nav manifest in `lib/docs-nav.ts`).
 */
export function DocsPageShell({
  href,
  title,
  lead,
  toc = [],
  children,
}: {
  href: string
  title: string
  lead?: string
  toc?: TocItem[]
  children: ReactNode
}) {
  return (
    <div className="flex gap-12">
      <article className="min-w-0 flex-1 max-w-[720px] pb-10">
        <DocsBreadcrumbs href={href} />
        <H1>{title}</H1>
        {lead && <Lead>{lead}</Lead>}
        <div className="mt-6">{children}</div>
        <DocsPrevNext href={href} />
      </article>
      <DocsTOC items={toc} />
    </div>
  )
}
