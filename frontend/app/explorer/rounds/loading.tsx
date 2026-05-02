import { Skeleton, TableRowSkeleton } from "@/components/shared/Skeleton"

export default function RoundsLoading() {
  return (
    <div className="pb-12 space-y-6">
      {/* Header */}
      <div className="flex items-end justify-between gap-4 flex-wrap">
        <Skeleton className="h-10 w-40" />
        <div className="flex gap-2">
          <Skeleton className="h-8 w-24" />
          <Skeleton className="h-8 w-20" />
          <Skeleton className="h-8 w-20" />
        </div>
      </div>

      {/* Filter row */}
      <div className="flex items-center gap-2 flex-wrap">
        <Skeleton className="h-3 w-12 mr-2" />
        {Array.from({ length: 4 }).map((_, i) => (
          <Skeleton key={i} className="h-7 w-20" />
        ))}
        <Skeleton className="ml-auto h-3 w-16" />
      </div>

      {/* Rounds table */}
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
            {Array.from({ length: 12 }).map((_, i) => (
              <TableRowSkeleton key={i} cols={6} />
            ))}
          </tbody>
        </table>
      </div>
    </div>
  )
}
