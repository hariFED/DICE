"use client"

import { createContext, useContext, useState, useCallback, type ReactNode } from "react"

interface GlobeReadyContextValue {
  globeReady: boolean
  setGlobeReady: () => void
}

const GlobeReadyContext = createContext<GlobeReadyContextValue>({
  globeReady: false,
  setGlobeReady: () => {},
})

export function GlobeReadyProvider({ children }: { children: ReactNode }) {
  const [globeReady, setReady] = useState(false)
  const setGlobeReady = useCallback(() => setReady(true), [])

  return (
    <GlobeReadyContext.Provider value={{ globeReady, setGlobeReady }}>
      {children}
    </GlobeReadyContext.Provider>
  )
}

export const useGlobeReady = () => useContext(GlobeReadyContext)
