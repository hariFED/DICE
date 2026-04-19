"use client"

import { motion, useScroll, useTransform } from "framer-motion"
import { useRef } from "react"

/**
 * ESP32 exploded isometric blueprint — Dan-Hollick / FIG_001 style.
 *
 * Three layers stacked vertically in dimetric (2:1) projection:
 *   top   : RF cap / WiFi antenna trace
 *   mid   : PCB with ESP32-S3 + components
 *   bottom: shielded base plate + USB-C cutaway
 *
 * As the viewport scrolls through the section the layers separate
 * further (explode). Labels with leader lines annotate each component.
 *
 * Projection: iso(x, y, z) = [ x - y, (x + y)/2 - z ]
 *   — standard 30° dimetric; x-right, y-back-into-page, z-up.
 */

// Dimetric helper: world (x,y,z) → screen (sx,sy)
function iso(x: number, y: number, z: number): [number, number] {
  return [x - y, (x + y) * 0.5 - z]
}

// Draw a horizontal slab of dims (w x d) at height z, offset ox,oy in screen.
function slab(ox: number, oy: number, w: number, d: number, t: number, z: number) {
  const [a0, a1] = iso(0, 0, z)
  const [b0, b1] = iso(w, 0, z)
  const [c0, c1] = iso(w, d, z)
  const [d0, d1] = iso(0, d, z)
  // bottom corners (z - t)
  const [ab0, ab1] = iso(0, 0, z - t)
  const [bb0, bb1] = iso(w, 0, z - t)
  const [cb0, cb1] = iso(w, d, z - t)
  const top = `${a0 + ox},${a1 + oy} ${b0 + ox},${b1 + oy} ${c0 + ox},${c1 + oy} ${d0 + ox},${d1 + oy}`
  const right = `${b0 + ox},${b1 + oy} ${c0 + ox},${c1 + oy} ${cb0 + ox},${cb1 + oy} ${bb0 + ox},${bb1 + oy}`
  const front = `${a0 + ox},${a1 + oy} ${b0 + ox},${b1 + oy} ${bb0 + ox},${bb1 + oy} ${ab0 + ox},${ab1 + oy}`
  return { top, right, front }
}

// Small flat rect on top of a slab at (wx, wy, wz) with w x d extent.
function chip(ox: number, oy: number, wx: number, wy: number, w: number, d: number, z: number) {
  const [a0, a1] = iso(wx, wy, z)
  const [b0, b1] = iso(wx + w, wy, z)
  const [c0, c1] = iso(wx + w, wy + d, z)
  const [d0, d1] = iso(wx, wy + d, z)
  return `${a0 + ox},${a1 + oy} ${b0 + ox},${b1 + oy} ${c0 + ox},${c1 + oy} ${d0 + ox},${d1 + oy}`
}

// Screen point from world coords + offset.
function sp(ox: number, oy: number, x: number, y: number, z: number): [number, number] {
  const [sx, sy] = iso(x, y, z)
  return [sx + ox, sy + oy]
}

export function Esp32Exploded() {
  const sectionRef = useRef<HTMLDivElement>(null)
  const { scrollYProgress } = useScroll({
    target: sectionRef,
    offset: ["start end", "end start"],
  })

  // Explosion factor: 0 → compact, 1 → fully separated.
  // Used below by the middle + top layers to animate their y-offset.
  const explode = useTransform(scrollYProgress, [0, 0.4, 0.7, 1], [0.1, 0.7, 1, 1])
  const midY = useTransform(explode, [0, 1], [0, -90])
  const topY = useTransform(explode, [0, 1], [0, -180])

  // Board dims (world units)
  const W = 240, D = 120, T = 6
  const OX = 500  // origin x offset in viewBox
  const OY = 340  // origin y offset in viewBox

  return (
    <section
      ref={sectionRef}
      className="relative border-b border-border overflow-hidden"
    >
      <div className="relative mx-auto w-full max-w-[1400px] px-4 sm:px-6 lg:px-8 py-20 md:py-32">
        {/* Section header */}
        <div className="grid grid-cols-1 lg:grid-cols-12 gap-8 mb-12">
          <div className="lg:col-span-3">
            <span className="chapter-num">03</span>
            <p className="mt-2 font-mono text-[11px] uppercase tracking-[0.2em] text-muted-foreground">
              entropy · node
            </p>
          </div>
          <div className="lg:col-span-6 lg:col-start-5">
            <h2 className="font-sans text-3xl md:text-4xl lg:text-5xl tracking-[-0.02em] leading-[1.05] text-foreground">
              One board. <span className="italic font-light text-muted-foreground">Six silicon primitives.</span>
            </h2>
            <p className="mt-5 font-mono text-sm text-muted-foreground max-w-md leading-relaxed">
              The DICE-S3 node is built from a single ESP32-S3 module with
              a hardware TRNG, WiFi radio, and secp256k1 signing — nothing
              more. Each unit is tamper-evident and field-provisioned.
            </p>
          </div>
        </div>

        {/* Exploded blueprint */}
        <div className="relative bg-grid border border-border overflow-hidden">
          {/* corner ticks */}
          <span className="absolute top-3 left-3 font-mono text-[10px] uppercase tracking-wider text-muted-foreground/50">FIG_003 · DICE-S3 · exploded</span>
          <span className="absolute top-3 right-3 font-mono text-[10px] uppercase tracking-wider text-muted-foreground/50">Scale 1:1 · dimetric 2:1</span>
          <span className="absolute bottom-3 left-3 font-mono text-[10px] uppercase tracking-wider text-muted-foreground/50">REV_C · 2026</span>
          <span className="absolute bottom-3 right-3 font-mono text-[10px] uppercase tracking-wider text-muted-foreground/50">[ scroll to separate ]</span>

          <motion.svg
            viewBox="0 0 1000 800"
            className="w-full h-auto"
            style={{
              color: "var(--foreground)",
            }}
          >
            <defs>
              <pattern id="hatch" patternUnits="userSpaceOnUse" width="6" height="6" patternTransform="rotate(45)">
                <line x1="0" y1="0" x2="0" y2="6" stroke="currentColor" strokeWidth="0.5" opacity="0.25" />
              </pattern>
            </defs>

            {/* Connector/explosion guide lines — faint dashes through all 3 layers */}
            <AxisGuides OX={OX} OY={OY} W={W} D={D} />

            {/* BOTTOM LAYER — shielded base plate */}
            <ExplodedLayer
              OX={OX}
              OY={OY}
              offset={0}
              W={W}
              D={D}
              T={T}
              content={(ox, oy, z) => (
                <>
                  {/* USB-C cutaway on left edge */}
                  <polygon
                    points={chip(ox, oy, -14, 40, 14, 40, z + T)}
                    fill="currentColor"
                    opacity="0.12"
                  />
                  <polygon
                    points={chip(ox, oy, -14, 40, 14, 40, z + T)}
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="1"
                  />
                  {/* Hatched shield region */}
                  <polygon
                    points={chip(ox, oy, 20, 20, W - 40, D - 40, z + T + 0.1)}
                    fill="url(#hatch)"
                  />
                </>
              )}
            />

            {/* MIDDLE LAYER — PCB */}
            <motion.g style={{ y: midY }}>
              <ExplodedLayer
                OX={OX}
                OY={OY}
                offset={-90}
                W={W}
                D={D}
                T={T}
                content={(ox, oy, z) => (
                  <>
                    {/* ESP32-S3 QFN — center */}
                    <polygon
                      points={chip(ox, oy, W / 2 - 30, D / 2 - 22, 60, 44, z + T)}
                      fill="currentColor"
                      opacity="0.8"
                    />
                    <polygon
                      points={chip(ox, oy, W / 2 - 30, D / 2 - 22, 60, 44, z + T + 1)}
                      fill="none"
                      stroke="currentColor"
                      strokeWidth="0.8"
                    />
                    {/* Flash chip */}
                    <polygon
                      points={chip(ox, oy, W / 2 + 50, D / 2 - 8, 22, 16, z + T)}
                      fill="currentColor"
                      opacity="0.5"
                    />
                    {/* Crystal oscillator */}
                    <polygon
                      points={chip(ox, oy, W / 2 - 58, D / 2 + 16, 20, 8, z + T)}
                      fill="currentColor"
                      opacity="0.35"
                    />
                    {/* Passives row */}
                    {[0, 1, 2, 3, 4, 5].map((i) => (
                      <polygon
                        key={i}
                        points={chip(ox, oy, 32 + i * 10, D - 28, 6, 3, z + T)}
                        fill="currentColor"
                        opacity="0.4"
                      />
                    ))}
                    {/* 4 status LEDs — tiny squares on bottom-right */}
                    {[0, 1, 2, 3].map((i) => (
                      <polygon
                        key={i}
                        points={chip(ox, oy, W - 50 + i * 9, 14, 4, 4, z + T)}
                        fill="currentColor"
                        opacity="0.7"
                      />
                    ))}
                    {/* BOOT + RST buttons */}
                    <polygon
                      points={chip(ox, oy, 14, 14, 10, 10, z + T)}
                      fill="currentColor"
                      opacity="0.6"
                    />
                    <polygon
                      points={chip(ox, oy, 30, 14, 10, 10, z + T)}
                      fill="currentColor"
                      opacity="0.6"
                    />
                    {/* ESP32-S3 label on chip */}
                    <text
                      x={sp(ox, oy, W / 2, D / 2, z + T + 1)[0]}
                      y={sp(ox, oy, W / 2, D / 2, z + T + 1)[1] + 4}
                      textAnchor="middle"
                      fontFamily="var(--font-mono)"
                      fontSize="7"
                      fill="var(--background)"
                      opacity="0.85"
                    >
                      ESP32-S3
                    </text>
                  </>
                )}
              />
            </motion.g>

            {/* TOP LAYER — RF cap / antenna silk */}
            <motion.g style={{ y: topY }}>
              <ExplodedLayer
                OX={OX}
                OY={OY}
                offset={-180}
                W={W}
                D={D}
                T={T * 0.6}
                content={(ox, oy, z) => (
                  <>
                    {/* Antenna trace — zigzag line on the top cap */}
                    <polyline
                      points={[
                        sp(ox, oy, W - 60, 20, z + T + 0.5),
                        sp(ox, oy, W - 48, 32, z + T + 0.5),
                        sp(ox, oy, W - 36, 20, z + T + 0.5),
                        sp(ox, oy, W - 24, 32, z + T + 0.5),
                        sp(ox, oy, W - 12, 20, z + T + 0.5),
                      ]
                        .map((p) => p.join(","))
                        .join(" ")}
                      fill="none"
                      stroke="currentColor"
                      strokeWidth="1.2"
                    />
                    {/* Perforation holes */}
                    {[0.25, 0.5, 0.75].map((f) => {
                      const [cx, cy] = sp(ox, oy, W * f, D / 2, z + T + 1)
                      return <circle key={f} cx={cx} cy={cy} r="2" fill="none" stroke="currentColor" strokeWidth="0.6" />
                    })}
                  </>
                )}
              />
            </motion.g>

            {/* LABELS — drawn last so they sit on top */}
            <Label
              anchorWorld={{ ox: OX, oy: OY, x: W - 36, y: 26, z: -180 + T + 4 }}
              labelXY={[880, 140]}
              num="01"
              title="WiFi ANT"
              sub="2.4 GHz trace"
            />
            <Label
              anchorWorld={{ ox: OX, oy: OY, x: W / 2, y: D / 2, z: -90 + T + 4 }}
              labelXY={[880, 320]}
              num="02"
              title="ESP32-S3"
              sub="TRNG · secp256k1"
            />
            <Label
              anchorWorld={{ ox: OX, oy: OY, x: -10, y: 60, z: 0 + T }}
              labelXY={[100, 520]}
              num="03"
              title="USB-C"
              sub="power · flash"
            />
            <Label
              anchorWorld={{ ox: OX, oy: OY, x: W - 45, y: 16, z: -90 + T + 4 }}
              labelXY={[900, 500]}
              num="04"
              title="Status LEDs"
              sub="pwr · net · ent · stat"
            />
            <Label
              anchorWorld={{ ox: OX, oy: OY, x: W / 2 - 48, y: D / 2 + 20, z: -90 + T + 4 }}
              labelXY={[120, 380]}
              num="05"
              title="XTAL"
              sub="26 MHz · PLL src"
            />
            <Label
              anchorWorld={{ ox: OX, oy: OY, x: 22, y: 19, z: -90 + T + 4 }}
              labelXY={[100, 240]}
              num="06"
              title="BOOT · RST"
              sub="provisioning"
            />
          </motion.svg>
        </div>

        {/* Below the blueprint — 3 text callouts summarizing the structure */}
        <div className="mt-10 grid grid-cols-1 md:grid-cols-3 gap-6 font-mono text-[13px] text-muted-foreground">
          <div>
            <p className="font-pixel text-foreground text-lg mb-1">STACK.01</p>
            <p>RF cap. Shields the antenna trace from mechanical damage. Perforated for airflow.</p>
          </div>
          <div>
            <p className="font-pixel text-foreground text-lg mb-1">STACK.02</p>
            <p>Main PCB. ESP32-S3 with in-chip hardware RNG, signing, WiFi. Six passives.</p>
          </div>
          <div>
            <p className="font-pixel text-foreground text-lg mb-1">STACK.03</p>
            <p>Base plate. USB-C cutout, shield ground, mounting posts for enclosure.</p>
          </div>
        </div>
      </div>
    </section>
  )
}

// ── internal components ─────────────────────────────────────────────────────

function ExplodedLayer({
  OX,
  OY,
  offset: _offset,
  W,
  D,
  T,
  content,
}: {
  OX: number
  OY: number
  offset: number
  W: number
  D: number
  T: number
  content: (ox: number, oy: number, z: number) => React.ReactNode
}) {
  const s = slab(OX, OY, W, D, T, 0)
  return (
    <g>
      {/* Front + right side faces */}
      <polygon points={s.front} fill="var(--background)" stroke="currentColor" strokeWidth="1" />
      <polygon points={s.right} fill="var(--background)" stroke="currentColor" strokeWidth="1" />
      {/* Top face */}
      <polygon points={s.top} fill="var(--background)" stroke="currentColor" strokeWidth="1.2" />
      {content(OX, OY, 0)}
    </g>
  )
}

function AxisGuides({
  OX,
  OY,
  W,
  D,
}: {
  OX: number
  OY: number
  W: number
  D: number
}) {
  const pts: [number, number, number][] = [
    [0, 0, 0],
    [W, 0, 0],
    [W, D, 0],
    [0, D, 0],
  ]
  return (
    <g stroke="currentColor" strokeWidth="0.6" strokeDasharray="2 3" opacity="0.35" fill="none">
      {pts.map((p, i) => {
        const [sx, sy] = iso(p[0], p[1], p[2])
        return (
          <line
            key={i}
            x1={sx + OX}
            y1={sy + OY - 220}
            x2={sx + OX}
            y2={sy + OY + 10}
          />
        )
      })}
    </g>
  )
}

function Label({
  anchorWorld,
  labelXY,
  num,
  title,
  sub,
}: {
  anchorWorld: { ox: number; oy: number; x: number; y: number; z: number }
  labelXY: [number, number]
  num: string
  title: string
  sub: string
}) {
  const [ax, ay] = sp(anchorWorld.ox, anchorWorld.oy, anchorWorld.x, anchorWorld.y, anchorWorld.z)
  const [lx, ly] = labelXY
  // Kinked leader: anchor → mid → label
  const midX = lx > ax ? lx - 18 : lx + 18
  const midY = ay
  return (
    <g>
      <circle cx={ax} cy={ay} r="2.5" fill="currentColor" />
      <polyline
        points={`${ax},${ay} ${midX},${midY} ${lx},${ly}`}
        fill="none"
        stroke="currentColor"
        strokeWidth="0.8"
        strokeDasharray="3 2"
        opacity="0.7"
      />
      <text
        x={lx + (lx > 500 ? 6 : -6)}
        y={ly - 4}
        fontFamily="var(--font-pixel)"
        fontSize="11"
        fill="currentColor"
        textAnchor={lx > 500 ? "start" : "end"}
      >
        {num} · {title}
      </text>
      <text
        x={lx + (lx > 500 ? 6 : -6)}
        y={ly + 8}
        fontFamily="var(--font-mono)"
        fontSize="9"
        fill="currentColor"
        opacity="0.6"
        textAnchor={lx > 500 ? "start" : "end"}
      >
        {sub}
      </text>
    </g>
  )
}

export default Esp32Exploded
