"use client"

import { useEffect, useMemo, useState } from "react"
import { motion } from "framer-motion"
import { geoEqualEarth, geoContains } from "d3-geo"
import { feature } from "topojson-client"
import type { Topology, GeometryCollection } from "topojson-specification"
import type { Feature, FeatureCollection, MultiPolygon, Polygon } from "geojson"
// `world-atlas` ships pre-simplified topojson. 110m is the lightest (60 KB)
// and has smooth enough coastlines for a dot-halftone rendering.
import landTopo from "world-atlas/land-110m.json"

/**
 * Flat dotted world map — the halftone look used by Reboot / Ultralogos /
 * many Awwwards editorial sites.
 *
 * Rendering approach:
 *   1. Project the world (EqualEarth) onto a fixed SVG viewBox.
 *   2. Walk a regular grid; for each grid cell, invert-project to lat/lon and
 *      ask d3's geoContains() whether that point is on land.
 *   3. Emit SVG <circle>s for each land cell + larger markers for the 20
 *      node cities, which pulse with staggered phase offsets.
 *
 * The land grid is computed once (useMemo) and cached; markers animate via
 * pure CSS so we don't re-render the 800+ dots on every frame.
 */

// Network node cities — matches the real DICE node geography.
const NODES: { name: string; lat: number; lon: number }[] = [
  { name: "SF",  lat: 37.77,  lon: -122.42 },
  { name: "NYC", lat: 40.71,  lon: -74.01  },
  { name: "LDN", lat: 51.51,  lon: -0.13   },
  { name: "PAR", lat: 48.86,  lon: 2.35    },
  { name: "BER", lat: 52.52,  lon: 13.41   },
  { name: "TYO", lat: 35.68,  lon: 139.65  },
  { name: "SGP", lat: 1.35,   lon: 103.82  },
  { name: "HKG", lat: 22.32,  lon: 114.17  },
  { name: "SYD", lat: -33.87, lon: 151.21  },
  { name: "MSK", lat: 55.76,  lon: 37.62   },
  { name: "DXB", lat: 25.20,  lon: 55.27   },
  { name: "BOM", lat: 19.08,  lon: 72.88   },
  { name: "SAO", lat: -23.55, lon: -46.63  },
  { name: "BUE", lat: -34.60, lon: -58.38  },
  { name: "DEL", lat: 28.61,  lon: 77.21   },
  { name: "SEA", lat: 47.61,  lon: -122.33 },
  { name: "TOR", lat: 43.65,  lon: -79.38  },
  { name: "STO", lat: 59.33,  lon: 18.07   },
  { name: "LAX", lat: 34.05,  lon: -118.24 },
  { name: "JHB", lat: -26.20, lon: 28.04   },
]

const VIEW_W = 960
const VIEW_H = 480
const DOT_SPACING = 8   // px — smaller = denser map, slower to build
const DOT_R = 1.5

type DotsResult = {
  dots: [number, number][]  // [x, y] in viewBox px
  markers: { x: number; y: number; name: string }[]
}

function buildDots(): DotsResult {
  // d3's GeoSphere is a valid input to fitSize — the upstream types just
  // don't model it explicitly, so cast via unknown.
  const projection = geoEqualEarth()
    .fitSize([VIEW_W, VIEW_H], { type: "Sphere" } as unknown as Feature)

  // topojson-client's `feature` returns either a Feature or FeatureCollection
  // depending on the input geometry type; our `land` object is a
  // GeometryCollection of a single MultiPolygon so we get a FeatureCollection.
  const topo = landTopo as unknown as Topology
  const landFC = feature(topo, topo.objects.land as GeometryCollection) as
    | FeatureCollection<MultiPolygon | Polygon>
    | Feature<MultiPolygon | Polygon>

  const dots: [number, number][] = []
  for (let y = 0; y < VIEW_H; y += DOT_SPACING) {
    for (let x = 0; x < VIEW_W; x += DOT_SPACING) {
      const inv = projection.invert?.([x, y])
      if (!inv) continue
      if (geoContains(landFC, inv)) {
        dots.push([x, y])
      }
    }
  }

  const markers = NODES.map(({ name, lat, lon }) => {
    const p = projection([lon, lat])
    return p ? { x: p[0], y: p[1], name } : null
  }).filter(Boolean) as { x: number; y: number; name: string }[]

  return { dots, markers }
}

export function DottedWorldMap({ className = "" }: { className?: string }) {
  // Expensive (~30ms) so we do it once. Render SSR shows the cities only;
  // dots land after hydration. Acceptable — avoids shipping a 60KB JSON to
  // the client as pre-computed data.
  const [computed, setComputed] = useState<DotsResult | null>(null)
  useEffect(() => {
    setComputed(buildDots())
  }, [])

  const dots = computed?.dots ?? []
  const markers = computed?.markers ?? []

  return (
    <svg
      className={`w-full h-auto ${className}`}
      viewBox={`0 0 ${VIEW_W} ${VIEW_H}`}
      role="img"
      aria-label="World map showing DICE node locations"
    >
      {/* Land dots */}
      <g fill="currentColor" className="text-foreground/60">
        {dots.map(([x, y], i) => (
          <circle key={i} cx={x} cy={y} r={DOT_R} />
        ))}
      </g>

      {/* Node markers — larger rings + center dot, staggered pulse */}
      <g>
        {markers.map((m, i) => (
          <motion.g
            key={m.name}
            initial={{ opacity: 0, scale: 0 }}
            animate={{ opacity: 1, scale: 1 }}
            transition={{ delay: 0.3 + i * 0.04, duration: 0.3 }}
          >
            {/* Outer pulsing ring */}
            <motion.circle
              cx={m.x}
              cy={m.y}
              r={4}
              fill="none"
              strokeWidth={1}
              className="stroke-foreground"
              initial={{ opacity: 0.6, scale: 1 }}
              animate={{ opacity: [0.6, 0, 0.6], scale: [1, 2.5, 1] }}
              transition={{
                duration: 2,
                repeat: Infinity,
                delay: (i % 5) * 0.4,
                ease: "easeOut",
              }}
              style={{ transformOrigin: `${m.x}px ${m.y}px` }}
            />
            {/* Solid center */}
            <circle cx={m.x} cy={m.y} r={2.5} className="fill-foreground" />
          </motion.g>
        ))}
      </g>
    </svg>
  )
}

export default DottedWorldMap
