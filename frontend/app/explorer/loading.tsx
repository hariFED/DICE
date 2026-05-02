import { Skeleton, TableRowSkeleton } from "@/components/shared/Skeleton"

export default function ExplorerLoading() {
  return (
    <div className="pb-12 space-y-10">
      {/* Page header */}
      <div className="border-b border-border pb-6">
        <div className="flex items-end justify-between gap-4 flex-wrap">
          <div>
            <Skeleton className="h-3 w-40 mb-3" />
            <Skeleton className="h-10 w-64" />
          </div>
          <div className="flex gap-2">
            <Skeleton className="h-8 w-24" />
            <Skeleton className="h-8 w-20" />
            <Skeleton className="h-8 w-20" />
          </div>
        </div>
      </div>

      {/* Hero stats */}
      <div className="grid grid-cols-2 md:grid-cols-5 border border-border divide-x divide-border">
        {Array.from({ length: 5 }).map((_, i) => (
          <div key={i} className="p-5 md:p-6">
            <Skeleton className="h-3 w-24 mb-3" />
            <Skeleton className="h-10 w-16" />
          </div>
        ))}
      </div>

      {/* Viz grid */}
      <div className="grid grid-cols-1 lg:grid-cols-12 gap-6">
        <div className="lg:col-span-8 border border-border p-6">
          <Skeleton className="h-3 w-32 mb-4" />
          <Skeleton className="h-40 w-full" />
        </div>
        <div className="lg:col-span-4 border border-border p-6">
          <Skeleton className="h-3 w-28 mb-4" />
          <Skeleton className="h-40 w-full" />
        </div>
      </div>

      {/* Node strip */}
      <div className="border border-border p-6">
        <Skeleton className="h-3 w-24 mb-4" />
        <div className="flex gap-3">
          {Array.from({ length: 8 }).map((_, i) => (
            <Skeleton key={i} className="h-8 w-12" />
          ))}
        </div>
      </div>

      {/* Recent rounds table */}
      <div className="space-y-4">
        <Skeleton className="h-4 w-36" />
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
              {Array.from({ length: 8 }).map((_, i) => (
                <TableRowSkeleton key={i} cols={6} />
              ))}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  )
}
