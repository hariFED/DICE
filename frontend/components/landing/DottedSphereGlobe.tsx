"use client"

import { useEffect, useRef } from "react"
import createGlobe from "cobe"
import { useTheme } from "next-themes"

/**
 * 3D dotted sphere globe (the "shutterstock halftone earth" look) using
 * the 5KB cobe library.
 *
 * - Rotates continuously on the Y axis.
 * - Markers are placed at the 20 DICE node lat/lons.
 * - Recolors on theme switch (dark: white dots on black / light: black on white).
 *
 * cobe renders to canvas via WebGL, so this is significantly cheaper than
 * the SVG dot grid we were using previously and scales with DPR.
 */

const MARKERS: { location: [number, number]; size: number }[] = [
  { location: [37.77, -122.42], size: 0.05 },  // SF
  { location: [40.71, -74.01],  size: 0.05 },  // NYC
  { location: [51.51, -0.13],   size: 0.05 },  // LDN
  { location: [48.86, 2.35],    size: 0.05 },  // PAR
  { location: [52.52, 13.41],   size: 0.05 },  // BER
  { location: [35.68, 139.65],  size: 0.06 },  // TYO
  { location: [1.35, 103.82],   size: 0.05 },  // SGP
  { location: [22.32, 114.17],  size: 0.05 },  // HKG
  { location: [-33.87, 151.21], size: 0.05 },  // SYD
  { location: [55.76, 37.62],   size: 0.05 },  // MSK
  { location: [25.20, 55.27],   size: 0.05 },  // DXB
  { location: [19.08, 72.88],   size: 0.05 },  // BOM
  { location: [-23.55, -46.63], size: 0.05 },  // SAO
  { location: [-34.60, -58.38], size: 0.05 },  // BUE
  { location: [28.61, 77.21],   size: 0.05 },  // DEL
  { location: [47.61, -122.33], size: 0.05 },  // SEA
  { location: [43.65, -79.38],  size: 0.05 },  // TOR
  { location: [59.33, 18.07],   size: 0.05 },  // STO
  { location: [34.05, -118.24], size: 0.05 },  // LAX
  { location: [-26.20, 28.04],  size: 0.05 },  // JHB
]

export function DottedSphereGlobe({
  size = 520,
  className = "",
}: {
  size?: number
  className?: string
}) {
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const phiRef = useRef(0)
  const { resolvedTheme } = useTheme()
  const isDark = resolvedTheme !== "light"

  useEffect(() => {
    if (!canvasRef.current) return

    phiRef.current = 0
    const globe = createGlobe(canvasRef.current, {
      devicePixelRatio: 2,
      width: size * 2,
      height: size * 2,
      phi: 0,
      theta: 0.25,
      dark: isDark ? 1 : 0,
      diffuse: 1.2,
      mapSamples: 16000,
      mapBrightness: isDark ? 5 : 2,
      baseColor: isDark ? [0.35, 0.35, 0.35] : [0.12, 0.12, 0.12],
      markerColor: isDark ? [1, 1, 1] : [0.05, 0.05, 0.05],
      glowColor: isDark ? [1, 1, 1] : [0.95, 0.95, 0.95],
      markers: MARKERS,
    })

    let raf = 0
    const tick = () => {
      phiRef.current += 0.0032
      globe.update({ phi: phiRef.current })
      raf = requestAnimationFrame(tick)
    }
    raf = requestAnimationFrame(tick)

    return () => {
      cancelAnimationFrame(raf)
      globe.destroy()
    }
  }, [isDark, size])

  return (
    <canvas
      ref={canvasRef}
      className={className}
      style={{
        width: size,
        height: size,
        maxWidth: "100%",
        aspectRatio: "1",
        contain: "layout paint size",
      }}
      aria-label="Rotating globe with DICE node locations"
    />
  )
}

export default DottedSphereGlobe
