"use client"

import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { ReactLenis } from "lenis/react"
import { ThemeProvider } from "next-themes"
import { useState, type ReactNode } from "react"
import { GlobeReadyProvider } from "@/lib/globe-ready-context"

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
    <ThemeProvider
      attribute="class"
      defaultTheme="dark"
      forcedTheme="dark"
      disableTransitionOnChange
    >
      <QueryClientProvider client={queryClient}>
        <GlobeReadyProvider>
          <ReactLenis root options={{ lerp: 0.1, duration: 1.2 }}>
            {children}
          </ReactLenis>
        </GlobeReadyProvider>
      </QueryClientProvider>
    </ThemeProvider>
  )
}
