import type { ReactNode } from "react"
import { cn } from "@/lib/utils"

export function H1({ children }: { children: ReactNode }) {
  return (
    <h1 className="font-sans text-[34px] font-semibold tracking-tight text-foreground leading-[1.15]">
      {children}
    </h1>
  )
}

export function Lead({ children }: { children: ReactNode }) {
  return (
    <p className="mt-3 font-mono text-[15px] leading-[1.65] text-muted-foreground">
      {children}
    </p>
  )
}

export function H2({ id, children }: { id: string; children: ReactNode }) {
  return (
    <h2
      id={id}
      className="group mt-12 scroll-mt-24 border-b border-border pb-2 font-sans text-[22px] font-semibold tracking-tight text-foreground"
    >
      <a href={`#${id}`} className="relative no-underline">
        <span className="pointer-events-none absolute -left-6 hidden text-muted-foreground group-hover:block">#</span>
        {children}
      </a>
    </h2>
  )
}

export function H3({ id, children }: { id: string; children: ReactNode }) {
  return (
    <h3
      id={id}
      className="mt-8 scroll-mt-24 font-sans text-[17px] font-semibold tracking-tight text-foreground"
    >
      {children}
    </h3>
  )
}

export function P({ children, className }: { children: ReactNode; className?: string }) {
  return (
    <p className={cn("mt-4 font-mono text-[14.5px] leading-[1.7] text-foreground/80", className)}>
      {children}
    </p>
  )
}

/** Inline `code` chip. */
export function Code({ children }: { children: ReactNode }) {
  return (
    <code className="rounded-sm border border-border bg-muted px-1.5 py-0.5 font-mono text-[13px] text-foreground">
      {children}
    </code>
  )
}

export function Ul({ children }: { children: ReactNode }) {
  return (
    <ul className="mt-3 flex flex-col gap-1.5 pl-5 font-mono text-[14.5px] leading-[1.65] text-foreground/80 [&_li]:list-['›_'] [&_li]:marker:text-muted-foreground">
      {children}
    </ul>
  )
}

export function Ol({ children }: { children: ReactNode }) {
  return (
    <ol className="mt-3 flex flex-col gap-2 pl-5 font-mono text-[14.5px] leading-[1.65] text-foreground/80 [&_li]:list-decimal [&_li]:marker:text-muted-foreground">
      {children}
    </ol>
  )
}

export function Li({ children }: { children: ReactNode }) {
  return <li>{children}</li>
}

export function DocsTable({ children }: { children: ReactNode }) {
  return (
    <div className="my-5 overflow-x-auto border border-border">
      <table className="w-full text-left text-[14px] font-mono">{children}</table>
    </div>
  )
}
export function Th({ children }: { children: ReactNode }) {
  return (
    <th className="border-b border-border bg-muted/30 px-3 py-2 ascii-label text-[10px]">
      {children}
    </th>
  )
}
export function Td({ children }: { children: ReactNode }) {
  return (
    <td className="border-b border-border px-3 py-2 text-foreground/80">
      {children}
    </td>
  )
}

export function A({ href, children }: { href: string; children: ReactNode }) {
  const external = href.startsWith("http")
  return (
    <a
      href={href}
      target={external ? "_blank" : undefined}
      rel={external ? "noopener noreferrer" : undefined}
      className="text-foreground underline decoration-muted-foreground/40 decoration-1 underline-offset-4 hover:decoration-foreground"
    >
      {children}
    </a>
  )
}
