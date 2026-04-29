"use client"

import { useRef, useMemo, useState, useEffect, useCallback } from "react"
import { Canvas, useFrame } from "@react-three/fiber"
import * as THREE from "three"
import { useSphereDots } from "@/lib/use-sphere-dots"
import { useGlobeReady } from "@/lib/globe-ready-context"

// ─── Constants ───────────────────────────────────────────────

const GLOBE_R = 1.0
const NODES = [
  { name: "SF",  lat: 37.77, lon: -122.42 },  // 0
  { name: "NYC", lat: 40.71, lon: -74.01 },   // 1
  { name: "LDN", lat: 51.51, lon: -0.13 },    // 2
  { name: "PAR", lat: 48.86, lon: 2.35 },     // 3
  { name: "BER", lat: 52.52, lon: 13.41 },    // 4
  { name: "TYO", lat: 35.68, lon: 139.65 },   // 5
  { name: "SGP", lat: 1.35, lon: 103.82 },    // 6
  { name: "HKG", lat: 22.32, lon: 114.17 },   // 7
  { name: "SYD", lat: -33.87, lon: 151.21 },  // 8
  { name: "MSK", lat: 55.76, lon: 37.62 },    // 9
  { name: "DXB", lat: 25.20, lon: 55.27 },    // 10
  { name: "BOM", lat: 19.08, lon: 72.88 },    // 11
  { name: "SAO", lat: -23.55, lon: -46.63 },  // 12
  { name: "BUE", lat: -34.60, lon: -58.38 },  // 13
  { name: "DEL", lat: 28.61, lon: 77.21 },    // 14
  { name: "SEA", lat: 47.61, lon: -122.33 },  // 15
  { name: "TOR", lat: 43.65, lon: -79.38 },   // 16
  { name: "STO", lat: 59.33, lon: 18.07 },    // 17
  { name: "LAX", lat: 34.05, lon: -118.24 },  // 18
  { name: "JHB", lat: -26.20, lon: 28.04 },   // 19
  { name: "CHI", lat: 41.88, lon: -87.63 },   // 20
  { name: "MIA", lat: 25.76, lon: -80.19 },   // 21
  { name: "LIS", lat: 38.72, lon: -9.14 },    // 22
  { name: "OSL", lat: 59.91, lon: 10.75 },    // 23
  { name: "HEL", lat: 60.17, lon: 24.94 },    // 24
  { name: "WAR", lat: 52.23, lon: 21.01 },    // 25
  { name: "IST", lat: 41.01, lon: 28.98 },    // 26
  { name: "CAI", lat: 30.04, lon: 31.24 },    // 27
  { name: "NBO", lat: -1.29, lon: 36.82 },    // 28
  { name: "LOS", lat: 6.52, lon: 3.38 },      // 29
  { name: "BKK", lat: 13.76, lon: 100.50 },   // 30
  { name: "SEL", lat: 37.57, lon: 126.98 },   // 31
  { name: "MEX", lat: 19.43, lon: -99.13 },   // 32
  { name: "LIM", lat: -12.05, lon: -77.04 },  // 33
  { name: "SCL", lat: -33.45, lon: -70.67 },  // 34
  { name: "AKL", lat: -36.85, lon: 174.76 },  // 35
  { name: "PER", lat: -31.95, lon: 115.86 },  // 36
  { name: "MNL", lat: 14.60, lon: 120.98 },   // 37
  { name: "DEN", lat: 39.74, lon: -104.99 },  // 38
  { name: "ATL", lat: 33.75, lon: -84.39 },   // 39
]

const CONNECTIONS: [number, number][] = [
  // ── North America mesh ──
  [0, 1],   [0, 15],  [0, 18],  [0, 38],
  [1, 16],  [1, 20],  [1, 21],  [1, 39],
  [15, 16], [18, 38], [18, 39], [20, 38],
  [20, 39], [21, 32], [32, 33], [32, 18],
  // ── Transatlantic ──
  [1, 2],   [1, 3],   [22, 21], [16, 2],
  // ── Europe mesh ──
  [2, 3],   [2, 4],   [2, 17],  [2, 22],
  [3, 4],   [3, 22],  [4, 25],  [4, 17],
  [23, 24], [23, 17], [24, 25], [25, 26],
  [22, 29], [17, 24],
  // ── Europe → East ──
  [26, 10], [26, 27], [4, 9],   [9, 24],
  [9, 25],
  // ── Africa & Middle East ──
  [10, 27], [10, 28], [27, 28], [28, 19],
  [29, 28], [29, 19], [19, 27],
  // ── South & Central Asia ──
  [10, 11], [10, 14], [11, 14], [14, 30],
  [11, 30], [11, 6],  [14, 9],
  // ── East & Southeast Asia mesh ──
  [30, 6],  [30, 7],  [30, 37], [5, 31],
  [5, 7],   [5, 37],  [6, 37],  [6, 7],
  [7, 31],  [31, 37], [7, 37],
  // ── South America mesh ──
  [21, 12], [12, 13], [12, 34], [13, 34],
  [33, 34], [33, 12],
  // ── Oceania ──
  [8, 35],  [8, 36],  [35, 36], [36, 6],
  [8, 6],   [35, 5],
  // ── Long-haul cross-globe links ──
  [0, 5],   [31, 0],  [0, 8],   [1, 5],
  [2, 11],  [5, 18],  [12, 29], [19, 11],
  [6, 8],   [39, 12], [21, 29], [14, 26],
  [32, 12], [9, 5],   [18, 5],  [3, 10],
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

function createArcCurve(
  from: THREE.Vector3,
  to: THREE.Vector3,
): THREE.CubicBezierCurve3 {
  const dist = from.distanceTo(to)
  // Height scales with distance — short hops stay low, long hauls lift moderately
  const arcHeight = GLOBE_R * (0.08 + dist * 0.18)

  // Midpoint direction — the arc's apex in "space"
  const mid = new THREE.Vector3().addVectors(from, to).multiplyScalar(0.5)
  mid.normalize().multiplyScalar(GLOBE_R + arcHeight)

  // Two control points: one near source lifting off, one near dest coming down
  // This creates a tall arch — launch up, cruise through space, descend
  const cp1 = new THREE.Vector3().lerpVectors(from, mid, 0.4)
  cp1.normalize().multiplyScalar(GLOBE_R + arcHeight * 0.85)

  const cp2 = new THREE.Vector3().lerpVectors(to, mid, 0.4)
  cp2.normalize().multiplyScalar(GLOBE_R + arcHeight * 0.85)

  return new THREE.CubicBezierCurve3(from, cp1, cp2, to)
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
 * DICE logo markers at each node location — the brand's isometric cube
 * with 2×2 face subdivisions and the signature protruding sub-cube,
 * built as real 3D geometry. Each marker is oriented outward from the
 * globe surface, representing a DICE hardware node deployed at that city.
 */
function DiceMarkers() {
  const S = 0.026 // main cube full size
  const H = S / 2 // half-extent

  const diceData = useMemo(() => {
    const up = new THREE.Vector3(0, 1, 0)
    return NODES.map((n) => {
      const pos = latLonToVec3(n.lat, n.lon, GLOBE_R * 1.012)
      const lookTarget = new THREE.Vector3(0, 0, 0)
      const mat4 = new THREE.Matrix4().lookAt(pos, lookTarget, up)
      const quat = new THREE.Quaternion().setFromRotationMatrix(mat4)
      return { pos, quat }
    })
  }, [])

  const edgeGeo = useMemo(
    () => new THREE.EdgesGeometry(new THREE.BoxGeometry(S, S, S)),
    [S],
  )

  // Face subdivision lines — 2×2 grid on all 6 faces
  const subdivGeo = useMemo(() => {
    const pts: number[] = []
    const push = (ax: number, ay: number, az: number, bx: number, by: number, bz: number) => {
      pts.push(ax, ay, az, bx, by, bz)
    }
    // Top & bottom face midlines
    push(-H, H, 0, H, H, 0); push(0, H, -H, 0, H, H)
    push(-H, -H, 0, H, -H, 0); push(0, -H, -H, 0, -H, H)
    // Front & back face midlines
    push(-H, 0, H, H, 0, H); push(0, -H, H, 0, H, H)
    push(-H, 0, -H, H, 0, -H); push(0, -H, -H, 0, H, -H)
    // Left & right face midlines
    push(-H, 0, -H, -H, 0, H); push(-H, -H, 0, -H, H, 0)
    push(H, 0, -H, H, 0, H); push(H, -H, 0, H, H, 0)

    const geo = new THREE.BufferGeometry()
    geo.setAttribute("position", new THREE.Float32BufferAttribute(pts, 3))
    return geo
  }, [H])

  const fillMat = useMemo(
    () => new THREE.MeshBasicMaterial({ color: "#ffffff", transparent: true, opacity: 0.06, depthTest: true }),
    [],
  )
  const edgeMat = useMemo(
    () => new THREE.LineBasicMaterial({ color: "#ffffff", transparent: true, opacity: 0.8 }),
    [],
  )
  const subdivMat = useMemo(
    () => new THREE.LineBasicMaterial({ color: "#ffffff", transparent: true, opacity: 0.35 }),
    [],
  )

  return (
    <group renderOrder={1}>
      {diceData.map(({ pos, quat }, i) => (
        <group key={i} position={pos} quaternion={quat}>
          <mesh material={fillMat} renderOrder={1}>
            <boxGeometry args={[S, S, S]} />
          </mesh>
          <lineSegments geometry={edgeGeo} material={edgeMat} renderOrder={1} />
          <lineSegments geometry={subdivGeo} material={subdivMat} renderOrder={1} />
        </group>
      ))}
    </group>
  )
}

const TRAIL_LENGTH = 1.1
const TRAIL_SEGS = 80
const TRAIL_RADIAL = 8
const HEAD_RADIUS = 0.008
const TAIL_RADIUS = 0.0008

/**
 * Build a tapered tube along a curve — thick at t=1 (head), thin at t=0 (tail).
 * Stores a `aT` attribute (0–1 along tube) for the shader.
 */
function buildTaperedTube(curve: THREE.CubicBezierCurve3) {
  const verts: number[] = []
  const normals: number[] = []
  const uvs: number[] = []
  const tValues: number[] = []
  const indices: number[] = []

  const frames = curve.computeFrenetFrames(TRAIL_SEGS, false)

  for (let i = 0; i <= TRAIL_SEGS; i++) {
    const t = i / TRAIL_SEGS
    const pt = curve.getPoint(t)
    // Smooth taper: thick at head (t=1), needle-thin at tail (t=0)
    const taper = t * t // quadratic taper
    const radius = TAIL_RADIUS + (HEAD_RADIUS - TAIL_RADIUS) * taper

    const N = frames.normals[i]
    const B = frames.binormals[i]

    for (let j = 0; j <= TRAIL_RADIAL; j++) {
      const angle = (j / TRAIL_RADIAL) * Math.PI * 2
      const sin = Math.sin(angle)
      const cos = Math.cos(angle)

      const nx = cos * N.x + sin * B.x
      const ny = cos * N.y + sin * B.y
      const nz = cos * N.z + sin * B.z

      verts.push(pt.x + radius * nx, pt.y + radius * ny, pt.z + radius * nz)
      normals.push(nx, ny, nz)
      uvs.push(t, j / TRAIL_RADIAL)
      tValues.push(t)
    }
  }

  for (let i = 0; i < TRAIL_SEGS; i++) {
    for (let j = 0; j < TRAIL_RADIAL; j++) {
      const a = i * (TRAIL_RADIAL + 1) + j
      const b = a + 1
      const c = a + TRAIL_RADIAL + 1
      const d = c + 1
      indices.push(a, c, b, b, c, d)
    }
  }

  const geo = new THREE.BufferGeometry()
  geo.setAttribute("position", new THREE.Float32BufferAttribute(verts, 3))
  geo.setAttribute("normal", new THREE.Float32BufferAttribute(normals, 3))
  geo.setAttribute("uv", new THREE.Float32BufferAttribute(uvs, 2))
  geo.setAttribute("aT", new THREE.Float32BufferAttribute(tValues, 1))
  geo.setIndex(indices)
  return geo
}

const TRAIL_VERTEX = /* glsl */ `
  attribute float aT;
  varying float vT;
  void main() {
    vT = aT;
    gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
  }
`

const TRAIL_FRAGMENT = /* glsl */ `
  uniform float uHead;
  uniform float uTailLen;
  uniform vec3 uColorHead;
  uniform vec3 uColorTail;
  varying float vT;

  void main() {
    // Distance behind the head, wrapping around 0↔1
    float d = uHead - vT;
    if (d < 0.0) d += 1.0;

    // Outside the trail → invisible
    if (d > uTailLen) discard;

    // Fade: 1.0 at head → 0.0 at tail
    float fade = 1.0 - (d / uTailLen);
    // Smooth hermite for organic falloff
    fade = fade * fade * (3.0 - 2.0 * fade);

    // White head → Solana purple tail
    vec3 color = mix(uColorTail, uColorHead, fade);

    // Softer glow — solid-ish at head, smoothly vanishes
    float alpha = fade * 0.55;

    gl_FragColor = vec4(color, alpha);
  }
`

function CometTrails({ curves }: { curves: THREE.CubicBezierCurve3[] }) {
  const trailData = useMemo(() => {
    return curves.map((curve) => {
      const geo = buildTaperedTube(curve)

      const mat = new THREE.ShaderMaterial({
        vertexShader: TRAIL_VERTEX,
        fragmentShader: TRAIL_FRAGMENT,
        transparent: true,
        depthWrite: false,
        depthTest: true,
        side: THREE.DoubleSide,
        uniforms: {
          uHead: { value: Math.random() },
          uTailLen: { value: TRAIL_LENGTH },
          uColorHead: { value: new THREE.Color("#ffffff") },
          uColorTail: { value: new THREE.Color("#9945FF") },
        },
      })

      return { geo, mat, speed: 0.06 + Math.random() * 0.12 }
    })
  }, [curves])

  useFrame((_, delta) => {
    trailData.forEach((trail) => {
      const u = trail.mat.uniforms.uHead
      u.value = (u.value + trail.speed * delta) % 1
    })
  })

  return (
    <group renderOrder={2}>
      {trailData.map(({ geo, mat }, i) => (
        <mesh key={i} geometry={geo} material={mat} renderOrder={2} />
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

function GlobeScene({
  dotData,
}: {
  dotData: { positions: Float32Array; landFlags: Float32Array }
}) {
  const groupRef = useRef<THREE.Group>(null)

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
      <SphereDots
        positions={dotData.positions}
        landFlags={dotData.landFlags}
      />

      {/* 2. Depth-only sphere — invisible, creates Z-barrier to hide
             backside arcs / markers / packets */}
      <DepthSphere />

      {/* 3. Network visualization — depth-tested against the barrier */}
      <DiceMarkers />
      <CometTrails curves={arcCurves} />

      {/* 4. Glass overlay — Apple-style sheen on the globe surface */}
      <GlassOverlay />

    </group>
  )
}

// ─── Exported Component ──────────────────────────────────────

export function NetworkGlobe({ className }: { className?: string }) {
  const [mounted, setMounted] = useState(false)
  const { dotData } = useSphereDots()
  const { setGlobeReady } = useGlobeReady()
  const hasSignaled = useRef(false)

  useEffect(() => setMounted(true), [])

  const handleCreated = useCallback(
    ({ gl }: { gl: THREE.WebGLRenderer }) => {
      gl.setClearColor(0x000000, 0)
      // Wait one frame for shaders to compile, then signal ready
      requestAnimationFrame(() => {
        if (!hasSignaled.current) {
          hasSignaled.current = true
          setGlobeReady()
        }
      })
    },
    [setGlobeReady],
  )

  if (!mounted || !dotData) return <div className={className} />

  return (
    <div className={className}>
      <Canvas
        camera={{ position: [0, 0, 3.0], fov: 45 }}
        gl={{ antialias: true, alpha: true }}
        onCreated={handleCreated}
        style={{ background: "none" }}
        dpr={[1, 2]}
      >
        <GlobeScene dotData={dotData} />
      </Canvas>
    </div>
  )
}

export default NetworkGlobe
