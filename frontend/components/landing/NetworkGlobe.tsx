"use client"

import { useRef, useMemo, useState, useEffect } from "react"
import { Canvas, useFrame } from "@react-three/fiber"
import * as THREE from "three"
import { geoContains } from "d3-geo"
import { feature } from "topojson-client"
import type { Topology, GeometryCollection } from "topojson-specification"
import type { FeatureCollection, Feature, MultiPolygon, Polygon } from "geojson"
import landTopo from "world-atlas/land-110m.json"

// ─── Constants ───────────────────────────────────────────────

const GLOBE_R = 1.0
const SPHERE_DOT_COUNT = 12000

const NODES = [
  { name: "SF",  lat: 37.77, lon: -122.42 },
  { name: "NYC", lat: 40.71, lon: -74.01 },
  { name: "LDN", lat: 51.51, lon: -0.13 },
  { name: "PAR", lat: 48.86, lon: 2.35 },
  { name: "BER", lat: 52.52, lon: 13.41 },
  { name: "TYO", lat: 35.68, lon: 139.65 },
  { name: "SGP", lat: 1.35, lon: 103.82 },
  { name: "HKG", lat: 22.32, lon: 114.17 },
  { name: "SYD", lat: -33.87, lon: 151.21 },
  { name: "MSK", lat: 55.76, lon: 37.62 },
  { name: "DXB", lat: 25.20, lon: 55.27 },
  { name: "BOM", lat: 19.08, lon: 72.88 },
  { name: "SAO", lat: -23.55, lon: -46.63 },
  { name: "BUE", lat: -34.60, lon: -58.38 },
  { name: "DEL", lat: 28.61, lon: 77.21 },
  { name: "SEA", lat: 47.61, lon: -122.33 },
  { name: "TOR", lat: 43.65, lon: -79.38 },
  { name: "STO", lat: 59.33, lon: 18.07 },
  { name: "LAX", lat: 34.05, lon: -118.24 },
  { name: "JHB", lat: -26.20, lon: 28.04 },
]

const CONNECTIONS: [number, number][] = [
  [0, 1],   // SF – NYC
  [0, 18],  // SF – LAX
  [0, 15],  // SF – SEA
  [0, 5],   // SF – TYO
  [1, 2],   // NYC – LDN
  [1, 16],  // NYC – TOR
  [2, 3],   // LDN – PAR
  [2, 4],   // LDN – BER
  [2, 17],  // LDN – STO
  [3, 4],   // PAR – BER
  [4, 9],   // BER – MSK
  [5, 6],   // TYO – SGP
  [5, 7],   // TYO – HKG
  [6, 7],   // SGP – HKG
  [6, 8],   // SGP – SYD
  [10, 11], // DXB – BOM
  [10, 14], // DXB – DEL
  [11, 14], // BOM – DEL
  [12, 13], // SAO – BUE
  [19, 10], // JHB – DXB
  [9, 17],  // MSK – STO
]

// ─── Utilities ───────────────────────────────────────────────

function latLonToVec3(lat: number, lon: number, r = GLOBE_R): THREE.Vector3 {
  const phi = (90 - lat) * (Math.PI / 180)
  const theta = (lon + 180) * (Math.PI / 180)
  return new THREE.Vector3(
    -r * Math.sin(phi) * Math.cos(theta),
    r * Math.cos(phi),
    r * Math.sin(phi) * Math.sin(theta),
  )
}

function xyzToLatLon(x: number, y: number, z: number): [number, number] {
  const lat = Math.asin(Math.max(-1, Math.min(1, y))) * (180 / Math.PI)
  let lon = Math.atan2(z, -x) * (180 / Math.PI) - 180
  if (lon < -180) lon += 360
  if (lon > 180) lon -= 360
  return [lat, lon]
}

/**
 * Fibonacci-sphere dot distribution covering the entire surface.
 * Each dot carries a `land` flag (1 = continent, 0 = ocean).
 * Ocean dots render dim + small to give the sphere its shape;
 * land dots render bright + large to show continents.
 */
function computeSphereDots(): {
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

function createArcCurve(
  from: THREE.Vector3,
  to: THREE.Vector3,
): THREE.QuadraticBezierCurve3 {
  const mid = new THREE.Vector3().addVectors(from, to).multiplyScalar(0.5)
  const dist = from.distanceTo(to)
  // Arc rises proportionally to distance, capped so it stays close to globe
  const arcHeight = Math.min(dist * 0.2, 0.25)
  mid.normalize().multiplyScalar(GLOBE_R + arcHeight)
  return new THREE.QuadraticBezierCurve3(from, mid, to)
}

// ─── Shaders ─────────────────────────────────────────────────

const DOT_VERTEX = /* glsl */ `
  attribute float aLand;
  varying float vAlpha;

  void main() {
    vec3 n = normalize(normalMatrix * normalize(position));
    vec4 mvPos = modelViewMatrix * vec4(position, 1.0);
    vec3 viewDir = normalize(-mvPos.xyz);
    float facing = dot(n, viewDir);

    // Backface dimming — softer falloff so the sphere edge is visible
    float backfade = smoothstep(-0.25, 0.35, facing);

    // Land: bright + large, Ocean: visible but subtle
    float baseAlpha = mix(0.22, 0.9, aLand);
    float baseSize  = mix(1.4, 2.8, aLand);

    vAlpha = backfade * baseAlpha;
    gl_PointSize = baseSize;
    gl_Position = projectionMatrix * mvPos;
  }
`

const DOT_FRAGMENT = /* glsl */ `
  varying float vAlpha;
  void main() {
    vec2 c = gl_PointCoord - 0.5;
    if (length(c) > 0.5) discard;
    float edge = smoothstep(0.5, 0.3, length(c));
    gl_FragColor = vec4(1.0, 1.0, 1.0, vAlpha * edge);
  }
`

// ─── Scene Sub-components ────────────────────────────────────

/**
 * Full-sphere dots. Rendered first (renderOrder -1) with no depth testing
 * so both front and back dots are always visible. The vertex shader handles
 * backface dimming in the alpha channel.
 */
function SphereDots({
  positions,
  landFlags,
}: {
  positions: Float32Array
  landFlags: Float32Array
}) {
  const geo = useMemo(() => {
    const g = new THREE.BufferGeometry()
    g.setAttribute("position", new THREE.BufferAttribute(positions, 3))
    g.setAttribute("aLand", new THREE.BufferAttribute(landFlags, 1))
    return g
  }, [positions, landFlags])

  const mat = useMemo(
    () =>
      new THREE.ShaderMaterial({
        vertexShader: DOT_VERTEX,
        fragmentShader: DOT_FRAGMENT,
        transparent: true,
        depthWrite: false,
        depthTest: false,
      }),
    [],
  )

  return <points geometry={geo} material={mat} renderOrder={-1} />
}

/**
 * Invisible sphere that only writes to the depth buffer.
 * Rendered at renderOrder 0 so that arcs / markers / packets
 * behind the globe are occluded, but dots (renderOrder -1,
 * depthTest=false) stay visible on all sides.
 */
function DepthSphere() {
  const mat = useMemo(
    () =>
      new THREE.MeshBasicMaterial({
        colorWrite: false,
        depthWrite: true,
      }),
    [],
  )
  return (
    <mesh material={mat} renderOrder={0}>
      <sphereGeometry args={[GLOBE_R * 0.997, 64, 64]} />
    </mesh>
  )
}

/**
 * 3D dice cubes at each node location — oriented outward from the
 * globe surface, gently spinning on their own axis. Each dice is a
 * wireframe cube with a subtle glass fill, representing a physical
 * DICE hardware device deployed in that country.
 */
function DiceMarkers() {
  const DICE_SIZE = 0.022

  const diceData = useMemo(() => {
    const up = new THREE.Vector3(0, 1, 0)
    return NODES.map((n) => {
      const pos = latLonToVec3(n.lat, n.lon, GLOBE_R * 1.01)
      const lookTarget = new THREE.Vector3(0, 0, 0)
      const mat4 = new THREE.Matrix4().lookAt(pos, lookTarget, up)
      const quat = new THREE.Quaternion().setFromRotationMatrix(mat4)
      return { pos, quat }
    })
  }, [])

  const edgeGeo = useMemo(
    () => new THREE.EdgesGeometry(new THREE.BoxGeometry(DICE_SIZE, DICE_SIZE, DICE_SIZE)),
    [],
  )

  return (
    <group renderOrder={1}>
      {diceData.map(({ pos, quat }, i) => (
        <group key={i} position={pos} quaternion={quat}>
          <group>
            {/* Glass-fill cube */}
            <mesh renderOrder={1}>
              <boxGeometry args={[DICE_SIZE, DICE_SIZE, DICE_SIZE]} />
              <meshBasicMaterial
                color="#ffffff"
                transparent
                opacity={0.08}
                depthTest
              />
            </mesh>
            {/* Wireframe edges */}
            <lineSegments geometry={edgeGeo} renderOrder={1}>
              <lineBasicMaterial
                color="#ffffff"
                transparent
                opacity={0.7}
              />
            </lineSegments>
          </group>
        </group>
      ))}
    </group>
  )
}

function Arcs({ curves }: { curves: THREE.QuadraticBezierCurve3[] }) {
  const arcGeos = useMemo(
    () => curves.map((c) => new THREE.TubeGeometry(c, 48, 0.003, 4, false)),
    [curves],
  )

  return (
    <group renderOrder={1}>
      {arcGeos.map((geo, i) => (
        <mesh key={i} geometry={geo} renderOrder={1}>
          <meshBasicMaterial
            color="#ffffff"
            transparent
            opacity={0.25}
            depthTest
          />
        </mesh>
      ))}
    </group>
  )
}

function DataPackets({ curves }: { curves: THREE.QuadraticBezierCurve3[] }) {
  const packets = useRef(
    curves.map(() => ({
      t: Math.random(),
      speed: 0.1 + Math.random() * 0.2,
    })),
  )
  const headRefs = useRef<(THREE.Mesh | null)[]>([])
  const glowRefs = useRef<(THREE.Mesh | null)[]>([])

  useFrame((_, delta) => {
    packets.current.forEach((p, i) => {
      p.t = (p.t + p.speed * delta) % 1
      const pos = curves[i].getPoint(p.t)
      headRefs.current[i]?.position.copy(pos)
      glowRefs.current[i]?.position.copy(pos)
    })
  })

  return (
    <group renderOrder={2}>
      {curves.map((_, i) => (
        <group key={i}>
          <mesh ref={(el) => (headRefs.current[i] = el)} renderOrder={2}>
            <sphereGeometry args={[0.008, 8, 8]} />
            <meshBasicMaterial color="#ffffff" depthTest />
          </mesh>
          <mesh ref={(el) => (glowRefs.current[i] = el)} renderOrder={2}>
            <sphereGeometry args={[0.022, 8, 8]} />
            <meshBasicMaterial
              color="#ffffff"
              transparent
              opacity={0.12}
              depthTest
            />
          </mesh>
        </group>
      ))}
    </group>
  )
}

/**
 * Soft, diffuse outer glow — a wide halo that smoothly fades out,
 * not a hard ring. Uses a large BackSide sphere so the glow spreads.
 */
function GlobeGlow() {
  const mat = useMemo(
    () =>
      new THREE.ShaderMaterial({
        vertexShader: /* glsl */ `
          varying vec3 vNormal;
          varying vec3 vViewDir;
          void main() {
            vNormal = normalize(normalMatrix * normal);
            vec4 mvPos = modelViewMatrix * vec4(position, 1.0);
            vViewDir = normalize(-mvPos.xyz);
            gl_Position = projectionMatrix * mvPos;
          }
        `,
        fragmentShader: /* glsl */ `
          varying vec3 vNormal;
          varying vec3 vViewDir;
          void main() {
            float rim = 1.0 - max(0.0, dot(vNormal, vViewDir));
            // Wide, smooth halo — low power = spreads far
            float outerGlow = pow(rim, 2.0) * 0.12;
            // Medium glow band
            float midGlow = pow(rim, 4.0) * 0.18;
            float alpha = outerGlow + midGlow;
            gl_FragColor = vec4(1.0, 1.0, 1.0, alpha);
          }
        `,
        transparent: true,
        depthWrite: false,
        side: THREE.BackSide,
      }),
    [],
  )

  return (
    <mesh material={mat} renderOrder={-2}>
      <sphereGeometry args={[GLOBE_R * 1.08, 64, 64]} />
    </mesh>
  )
}

/**
 * Apple-style glass sheen overlay — sits on top of the dotted sphere.
 * Fresnel transparency (clear in center, subtly opaque at edges) +
 * a specular highlight simulating light hitting curved glass.
 */
function GlassOverlay() {
  const mat = useMemo(
    () =>
      new THREE.ShaderMaterial({
        vertexShader: /* glsl */ `
          varying vec3 vNormal;
          varying vec3 vViewDir;
          varying vec3 vWorldPos;
          void main() {
            vNormal = normalize(normalMatrix * normal);
            vec4 mvPos = modelViewMatrix * vec4(position, 1.0);
            vViewDir = normalize(-mvPos.xyz);
            vWorldPos = (modelMatrix * vec4(position, 1.0)).xyz;
            gl_Position = projectionMatrix * mvPos;
          }
        `,
        fragmentShader: /* glsl */ `
          varying vec3 vNormal;
          varying vec3 vViewDir;
          varying vec3 vWorldPos;
          void main() {
            float facing = max(0.0, dot(vNormal, vViewDir));
            float fresnel = pow(1.0 - facing, 3.0);

            // Specular highlight — simulates a light source top-right
            vec3 lightDir = normalize(vec3(0.8, 1.0, 0.6));
            vec3 halfDir = normalize(lightDir + vViewDir);
            float spec = pow(max(0.0, dot(vNormal, halfDir)), 48.0);

            // Glass tint — very slight cool shift at edges
            vec3 color = mix(vec3(1.0), vec3(0.85, 0.9, 1.0), fresnel);

            // Combine: edge fresnel + specular highlight
            float alpha = fresnel * 0.07 + spec * 0.25;

            gl_FragColor = vec4(color, alpha);
          }
        `,
        transparent: true,
        depthWrite: false,
      }),
    [],
  )

  return (
    <mesh material={mat} renderOrder={3}>
      <sphereGeometry args={[GLOBE_R * 1.001, 64, 64]} />
    </mesh>
  )
}

// ─── Main Scene ──────────────────────────────────────────────

function GlobeScene() {
  const groupRef = useRef<THREE.Group>(null)
  const [dotData, setDotData] = useState<{
    positions: Float32Array
    landFlags: Float32Array
  } | null>(null)

  useEffect(() => {
    setDotData(computeSphereDots())
  }, [])

  const arcCurves = useMemo(
    () =>
      CONNECTIONS.map(([a, b]) => {
        const from = latLonToVec3(NODES[a].lat, NODES[a].lon)
        const to = latLonToVec3(NODES[b].lat, NODES[b].lon)
        return createArcCurve(from, to)
      }),
    [],
  )

  useFrame((_, delta) => {
    if (groupRef.current) {
      groupRef.current.rotation.y += delta * 0.08
    }
  })

  return (
    <group ref={groupRef} rotation={[0.25, 0, 0]}>
      {/* 1. Sphere dots — always visible, shader handles backface dimming */}
      {dotData && (
        <SphereDots
          positions={dotData.positions}
          landFlags={dotData.landFlags}
        />
      )}

      {/* 2. Depth-only sphere — invisible, creates Z-barrier to hide
             backside arcs / markers / packets */}
      <DepthSphere />

      {/* 3. Network visualization — depth-tested against the barrier */}
      <DiceMarkers />
      <Arcs curves={arcCurves} />
      <DataPackets curves={arcCurves} />

      {/* 4. Glass overlay — Apple-style sheen on the globe surface */}
      <GlassOverlay />

    </group>
  )
}

// ─── Exported Component ──────────────────────────────────────

export function NetworkGlobe({ className }: { className?: string }) {
  const [mounted, setMounted] = useState(false)
  useEffect(() => setMounted(true), [])

  if (!mounted) return <div className={className} />

  return (
    <div className={className}>
      <Canvas
        camera={{ position: [0, 0, 3.0], fov: 45 }}
        gl={{ antialias: true, alpha: true }}
        onCreated={({ gl }) => {
          gl.setClearColor(0x000000, 0)
        }}
        style={{ background: "none" }}
        dpr={[1, 2]}
      >
        <GlobeScene />
      </Canvas>
    </div>
  )
}

export default NetworkGlobe
