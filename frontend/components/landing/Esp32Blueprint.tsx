"use client"

import { motion } from "framer-motion"

/**
 * Isometric blueprint-style ESP32-S3 board.
 *
 * Drawn as a single SVG with:
 *   • a parallelogram PCB (isometric rotation ≈ 30°)
 *   • rectangular silkscreen components sitting on top
 *   • label callouts connected by dashed leader lines
 *
 * Takes an `activeKey` prop — the matching label gets `text-foreground` +
 * a bold leader line; others dim to `muted-foreground`.
 *
 * All coords are in the viewBox (1000×700) so any parent sizing works.
 */

export type Esp32Key =
  | "usb"
  | "soc"
  | "ant"
  | "leds"
  | "passives"
  | "buttons"

interface Props {
  activeKey?: Esp32Key
  className?: string
}

/* Isometric projection helpers.
 * 2:1 isometric: each "world" unit maps to (2, 1) in screen px after a
 * skew. We pre-compose: x_screen = x - y, y_screen = (x + y) * 0.5 + z.
 * z is the vertical lift for components sitting on top of the board. */
const iso = (x: number, y: number, z = 0): [number, number] => [
  x - y,
  (x + y) * 0.5 - z,
]

// Board dimensions in world-space
const BOARD_W = 600
const BOARD_H = 340

// Offset so nothing hangs outside the SVG
const OX = 500
const OY = 340

const i = (x: number, y: number, z = 0) => {
  const [sx, sy] = iso(x, y, z)
  return [OX + sx, OY + sy] as const
}

function Parallelogram({
  x, y, w, h, z = 6, fill, stroke, className,
}: {
  x: number; y: number; w: number; h: number; z?: number
  fill?: string; stroke?: string; className?: string
}) {
  const p = [
    i(x, y, z),
    i(x + w, y, z),
    i(x + w, y + h, z),
    i(x, y + h, z),
  ]
  return (
    <polygon
      points={p.map((pt) => pt.join(",")).join(" ")}
      fill={fill ?? "none"}
      stroke={stroke}
      strokeWidth={1.5}
      className={className}
    />
  )
}

/** Side face — connects the bottom-of-top rectangle to the ground rectangle */
function SideFace({
  x, y, w, h, z, className,
}: {
  x: number; y: number; w: number; h: number; z: number; className?: string
}) {
  // Front + right sides
  const topFront = i(x, y + h, z)
  const topRight = i(x + w, y + h, z)
  const topBR = i(x + w, y, z)
  const botFront = i(x, y + h, 0)
  const botRight = i(x + w, y + h, 0)
  const botBR = i(x + w, y, 0)
  return (
    <g className={className}>
      <polygon
        points={[topFront, topRight, botRight, botFront].map((p) => p.join(",")).join(" ")}
        fill="currentColor"
        fillOpacity={0.06}
        stroke="currentColor"
        strokeWidth={1}
      />
      <polygon
        points={[topRight, topBR, botBR, botRight].map((p) => p.join(",")).join(" ")}
        fill="currentColor"
        fillOpacity={0.10}
        stroke="currentColor"
        strokeWidth={1}
      />
    </g>
  )
}

function ComponentBlock({
  x, y, w, h, z = 6, active,
}: {
  x: number; y: number; w: number; h: number; z?: number; active?: boolean
}) {
  return (
    <g className={active ? "text-foreground" : "text-muted-foreground/60"}>
      <SideFace x={x} y={y} w={w} h={h} z={z} className="" />
      <Parallelogram
        x={x}
        y={y}
        w={w}
        h={h}
        z={z}
        stroke="currentColor"
        className={active ? "fill-foreground/[0.06]" : ""}
      />
    </g>
  )
}

/** Pin row along one side of an IC — evenly spaced ticks */
function PinRow({
  x, y, len, count, axis = "x", active,
}: {
  x: number; y: number; len: number; count: number; axis?: "x" | "y"; active?: boolean
}) {
  const step = len / (count + 1)
  return (
    <g className={active ? "text-foreground" : "text-muted-foreground/50"}>
      {Array.from({ length: count }).map((_, idx) => {
        const offset = step * (idx + 1)
        const [x1, y1] =
          axis === "x"
            ? i(x + offset, y, 3)
            : i(x, y + offset, 3)
        const [x2, y2] =
          axis === "x"
            ? i(x + offset, y, 0)
            : i(x, y + offset, 0)
        return (
          <line
            key={idx}
            x1={x1}
            y1={y1}
            x2={x2}
            y2={y2}
            stroke="currentColor"
            strokeWidth={1}
          />
        )
      })}
    </g>
  )
}

interface LabelLineProps {
  from: [number, number]       // world-space (x, y, z=component-top)
  to: [number, number]         // SVG absolute
  text: string
  sub?: string
  active: boolean
  align?: "start" | "end"
}

function LabelLine({ from, to, text, sub, active, align = "start" }: LabelLineProps) {
  const [fx, fy] = i(from[0], from[1], 6)
  const [tx, ty] = to
  return (
    <g className={active ? "text-foreground" : "text-muted-foreground/60"}>
      {/* Leader line — short horizontal kink so labels stack cleanly */}
      <polyline
        points={`${fx},${fy} ${tx},${fy} ${tx},${ty}`}
        fill="none"
        stroke="currentColor"
        strokeWidth={active ? 1.4 : 1}
        strokeDasharray={active ? "none" : "2 3"}
      />
      {/* Anchor dot on the component */}
      <circle cx={fx} cy={fy} r={active ? 3 : 2} fill="currentColor" />
      {/* Label text */}
      <text
        x={tx + (align === "start" ? 6 : -6)}
        y={ty + 4}
        textAnchor={align}
        fontSize={13}
        fontFamily="var(--font-mono), ui-monospace, monospace"
        className="uppercase tracking-wider"
        fill="currentColor"
        style={{ letterSpacing: "0.08em" }}
      >
        {text}
      </text>
      {sub && (
        <text
          x={tx + (align === "start" ? 6 : -6)}
          y={ty + 22}
          textAnchor={align}
          fontSize={11}
          fontFamily="var(--font-mono), ui-monospace, monospace"
          fill="currentColor"
          fillOpacity={0.6}
        >
          {sub}
        </text>
      )}
    </g>
  )
}

export function Esp32Blueprint({ activeKey, className = "" }: Props) {
  const isActive = (k: Esp32Key) => activeKey === k

  return (
    <svg
      viewBox="0 0 1200 800"
      className={`w-full h-auto ${className}`}
      role="img"
      aria-label="Isometric exploded diagram of the DICE-S3 entropy node"
    >
      {/* ── graph paper grid (very faint) ───────────────────────── */}
      <defs>
        <pattern id="grid" width="20" height="20" patternUnits="userSpaceOnUse">
          <circle cx="1" cy="1" r="0.6" className="fill-muted-foreground/20" />
        </pattern>
        <marker
          id="arrow"
          viewBox="0 0 10 10"
          refX="8"
          refY="5"
          markerWidth="6"
          markerHeight="6"
          orient="auto-start-reverse"
        >
          <path d="M0,0 L10,5 L0,10 z" fill="currentColor" />
        </marker>
      </defs>

      <rect width="1200" height="800" fill="url(#grid)" />

      {/* Corner markers like blueprint */}
      <g className="text-muted-foreground/50" fontFamily="var(--font-mono), ui-monospace, monospace" fontSize="11">
        <text x="28" y="36" className="uppercase tracking-widest" fill="currentColor">FIG_001</text>
        <text x="1172" y="36" className="uppercase tracking-widest" fill="currentColor" textAnchor="end">[ DICE-S3 · ENTROPY NODE ]</text>
        <text x="28" y="780" className="uppercase tracking-widest" fill="currentColor">REV_B · 2025</text>
        <text x="1172" y="780" className="uppercase tracking-widest" fill="currentColor" textAnchor="end">SCHEMATIC / TOP</text>

        {/* thin corner ticks */}
        <polyline points="18,22 18,38 34,38" stroke="currentColor" strokeWidth="1" fill="none" />
        <polyline points="1182,22 1182,38 1166,38" stroke="currentColor" strokeWidth="1" fill="none" />
        <polyline points="18,778 18,762 34,762" stroke="currentColor" strokeWidth="1" fill="none" />
        <polyline points="1182,778 1182,762 1166,762" stroke="currentColor" strokeWidth="1" fill="none" />
      </g>

      {/* ── PCB base ────────────────────────────────────────────── */}
      <g className="text-foreground">
        <SideFace x={0} y={0} w={BOARD_W} h={BOARD_H} z={4} className="" />
        <Parallelogram
          x={0}
          y={0}
          w={BOARD_W}
          h={BOARD_H}
          z={4}
          stroke="currentColor"
          className="fill-muted/30"
        />
      </g>

      {/* ── USB-C connector (left edge) ─────────────────────────── */}
      <ComponentBlock x={-18} y={90} w={70} h={50} z={18} active={isActive("usb")} />

      {/* ── ESP32-S3 SoC (center) ───────────────────────────────── */}
      <g>
        <ComponentBlock x={180} y={110} w={180} h={130} z={22} active={isActive("soc")} />
        {/* inner die rectangle */}
        <Parallelogram
          x={205}
          y={135}
          w={130}
          h={80}
          z={28}
          stroke="currentColor"
          className={isActive("soc") ? "text-foreground fill-foreground/15" : "text-muted-foreground/60 fill-foreground/[0.04]"}
        />
        {/* pin rows on all 4 sides */}
        <PinRow x={180} y={110} len={180} count={12} axis="x" active={isActive("soc")} />
        <PinRow x={180} y={240} len={180} count={12} axis="x" active={isActive("soc")} />
        <PinRow x={180} y={110} len={130} count={10} axis="y" active={isActive("soc")} />
        <PinRow x={360} y={110} len={130} count={10} axis="y" active={isActive("soc")} />
      </g>

      {/* ── WiFi antenna trace (top-right corner) ───────────────── */}
      <g className={isActive("ant") ? "text-foreground" : "text-muted-foreground/55"}>
        <ComponentBlock x={440} y={20} w={130} h={50} z={10} active={isActive("ant")} />
        {/* zig-zag trace inside */}
        <polyline
          points={`${i(450, 45, 16).join(",")} ${i(470, 35, 16).join(",")} ${i(490, 45, 16).join(",")} ${i(510, 35, 16).join(",")} ${i(530, 45, 16).join(",")} ${i(550, 35, 16).join(",")} ${i(560, 45, 16).join(",")}`}
          fill="none"
          stroke="currentColor"
          strokeWidth={1.4}
        />
      </g>

      {/* ── Status LEDs (right edge, 4 stacked) ─────────────────── */}
      {[0, 1, 2, 3].map((n) => (
        <g key={n}>
          <ComponentBlock x={520} y={100 + n * 40} w={28} h={20} z={14} active={isActive("leds")} />
        </g>
      ))}

      {/* ── XTAL + Passives ─────────────────────────────────────── */}
      <ComponentBlock x={80} y={260} w={36} h={24} z={12} active={isActive("passives")} />
      <ComponentBlock x={130} y={260} w={36} h={24} z={12} active={isActive("passives")} />
      <ComponentBlock x={180} y={260} w={36} h={24} z={12} active={isActive("passives")} />
      <ComponentBlock x={230} y={260} w={36} h={24} z={12} active={isActive("passives")} />
      <ComponentBlock x={290} y={260} w={60} h={30} z={16} active={isActive("passives")} />

      {/* ── Buttons (bottom-left) ───────────────────────────────── */}
      <ComponentBlock x={30} y={300} w={30} h={24} z={14} active={isActive("buttons")} />
      <ComponentBlock x={75} y={300} w={30} h={24} z={14} active={isActive("buttons")} />

      {/* ── Labels with leader lines ────────────────────────────── */}

      {/* USB-C — label on left */}
      <LabelLine
        from={[-18 + 35, 90 + 25]}
        to={[160, 220]}
        text="USB-C"
        sub="power / serial"
        active={isActive("usb")}
        align="end"
      />

      {/* ESP32-S3 — label top-center */}
      <LabelLine
        from={[180 + 90, 110 + 10]}
        to={[650, 160]}
        text="ESP32-S3 · N16R8"
        sub="TRNG · ECDSA · secp256k1"
        active={isActive("soc")}
        align="start"
      />

      {/* Antenna — label top-right */}
      <LabelLine
        from={[440 + 65, 20 + 25]}
        to={[900, 90]}
        text="WiFi · ANT"
        sub="2.4 GHz · mTLS link"
        active={isActive("ant")}
        align="start"
      />

      {/* LEDs — label right */}
      <LabelLine
        from={[534, 140]}
        to={[900, 280]}
        text="Status LEDs"
        sub="pwr · stat · ent · net"
        active={isActive("leds")}
        align="start"
      />

      {/* Passives / XTAL — bottom-right */}
      <LabelLine
        from={[320, 272]}
        to={[900, 430]}
        text="Passives · XTAL"
        sub="26 MHz clock · decoupling"
        active={isActive("passives")}
        align="start"
      />

      {/* Buttons — bottom-left */}
      <LabelLine
        from={[90, 312]}
        to={[160, 560]}
        text="BOOT · RST"
        sub="factory reset · wifi provisioning"
        active={isActive("buttons")}
        align="end"
      />
    </svg>
  )
}

export default Esp32Blueprint
