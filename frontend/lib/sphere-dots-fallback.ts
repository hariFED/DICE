/**
 * Main-thread fallback for sphere dot computation.
 * Only used if the Web Worker fails to load.
 */

import { geoContains } from "d3-geo"
import { feature } from "topojson-client"
import type { Topology, GeometryCollection } from "topojson-specification"
import type { FeatureCollection, Feature, MultiPolygon, Polygon } from "geojson"
import landTopo from "world-atlas/land-110m.json"

const GLOBE_R = 1.0
const SPHERE_DOT_COUNT = 12000

function xyzToLatLon(x: number, y: number, z: number): [number, number] {
  const lat = Math.asin(Math.max(-1, Math.min(1, y))) * (180 / Math.PI)
  let lon = Math.atan2(z, -x) * (180 / Math.PI) - 180
  if (lon < -180) lon += 360
  if (lon > 180) lon -= 360
  return [lat, lon]
}

export function computeSphereDotsFallback(): {
  positions: Float32Array
  landFlags: Float32Array
} {
  const topo = landTopo as unknown as Topology
  const landFC = feature(topo, topo.objects.land as GeometryCollection) as
    | FeatureCollection<MultiPolygon | Polygon>
    | Feature<MultiPolygon | Polygon>

  const positions: number[] = []
  const landFlags: number[] = []
  const golden = Math.PI * (3 - Math.sqrt(5))

  for (let i = 0; i < SPHERE_DOT_COUNT; i++) {
    const y = 1 - (i / (SPHERE_DOT_COUNT - 1)) * 2
    const r = Math.sqrt(1 - y * y)
    const theta = golden * i
    const x = Math.cos(theta) * r
    const z = Math.sin(theta) * r

    positions.push(x * GLOBE_R, y * GLOBE_R, z * GLOBE_R)
    const [lat, lon] = xyzToLatLon(x, y, z)
    landFlags.push(geoContains(landFC, [lon, lat]) ? 1.0 : 0.0)
  }

  return {
    positions: new Float32Array(positions),
    landFlags: new Float32Array(landFlags),
  }
}
