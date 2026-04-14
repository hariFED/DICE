"use client"

import { useRef, useState } from "react"
import * as THREE from "three"
import { Canvas, useFrame, useThree } from "@react-three/fiber"
import { Environment, ContactShadows } from "@react-three/drei"
import { useScroll, useTransform, useMotionValueEvent } from "framer-motion"
import { cn } from "@/lib/utils"
import { ESP32Model } from "./ESP32Model"

/* ── Scroll phase annotations ── */
const ANNOTATIONS = [
  {
    progress: [0, 0.15] as const,
    text: "Meet the DICE Node",
    sub: "ESP32-S3 Development Board",
  },
  {
    progress: [0.15, 0.3] as const,
    text: "USB-C Power",
    sub: "Plug in, always on. 5 V, 500 mA.",
  },
  {
    progress: [0.3, 0.45] as const,
    text: "Status LEDs",
    sub: "Green: online. Blue: processing entropy.",
  },
  {
    progress: [0.45, 0.6] as const,
    text: "ESP32-S3 SoC",
    sub: "Hardware TRNG + ADC noise. True entropy.",
  },
  {
    progress: [0.6, 0.75] as const,
    text: "WiFi Antenna",
    sub: "Connects to coordinator via mTLS.",
  },
  {
    progress: [0.75, 1] as const,
    text: "What's Inside",
    sub: "Every component working together.",
  },
]

/* ── Camera that lerps to match scroll-driven zoom ── */
function CameraController({ zoom }: { zoom: number }) {
  const { camera } = useThree()
  const target = useRef(new THREE.Vector3(0, 2, 4))

  useFrame(() => {
    target.current.set(0, 2, zoom)
    camera.position.lerp(target.current, 0.05)
    camera.lookAt(0, 0, 0)
  })

  return null
}

/* ── Exported scene with scroll-driven transforms ── */
export function ESP32Scene() {
  const containerRef = useRef<HTMLDivElement>(null)

  const { scrollYProgress } = useScroll({
    target: containerRef,
    offset: ["start start", "end end"],
  })

  /* Map scroll progress to rotation / explode / zoom */
  const rotationY = useTransform(
    scrollYProgress,
    [0, 0.15, 0.3, 0.45, 0.6, 0.75, 1],
    [0, Math.PI * 0.5, Math.PI, Math.PI * 1.2, Math.PI * 1.5, Math.PI * 1.8, Math.PI * 2],
  )
  const rotationX = useTransform(
    scrollYProgress,
    [0, 0.15, 0.3, 0.45, 0.6, 0.75, 1],
    [0.3, 0.2, 0.4, 0.6, 0.3, 0.2, 0.5],
  )
  const explode = useTransform(scrollYProgress, [0.7, 1], [0, 1])
  const zoom = useTransform(
    scrollYProgress,
    [0, 0.15, 0.45, 0.6, 0.75, 1],
    [4, 3.5, 2.5, 2.5, 3, 4.5],
  )

  /* Reactive state piped into the Canvas tree */
  const [currentAnnotation, setCurrentAnnotation] = useState(0)
  const [rotY, setRotY] = useState(0)
  const [rotX, setRotX] = useState(0.3)
  const [exp, setExp] = useState(0)
  const [zm, setZm] = useState(4)

  useMotionValueEvent(scrollYProgress, "change", (v) => {
    const idx = ANNOTATIONS.findIndex(
      (a) => v >= a.progress[0] && v < a.progress[1],
    )
    if (idx !== -1) setCurrentAnnotation(idx)
  })
  useMotionValueEvent(rotationY, "change", setRotY)
  useMotionValueEvent(rotationX, "change", setRotX)
  useMotionValueEvent(explode, "change", setExp)
  useMotionValueEvent(zoom, "change", setZm)

  return (
    <div ref={containerRef} className="relative" style={{ height: "400vh" }}>
      {/* Sticky viewport that stays while the user scrolls through 400 vh */}
      <div className="sticky top-0 h-screen flex items-center justify-center overflow-hidden">
        {/* ── Annotation overlay (left side) ── */}
        <div className="absolute z-10 left-8 md:left-16 top-1/2 -translate-y-1/2 max-w-sm pointer-events-none">
          <p className="text-xs uppercase tracking-[0.2em] text-[#00FF85] mb-2">
            The DICE Node
          </p>
          <h3
            key={currentAnnotation}
            className="text-3xl md:text-4xl font-bold text-white transition-all duration-500"
          >
            {ANNOTATIONS[currentAnnotation]?.text}
          </h3>
          <p className="mt-2 text-zinc-400 transition-all duration-500">
            {ANNOTATIONS[currentAnnotation]?.sub}
          </p>

          {/* Progress dots */}
          <div className="mt-8 flex gap-1.5">
            {ANNOTATIONS.map((_, i) => (
              <div
                key={i}
                className={cn(
                  "h-1 rounded-full transition-all duration-300",
                  i === currentAnnotation
                    ? "w-8 bg-[#00FF85]"
                    : "w-2 bg-zinc-700",
                )}
              />
            ))}
          </div>
        </div>

        {/* ── 3D Canvas ── */}
        <div className="w-full h-full">
          <Canvas camera={{ position: [0, 2, 4], fov: 35 }}>
            <CameraController zoom={zm} />

            <ambientLight intensity={0.3} />
            <directionalLight position={[5, 5, 5]} intensity={1} />
            <directionalLight
              position={[-3, 3, -3]}
              intensity={0.5}
              color="#00FF85"
            />

            <group rotation={[rotX, rotY, 0]}>
              <ESP32Model explode={exp} />
            </group>

            <ContactShadows
              position={[0, -0.5, 0]}
              opacity={0.4}
              scale={5}
              blur={2.5}
              far={4}
              color="#00FF85"
            />
            <Environment preset="city" environmentIntensity={0.5} />
          </Canvas>
        </div>
      </div>
    </div>
  )
}

export default ESP32Scene
