"use client"

import { useEffect, useRef, useCallback } from "react"
import createGlobe from "cobe"
import type { Marker } from "cobe"

interface GlobeProps {
  markers?: Array<{ location: [number, number]; size: number }>
  className?: string
}

function GlobeCanvas({ markers = [], className }: GlobeProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const phiRef = useRef(0)
  const globeRef = useRef<ReturnType<typeof createGlobe> | null>(null)
  const rafRef = useRef<number | null>(null)

  const startAnimation = useCallback(() => {
    if (!globeRef.current) return

    function tick() {
      phiRef.current += 0.003
      globeRef.current?.update({ phi: phiRef.current })
      rafRef.current = requestAnimationFrame(tick)
    }

    rafRef.current = requestAnimationFrame(tick)
  }, [])

  useEffect(() => {
    if (!canvasRef.current) return

    const canvas = canvasRef.current
    const container = canvas.parentElement
    const width = container?.clientWidth ?? 600
    const height = container?.clientHeight ?? 600

    const cobeMarkers: Marker[] = markers.map((m) => ({
      location: m.location,
      size: m.size,
    }))

    globeRef.current = createGlobe(canvas, {
      devicePixelRatio: 2,
      width: width,
      height: height,
      phi: 0,
      theta: 0.3,
      dark: 1,
      diffuse: 3,
      mapSamples: 16000,
      mapBrightness: 1.2,
      baseColor: [0.1, 0.1, 0.1],
      markerColor: [0, 1, 0.52],
      glowColor: [0, 0.3, 0.15],
      markers: cobeMarkers,
      scale: 1.05,
      offset: [0, 0],
    })

    startAnimation()

    // Handle resize
    const handleResize = () => {
      if (!container || !globeRef.current) return
      const w = container.clientWidth
      const h = container.clientHeight
      globeRef.current.update({ width: w, height: h })
    }

    window.addEventListener("resize", handleResize)

    return () => {
      window.removeEventListener("resize", handleResize)
      if (rafRef.current !== null) {
        cancelAnimationFrame(rafRef.current)
      }
      globeRef.current?.destroy()
      globeRef.current = null
    }
  }, [markers, startAnimation])

  return (
    <div className={className} style={{ width: "100%", height: "100%" }}>
      <canvas
        ref={canvasRef}
        style={{
          width: "100%",
          height: "100%",
          contain: "layout paint size",
          opacity: 0,
          transition: "opacity 1s ease",
        }}
        onContextMenu={(e) => e.preventDefault()}
        onLoad={(e) => {
          ;(e.target as HTMLCanvasElement).style.opacity = "1"
        }}
      />
    </div>
  )
}

// Make canvas visible after globe renders
function GlobeWithFadeIn(props: GlobeProps) {
  const containerRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    // Fade in the canvas after a short delay to let the globe render
    const timer = setTimeout(() => {
      const canvas = containerRef.current?.querySelector("canvas")
      if (canvas) {
        canvas.style.opacity = "1"
      }
    }, 100)
    return () => clearTimeout(timer)
  }, [])

  return (
    <div ref={containerRef} style={{ width: "100%", height: "100%" }}>
      <GlobeCanvas {...props} />
    </div>
  )
}

export default GlobeWithFadeIn
