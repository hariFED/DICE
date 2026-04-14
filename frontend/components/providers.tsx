"use client"

import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { ReactLenis } from "lenis/react"
import { useState, type ReactNode } from "react"

export function Providers({ children }: { children: ReactNode }) {
  const [queryClient] = useState(
    () =>
      new QueryClient({
        defaultOptions: {
          queries: {
            staleTime: 10000,
            refetchOnWindowFocus: false,
          },
        },
      })
  )

  return (
    <QueryClientProvider client={queryClient}>
      <ReactLenis root options={{ lerp: 0.1, duration: 1.2 }}>
        {children}
      </ReactLenis>
    </QueryClientProvider>
  )
}
