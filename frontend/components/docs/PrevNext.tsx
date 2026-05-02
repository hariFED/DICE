import Link from "next/link"
import { prevNext } from "@/lib/docs-nav"

export function DocsPrevNext({ href }: { href: string }) {
  const { prev, next } = prevNext(href)
  return (
    <div className="mt-16 grid grid-cols-1 gap-3 sm:grid-cols-2">
      {prev ? (
        <Link
          href={prev.href}
          className="group ascii-box flex flex-col px-4 py-3 transition-colors hover:bg-muted/30"
        >
          <span className="flex items-center gap-1.5 ascii-label text-[10px]">
            ‹ previous
          </span>
          <span className="mt-1 font-mono text-sm text-foreground">
            {prev.title}
          </span>
        </Link>
      ) : (
        <span />
      )}

      {next ? (
        <Link
          href={next.href}
          className="group ascii-box flex flex-col items-end px-4 py-3 text-right transition-colors hover:bg-muted/30 sm:col-start-2"
        >
          <span className="flex items-center gap-1.5 ascii-label text-[10px]">
            next ›
          </span>
          <span className="mt-1 font-mono text-sm text-foreground">
            {next.title}
          </span>
        </Link>
      ) : null}
    </div>
  )
}
