"use client"

import { useState, useEffect } from "react"

interface SphereDotData {
  positions: Float32Array
  landFlags: Float32Array
}

const IDB_NAME = "dice-globe-cache"
const IDB_STORE = "dots"
const IDB_KEY = "sphere-dots-v1"

// Module-level cache — instant on re-renders / navigations within same session
let cachedDots: SphereDotData | null = null

// ─── IndexedDB helpers ──────────────────────────────────────

function openDB(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const req = indexedDB.open(IDB_NAME, 1)
    req.onupgradeneeded = () => {
      req.result.createObjectStore(IDB_STORE)
    }
    req.onsuccess = () => resolve(req.result)
    req.onerror = () => reject(req.error)
  })
}

async function readFromIDB(): Promise<SphereDotData | null> {
  try {
    const db = await openDB()
    return new Promise((resolve) => {
      const tx = db.transaction(IDB_STORE, "readonly")
      const store = tx.objectStore(IDB_STORE)
      const req = store.get(IDB_KEY)
      req.onsuccess = () => {
        const val = req.result
        if (
          val &&
          val.positions instanceof ArrayBuffer &&
          val.landFlags instanceof ArrayBuffer
        ) {
          resolve({
            positions: new Float32Array(val.positions),
            landFlags: new Float32Array(val.landFlags),
          })
        } else {
          resolve(null)
        }
      }
      req.onerror = () => resolve(null)
    })
  } catch {
    return null
  }
}

async function writeToIDB(data: SphereDotData): Promise<void> {
  try {
    const db = await openDB()
    const tx = db.transaction(IDB_STORE, "readwrite")
    const store = tx.objectStore(IDB_STORE)
    // Store as plain ArrayBuffers (IDB handles them natively)
    store.put(
      {
        positions: data.positions.buffer.slice(0),
        landFlags: data.landFlags.buffer.slice(0),
      },
      IDB_KEY,
    )
  } catch {
    // Silently fail — caching is best-effort
  }
}

// ─── Worker spawner ─────────────────────────────────────────

function computeViaWorker(): Promise<SphereDotData> {
  return new Promise((resolve, reject) => {
    const worker = new Worker(
      new URL("../workers/sphere-dots.worker.ts", import.meta.url),
    )
    worker.onmessage = (e: MessageEvent<SphereDotData>) => {
      resolve(e.data)
      worker.terminate()
    }
    worker.onerror = (err) => {
      reject(err)
      worker.terminate()
    }
  })
}

// ─── Hook ───────────────────────────────────────────────────

export function useSphereDots(): { dotData: SphereDotData | null } {
  const [dotData, setDotData] = useState<SphereDotData | null>(cachedDots)

  useEffect(() => {
    // Already have data from module cache
    if (cachedDots) {
      setDotData(cachedDots)
      return
    }

    let cancelled = false

    async function load() {
      // Try IndexedDB first
      const fromIDB = await readFromIDB()
      if (fromIDB) {
        cachedDots = fromIDB
        if (!cancelled) setDotData(fromIDB)
        return
      }

      // Fall back to Web Worker computation
      try {
        const fromWorker = await computeViaWorker()
        cachedDots = fromWorker
        if (!cancelled) setDotData(fromWorker)
        // Persist for future sessions
        writeToIDB(fromWorker)
      } catch {
        // Worker failed — fall back to main-thread computation as last resort
        const { computeSphereDotsFallback } = await import(
          "./sphere-dots-fallback"
        )
        const fallback = computeSphereDotsFallback()
        cachedDots = fallback
        if (!cancelled) setDotData(fallback)
        writeToIDB(fallback)
      }
    }

    load()
    return () => {
      cancelled = true
    }
  }, [])

  return { dotData }
}
