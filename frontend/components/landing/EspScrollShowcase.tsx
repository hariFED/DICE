"use client"

import { useEffect, useRef, useState } from "react"
import type { MotionValue } from "framer-motion"
import { motion, useScroll, useTransform, useSpring } from "framer-motion"
import { SectionHeader } from "@/components/landing/HowItWorks"
import { Esp32Blueprint, type Esp32Key } from "@/components/landing/Esp32Blueprint"

/**
 * Scroll-driven ESP32 component tour.
 *
 * A section that's ~6× the viewport tall. Its contents are sticky-positioned
 * and stay pinned while the page scrolls through the range. Scroll progress
 * selects which component is "active" — the matching board row is bolded,
 * the matching callout slot is opaque.
 *
 * Design intent (per user feedback):
 *   • less prose, more diagram (inline ASCII flow for each component)
 *   • proper spacing between silkscreen elements
 *   • index strip as TUI chrome: [ 01 ][ 02 ][ 03 ][ 04 ][ 05 ][ 06 ]
 */

type Component = {
  key: Esp32Key
  index: string
  name: string
  flow: string
  detail: string
}

const COMPONENTS: Component[] = [
  {
    key: "usb",
    index: "01",
    name: "USB-C",
    flow: "host ──→ USB ──→ serial ──→ device",
    detail: "Power + firmware flashing. No data pipe to prod.",
  },
  {
    key: "soc",
    index: "02",
    name: "ESP32-S3",
    flow: "TRNG ──→ SHA-256 ──→ secp256k1 ──→ signed reveal",
    detail: "Xtensa dual-core + hardware RNG + in-chip signing.",
  },
  {
    key: "ant",
    index: "03",
    name: "WiFi · ANT",
    flow: "device ──wss──→ coordinator (mTLS · cert-pinned)",
    detail: "2.4 GHz. Persistent WebSocket. Client-cert identity.",
  },
  {
    key: "leds",
    index: "04",
    name: "Status LEDs",
    flow: "◉ pwr   ◉ stat   ◉ ent   ◉ net",
    detail: "Visual heartbeat — lit green while a round is in progress.",
  },
  {
    key: "passives",
    index: "05",
    name: "Passives · XTAL",
    flow: "26 MHz xtal ──→ PLL ──→ core clock",
    detail: "Oscillator + decoupling. Stable timing for ECDSA.",
  },
  {
    key: "buttons",
    index: "06",
    name: "BOOT · RST",
    flow: "hold BOOT 5s ──→ NVS reset ──→ captive-portal",
    detail: "WiFi reprovisioning without reflashing.",
  },
]

// ── MotionValue helpers ────────────────────────────────────────────────────

function useMotionSubscribe<T>(mv: MotionValue<T>, cb: (v: T) => void) {
  useEffect(() => {
    cb(mv.get())
    const unsub = mv.on("change", cb)
    return () => unsub()
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [mv])
}

// ── Sub-components ─────────────────────────────────────────────────────────

function BoardView({ activeIndex }: { activeIndex: MotionValue<number> }) {
  const [idx, setIdx] = useState(0)
  useMotionSubscribe(activeIndex, (n) => setIdx(n))
  return <Esp32Blueprint activeKey={COMPONENTS[idx]?.key} />
}

function IndexStrip({ activeIndex }: { activeIndex: MotionValue<number> }) {
  const [idx, setIdx] = useState(0)
  useMotionSubscribe(activeIndex, (n) => setIdx(n))
  return (
    <div className="grid grid-cols-6 border border-border text-[10px] font-pixel">
      {COMPONENTS.map((c, i) => (
        <span
          key={c.key}
          className={`text-center px-2 py-1.5 border-r border-border last:border-0 transition-colors ${
            i === idx ? "bg-foreground text-background" : "text-muted-foreground"
          }`}
        >
          {c.index}
        </span>
      ))}
    </div>
  )
}

function Callout({
  component,
  idx,
  activeIndex,
  total,
}: {
  component: Component
  idx: number
  activeIndex: MotionValue<number>
  total: number
}) {
  const [active, setActive] = useState(false)
  useMotionSubscribe(activeIndex, (n) => setActive(n === idx))
  return (
    <motion.div
      initial={false}
      animate={{ opacity: active ? 1 : 0, y: active ? 0 : 12 }}
      transition={{ duration: 0.35 }}
      className="absolute inset-0 flex flex-col justify-start font-mono"
      style={{ pointerEvents: active ? "auto" : "none" }}
    >
      <p className="font-pixel text-6xl md:text-7xl text-foreground leading-none tabular-nums">
        {component.index}
      </p>
      <p className="mt-2 ascii-label">
        {String(idx + 1).padStart(2, "0")} / {String(total).padStart(2, "0")}
      </p>

      <h3 className="mt-8 text-2xl md:text-3xl font-semibold tracking-tight">
        {component.name}
      </h3>

      <pre className="mt-6 font-mono text-xs md:text-sm text-foreground whitespace-pre-wrap leading-relaxed">
        {component.flow}
      </pre>

      <p className="mt-6 text-sm text-muted-foreground leading-relaxed max-w-sm">
        {component.detail}
      </p>
    </motion.div>
  )
}

function ActiveCounter({
  activeIndex,
  total,
}: {
  activeIndex: MotionValue<number>
  total: number
}) {
  const [idx, setIdx] = useState(0)
  useMotionSubscribe(activeIndex, setIdx)
  return (
    <span className="tabular-nums">
      {String(idx + 1).padStart(2, "0")} / {String(total).padStart(2, "0")}
    </span>
  )
}

// ── Main export ────────────────────────────────────────────────────────────

export function EspScrollShowcase() {
  const sectionRef = useRef<HTMLDivElement>(null)
  const { scrollYProgress } = useScroll({
    target: sectionRef,
    offset: ["start start", "end end"],
  })
  const smooth = useSpring(scrollYProgress, { stiffness: 80, damping: 20 })

  const activeIndex = useTransform(smooth, (p) => {
    // Clamp to [0, N-1]; treat the last sliver as the final component.
    const i = Math.floor(p * COMPONENTS.length)
    return Math.max(0, Math.min(COMPONENTS.length - 1, i))
  })

  return (
    <section
      ref={sectionRef}
      className="relative border-b border-border"
      style={{ height: `${COMPONENTS.length * 80}vh` }}
    >
      <div className="sticky top-0 h-screen flex flex-col">
        <div className="mx-auto w-full max-w-[1400px] px-4 sm:px-6 lg:px-8 py-14 md:py-16 flex-1 flex flex-col min-h-0">
          <SectionHeader
            index="03"
            name="entropy · node"
            title="The DICE-S3 board."
            lead="Six labelled components. Scroll to walk through each. No moving parts — the only thing between you and randomness is silicon."
          />

          <div className="mt-8 md:mt-10 grid grid-cols-1 lg:grid-cols-12 gap-6 flex-1 items-stretch min-h-0">
            {/* LEFT — blueprint */}
            <div className="lg:col-span-7 relative border border-border bg-background overflow-hidden">
              <div className="h-full flex items-center justify-center p-4">
                <BoardView activeIndex={activeIndex} />
              </div>
            </div>

            {/* RIGHT — callout */}
            <div className="lg:col-span-5 flex flex-col min-h-0 overflow-hidden">
              <IndexStrip activeIndex={activeIndex} />

              <div className="relative flex-1 mt-6">
                {COMPONENTS.map((c, i) => (
                  <Callout
                    key={c.key}
                    component={c}
                    idx={i}
                    activeIndex={activeIndex}
                    total={COMPONENTS.length}
                  />
                ))}
              </div>

              <div className="mt-4 flex items-center justify-between font-mono text-[10px] uppercase tracking-wider text-muted-foreground">
                <span>scroll ──→ next</span>
                <span className="font-pixel">
                  <ActiveCounter activeIndex={activeIndex} total={COMPONENTS.length} />
                </span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </section>
  )
}

export default EspScrollShowcase
