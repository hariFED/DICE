import { Skeleton, TableRowSkeleton } from "@/components/shared/Skeleton"

export default function NodesLoading() {
  return (
    <div className="pb-12 space-y-6">
      {/* Header */}
      <div className="flex items-end justify-between gap-4 flex-wrap">
        <Skeleton className="h-10 w-32" />
        <div className="flex gap-2">
          <Skeleton className="h-8 w-24" />
          <Skeleton className="h-8 w-20" />
          <Skeleton className="h-8 w-20" />
        </div>
      </div>

      {/* Uptime bar */}
      <div className="border border-border p-5">
        <div className="flex items-center justify-between mb-3">
          <Skeleton className="h-4 w-48" />
          <Skeleton className="h-3 w-16" />
        </div>
        <Skeleton className="h-4 w-full" />
      </div>

      {/* Nodes table */}
      <div className="border border-border">
        <table className="w-full text-sm font-mono">
          <thead>
            <tr className="border-b border-border bg-muted/30">
              {Array.from({ length: 6 }).map((_, i) => (
                <th key={i} className="text-left px-3 py-2">
                  <Skeleton className="h-3 w-20" />
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {Array.from({ length: 10 }).map((_, i) => (
              <TableRowSkeleton key={i} cols={6} />
            ))}
          </tbody>
        </table>
      </div>
    </div>
  )
}
