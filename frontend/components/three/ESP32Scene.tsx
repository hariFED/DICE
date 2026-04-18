"use client";

import { useRef, useState } from "react";
import {
  motion,
  useScroll,
  useTransform,
  useMotionValueEvent,
  type MotionValue,
} from "framer-motion";
import { cn } from "@/lib/utils";

/* ──────────────────────────────────────────────────────────────────────────
 *  Scroll-driven ESP32 showcase — pencil-line SVG edition.
 *
 *  The old r3f-based scene was expensive to render, flaky on low-GPU
 *  devices, and visually didn't match the rest of the site. This version:
 *    - Hand-drawn ESP32 dev-board SVG, stroke-only (white on transparent).
 *    - Components (USB-C, LEDs, SoC, antenna, pin headers) live in their
 *      own groups so we can highlight one at a time.
 *    - Scroll phases drive a spotlight ring + faint grid that lerp between
 *      component centers, plus a gentle parallax tilt.
 *    - Zero React state writes on scroll — all smooth via motion values.
 * ────────────────────────────────────────────────────────────────────────── */

const ANNOTATIONS = [
  { at: [0,     0.17] as const, text: "Meet the DICE Node",  sub: "ESP32-S3 Development Board" },
  { at: [0.17,  0.33] as const, text: "USB-C Power",         sub: "Plug in, always on. 5 V, 500 mA." },
  { at: [0.33,  0.5]  as const, text: "Status LEDs",         sub: "Green: online. Blue: processing entropy." },
  { at: [0.5,   0.67] as const, text: "ESP32-S3 SoC",        sub: "Hardware TRNG + ADC noise. True entropy." },
  { at: [0.67,  0.83] as const, text: "WiFi Antenna",        sub: "Talks to the coordinator over mTLS." },
  { at: [0.83,  1]    as const, text: "What's Inside",       sub: "Every component working together." },
];

/** Normalised coordinates of each feature we spotlight (board is 800×480). */
const SPOTLIGHTS = [
  { x: 400, y: 240, r: 340 }, // whole board
  { x:  80, y: 240, r:  55 }, // USB-C
  { x: 230, y: 120, r:  38 }, // LEDs
  { x: 440, y: 240, r:  95 }, // SoC
  { x: 720, y: 130, r:  70 }, // antenna
  { x: 400, y: 240, r: 340 }, // full board (reveal all)
];

export function ESP32Scene() {
  const containerRef = useRef<HTMLDivElement>(null);
  const { scrollYProgress } = useScroll({
    target: containerRef,
    offset: ["start start", "end end"],
  });

  // Annotation index — the one piece of state that needs React.
  const [idx, setIdx] = useState(0);
  useMotionValueEvent(scrollYProgress, "change", (v) => {
    const next = ANNOTATIONS.findIndex((a) => v >= a.at[0] && v < a.at[1]);
    const resolved = next === -1 ? ANNOTATIONS.length - 1 : next;
    if (resolved !== idx) setIdx(resolved);
  });

  // Spotlight position / radius — piped into the SVG via motion-valued
  // attributes so they animate on every frame without a re-render.
  const stops = SPOTLIGHTS.map((_, i) => i / (SPOTLIGHTS.length - 1));
  const spotX = useTransform(scrollYProgress, stops, SPOTLIGHTS.map((s) => s.x));
  const spotY = useTransform(scrollYProgress, stops, SPOTLIGHTS.map((s) => s.y));
  const spotR = useTransform(scrollYProgress, stops, SPOTLIGHTS.map((s) => s.r));

  // Stroke reveal: the whole board draws itself in during the first 15 % of
  // scroll, then stays drawn.
  const drawProgress = useTransform(scrollYProgress, [0, 0.15], [0, 1]);

  return (
    <div
      ref={containerRef}
      // 280 vh keeps the section snappy — at 400 vh it felt like a trap.
      // `position: relative` so Framer's useScroll measures against us.
      // Intentionally NO `overflow-hidden` here; wheel events bubble to page.
      style={{ position: "relative", height: "280vh" }}
    >
      <div className="sticky top-0 h-screen grid grid-cols-1 md:grid-cols-[1fr_1.3fr] items-center gap-8 md:gap-12 px-6 md:px-16 bg-background">
        {/* ── Annotation column ── */}
        <div className="pointer-events-none select-none max-w-md">
          <p className="text-xs uppercase tracking-[0.24em] text-[#00FF85] mb-3 font-mono">
            The DICE Node
          </p>
          <h3
            key={idx}
            className="text-3xl md:text-4xl font-bold text-white leading-tight transition-all duration-500"
          >
            {ANNOTATIONS[idx]?.text}
          </h3>
          <p className="mt-3 text-sm md:text-base text-zinc-400 transition-all duration-500 max-w-xs">
            {ANNOTATIONS[idx]?.sub}
          </p>
          <div className="mt-8 flex gap-1.5">
            {ANNOTATIONS.map((_, i) => (
              <div
                key={i}
                className={cn(
                  "h-1 rounded-full transition-all duration-300",
                  i === idx ? "w-8 bg-[#00FF85]" : "w-2 bg-zinc-700",
                )}
              />
            ))}
          </div>
          <p className="mt-10 text-[11px] text-zinc-600 uppercase tracking-[0.2em]">
            Scroll to inspect ↓
          </p>
        </div>

        {/* ── SVG illustration column ── */}
        <div className="w-full max-w-[720px] justify-self-center md:justify-self-end">
          <Board
            spotX={spotX}
            spotY={spotY}
            spotR={spotR}
            drawProgress={drawProgress}
          />
        </div>
      </div>
    </div>
  );
}

export default ESP32Scene;

/* ──────────────────────────────────────────────────────────────────────────
 *  The hand-drawn board.
 *
 *  Coordinates use an 800×480 viewBox. Everything is stroke-only so the
 *  site's dark background shows through. The spotlight mask brightens the
 *  currently-annotated region; everything else dims to ~30 % opacity.
 * ────────────────────────────────────────────────────────────────────────── */
function Board({
  spotX,
  spotY,
  spotR,
  drawProgress,
}: {
  spotX: MotionValue<number>;
  spotY: MotionValue<number>;
  spotR: MotionValue<number>;
  drawProgress: MotionValue<number>;
}) {
  return (
    <svg
      viewBox="0 0 800 480"
      xmlns="http://www.w3.org/2000/svg"
      className="w-full h-full overflow-visible"
      style={{ filter: "drop-shadow(0 30px 60px rgba(0, 255, 133, 0.08))" }}
      aria-label="ESP32-S3 development board illustration"
    >
      <defs>
        {/* Spotlight mask — anything inside spotR is fully visible, outside
            is dimmed. Radius + center animate via motion values. */}
        <radialGradient id="spot" cx="50%" cy="50%" r="50%">
          <stop offset="0%"  stopColor="white" stopOpacity="1" />
          <stop offset="75%" stopColor="white" stopOpacity="0.35" />
          <stop offset="100%" stopColor="white" stopOpacity="0.15" />
        </radialGradient>
        <mask id="spot-mask">
          <rect width="800" height="480" fill="white" fillOpacity="0.28" />
          <motion.circle
            cx={spotX}
            cy={spotY}
            r={spotR}
            fill="url(#spot)"
          />
        </mask>
        <linearGradient id="strokeGrad" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%"   stopColor="#ffffff" stopOpacity="0.95" />
          <stop offset="100%" stopColor="#d4d4d8" stopOpacity="0.75" />
        </linearGradient>
      </defs>

      <g
        stroke="url(#strokeGrad)"
        strokeWidth="1.4"
        fill="none"
        strokeLinecap="round"
        strokeLinejoin="round"
        mask="url(#spot-mask)"
      >
        {/* ── Board outline ─────────────────────────────────────────── */}
        <motion.rect
          x="40" y="60" width="720" height="360" rx="20"
          strokeWidth="1.6"
          pathLength={drawProgress}
          strokeDasharray="1 1"
          strokeDashoffset={useInverse(drawProgress)}
        />
        {/* Inner silkscreen line */}
        <rect x="52" y="72" width="696" height="336" rx="14" strokeOpacity="0.45" />

        {/* Mounting holes */}
        <circle cx="70"  cy="90"  r="6" />
        <circle cx="730" cy="90"  r="6" />
        <circle cx="70"  cy="390" r="6" />
        <circle cx="730" cy="390" r="6" />

        {/* ── USB-C connector (left edge) ──────────────────────────── */}
        <g>
          <rect x="20"  y="210" width="78" height="60" rx="8" />
          <rect x="28"  y="218" width="62" height="44" rx="4" strokeOpacity="0.55" />
          {/* 8 pin markers inside the USB-C mouth */}
          {Array.from({ length: 8 }).map((_, i) => (
            <line
              key={i}
              x1={36 + i * 6}
              y1="232"
              x2={36 + i * 6}
              y2="248"
              strokeOpacity="0.7"
            />
          ))}
        </g>

        {/* ── Status LEDs (upper-left area) ─────────────────────────── */}
        <g>
          <circle cx="210" cy="120" r="8" />
          <circle cx="210" cy="120" r="3" strokeOpacity="0.6" />
          <circle cx="250" cy="120" r="8" />
          <circle cx="250" cy="120" r="3" strokeOpacity="0.6" />
          <text x="200" y="148" fontSize="9" fill="#a1a1aa" fontFamily="ui-monospace">LED1</text>
          <text x="240" y="148" fontSize="9" fill="#a1a1aa" fontFamily="ui-monospace">LED2</text>
        </g>

        {/* ── ESP32-S3 SoC (centre) ─────────────────────────────────── */}
        <g>
          <rect x="360" y="170" width="160" height="140" rx="4" strokeWidth="1.8" />
          <rect x="372" y="182" width="136" height="116" rx="2" strokeOpacity="0.45" />
          <text
            x="440" y="228" textAnchor="middle" fontSize="14"
            fill="#e4e4e7" fontFamily="ui-monospace" fontWeight="700"
          >
            ESP32-S3
          </text>
          <text
            x="440" y="250" textAnchor="middle" fontSize="9"
            fill="#a1a1aa" fontFamily="ui-monospace"
          >
            Xtensa LX7 dual-core
          </text>
          <text
            x="440" y="266" textAnchor="middle" fontSize="9"
            fill="#a1a1aa" fontFamily="ui-monospace"
          >
            TRNG · Wi-Fi · BLE
          </text>
          {/* SoC orientation pin */}
          <circle cx="374" cy="184" r="2" fill="#d4d4d8" strokeWidth="0" />
          {/* QFN legs — tick marks all four sides */}
          {Array.from({ length: 16 }).map((_, i) => (
            <g key={`l-${i}`}>
              <line x1="357" x2="362" y1={180 + i * 8} y2={180 + i * 8} strokeOpacity="0.55" />
              <line x1="518" x2="523" y1={180 + i * 8} y2={180 + i * 8} strokeOpacity="0.55" />
            </g>
          ))}
          {Array.from({ length: 18 }).map((_, i) => (
            <g key={`tb-${i}`}>
              <line y1="167" y2="172" x1={368 + i * 8} x2={368 + i * 8} strokeOpacity="0.55" />
              <line y1="308" y2="313" x1={368 + i * 8} x2={368 + i * 8} strokeOpacity="0.55" />
            </g>
          ))}
        </g>

        {/* ── WiFi antenna (upper-right) ────────────────────────────── */}
        <g>
          <rect x="650" y="96" width="110" height="64" rx="3" />
          {/* Meandering trace inside the antenna footprint */}
          <path d="M 662 110 L 694 110 L 694 122 L 722 122 L 722 110 L 748 110" strokeOpacity="0.75" />
          <path d="M 662 130 L 694 130 L 694 142 L 722 142 L 722 130 L 748 130" strokeOpacity="0.75" />
          {/* Radiating arcs (signal) */}
          <path d="M 660 128 Q 640 128 640 110 Q 640 92 660 92" strokeOpacity="0.35" strokeDasharray="3 3" />
          <path d="M 650 140 Q 620 140 620 110 Q 620 80 650 80"  strokeOpacity="0.25" strokeDasharray="3 3" />
          <path d="M 640 152 Q 600 152 600 110 Q 600 68 640 68"  strokeOpacity="0.18" strokeDasharray="3 3" />
        </g>

        {/* ── Pin headers (top + bottom) ────────────────────────────── */}
        <g strokeOpacity="0.55">
          {Array.from({ length: 18 }).map((_, i) => {
            const x = 110 + i * 26;
            return (
              <g key={`hdr-${i}`}>
                <rect x={x - 4} y="72" width="8" height="14" rx="1" />
                <rect x={x - 4} y="394" width="8" height="14" rx="1" />
              </g>
            );
          })}
        </g>

        {/* ── Random GPIO labels (give it that dev-board personality) ── */}
        <g
          stroke="none"
          fill="#71717a"
          fontFamily="ui-monospace"
          fontSize="7"
          textAnchor="middle"
        >
          {["3V3","GND","IO0","IO1","IO2","IO3","IO4","RX","TX","IO5","IO6","IO7","IO8","IO9","IO10","IO11","5V","GND"].map((label, i) => (
            <text key={i} x={110 + i * 26} y="66">{label}</text>
          ))}
        </g>

        {/* ── Passive components sprinkle (caps + resistors near SoC) ── */}
        <g strokeOpacity="0.55">
          <rect x="338" y="200" width="10" height="6" />
          <rect x="338" y="214" width="10" height="6" />
          <rect x="338" y="228" width="10" height="6" />
          <rect x="338" y="244" width="10" height="6" />
          <rect x="338" y="258" width="10" height="6" />
          <rect x="532" y="200" width="10" height="6" />
          <rect x="532" y="214" width="10" height="6" />
          <rect x="532" y="228" width="10" height="6" />
          <rect x="532" y="244" width="10" height="6" />
          <rect x="532" y="258" width="10" height="6" />
        </g>

        {/* ── Crystal oscillator (bottom-left of SoC) ───────────────── */}
        <g>
          <rect x="310" y="336" width="32" height="14" rx="2" />
          <text x="326" y="360" textAnchor="middle" fontSize="8" fill="#71717a" stroke="none" fontFamily="ui-monospace">40 MHz</text>
        </g>

        {/* ── Flash chip (right of SoC) ─────────────────────────────── */}
        <g>
          <rect x="550" y="336" width="44" height="22" rx="2" />
          <text x="572" y="350" textAnchor="middle" fontSize="7" fill="#71717a" stroke="none" fontFamily="ui-monospace">FLASH 16 MB</text>
        </g>

        {/* ── Version mark (bottom-right silkscreen) ────────────────── */}
        <g stroke="none" fill="#71717a" fontFamily="ui-monospace" fontSize="9">
          <text x="750" y="402" textAnchor="end">DICE NODE v7</text>
          <text x="750" y="414" textAnchor="end" fontSize="7">ESP32-S3-N16R8</text>
        </g>
      </g>

      {/* Crosshair accent on the current spotlight (drawn OUTSIDE the mask
          so it stays fully bright). */}
      <motion.g
        style={{ x: spotX, y: spotY }}
        stroke="#00FF85"
        strokeWidth="1.2"
        fill="none"
        opacity="0.7"
      >
        <motion.circle cx="0" cy="0" r={useSmaller(spotR, 4)} strokeDasharray="4 4" />
        <line x1="-18" y1="0" x2="-8" y2="0" />
        <line x1="8"   y1="0" x2="18" y2="0" />
        <line x1="0" y1="-18" x2="0" y2="-8" />
        <line x1="0" y1="8"   x2="0" y2="18" />
      </motion.g>
    </svg>
  );
}

/* ──────────────────────────────────────────────────────────────────────────
 *  Motion-value helpers — tiny, inline because we only need them once.
 * ────────────────────────────────────────────────────────────────────────── */
function useInverse(mv: MotionValue<number>): MotionValue<number> {
  return useTransform(mv, (v) => 1 - v);
}

function useSmaller(
  mv: MotionValue<number>,
  shrinkBy: number,
): MotionValue<number> {
  return useTransform(mv, (v) => Math.max(0, v - shrinkBy));
}
