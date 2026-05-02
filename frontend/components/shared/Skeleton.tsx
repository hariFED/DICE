"use client"

import { cn } from "@/lib/utils"

/**
 * Shimmer skeleton block — matches the DICE aesthetic.
 * Renders a pulsing rectangle used as a placeholder while content loads.
 */
export function Skeleton({
  className,
  ...props
}: React.HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      className={cn(
        "animate-pulse rounded-sm bg-muted/40",
        className,
      )}
      {...props}
    />
  )
}

/** Full-width table row skeleton with N cells. */
export function TableRowSkeleton({ cols = 6 }: { cols?: number }) {
  return (
    <tr className="border-b border-border last:border-0">
      {Array.from({ length: cols }).map((_, i) => (
        <td key={i} className="px-3 py-2.5">
          <Skeleton className="h-4 w-full max-w-[120px]" />
        </td>
      ))}
    </tr>
  )
}
