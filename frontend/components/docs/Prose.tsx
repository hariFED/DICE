import type { ReactNode } from "react"
import { cn } from "@/lib/utils"

/** Page title shown below the breadcrumbs. */
export function H1({ children }: { children: ReactNode }) {
  return (
    <h1 className="font-heading text-[34px] font-semibold tracking-tight text-white leading-[1.15]">
      {children}
    </h1>
  )
}

/** Lead paragraph directly under H1. */
export function Lead({ children }: { children: ReactNode }) {
  return (
    <p className="mt-3 text-[16.5px] leading-[1.6] text-zinc-400">{children}</p>
  )
}

export function H2({ id, children }: { id: string; children: ReactNode }) {
  return (
    <h2
      id={id}
      className="group mt-12 scroll-mt-24 border-b border-white/[0.07] pb-2 font-heading text-[22px] font-semibold tracking-tight text-white"
    >
      <a href={`#${id}`} className="relative no-underline">
        <span className="pointer-events-none absolute -left-6 hidden text-zinc-600 group-hover:block">
          #
        </span>
        {children}
      </a>
    </h2>
  )
}

export function H3({ id, children }: { id: string; children: ReactNode }) {
  return (
    <h3
      id={id}
      className="mt-8 scroll-mt-24 font-heading text-[17px] font-semibold tracking-tight text-zinc-100"
    >
      {children}
    </h3>
  )
}

export function P({
  children,
  className,
}: {
  children: ReactNode
  className?: string
}) {
  return (
    <p
      className={cn(
        "mt-4 text-[15.5px] leading-[1.7] text-zinc-300",
        className,
      )}
    >
      {children}
    </p>
  )
}

/** Inline `code` chip. */
export function Code({ children }: { children: ReactNode }) {
  return (
    <code className="rounded-md border border-white/[0.08] bg-white/[0.04] px-1.5 py-0.5 font-mono text-[13px] text-[#7cf9c1]">
      {children}
    </code>
  )
}

export function Ul({ children }: { children: ReactNode }) {
  return (
    <ul className="mt-3 flex flex-col gap-1.5 pl-5 text-[15.5px] leading-[1.65] text-zinc-300 [&_li]:list-disc [&_li]:marker:text-zinc-600">
      {children}
    </ul>
  )
}

export function Ol({ children }: { children: ReactNode }) {
  return (
    <ol className="mt-3 flex flex-col gap-2 pl-5 text-[15.5px] leading-[1.65] text-zinc-300 [&_li]:list-decimal [&_li]:marker:text-zinc-500">
      {children}
    </ol>
  )
}

export function Li({ children }: { children: ReactNode }) {
  return <li>{children}</li>
}

/** Styled table wrapper for docs. */
export function DocsTable({ children }: { children: ReactNode }) {
  return (
    <div className="my-5 overflow-x-auto rounded-lg border border-white/[0.08]">
      <table className="w-full text-left text-[14px]">{children}</table>
    </div>
  )
}
export function Th({ children }: { children: ReactNode }) {
  return (
    <th className="border-b border-white/[0.08] bg-white/[0.03] px-3 py-2 font-medium text-zinc-300">
      {children}
    </th>
  )
}
export function Td({ children }: { children: ReactNode }) {
  return (
    <td className="border-b border-white/[0.04] px-3 py-2 text-zinc-400">
      {children}
    </td>
  )
}

/** Anchor link out of docs. */
export function A({
  href,
  children,
}: {
  href: string
  children: ReactNode
}) {
  const external = href.startsWith("http")
  return (
    <a
      href={href}
      target={external ? "_blank" : undefined}
      rel={external ? "noopener noreferrer" : undefined}
      className="text-[#00FF85] underline decoration-[#00FF85]/30 decoration-1 underline-offset-4 hover:decoration-[#00FF85]"
    >
      {children}
    </a>
  )
}
