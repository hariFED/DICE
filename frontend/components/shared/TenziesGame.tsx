"use client"

import { useState, useEffect, useCallback, useRef } from "react"
import { motion, AnimatePresence } from "framer-motion"
import { cn } from "@/lib/utils"
import { BracketButton } from "@/components/shared/BracketButton"

/* ── types ─────────────────────────────────────────────────────── */
interface Die {
  id: number
  value: number
  isHeld: boolean
}

/* ── helpers ───────────────────────────────────────────────────── */
function randomValue() {
  return Math.ceil(Math.random() * 6)
}

function createInitialDice(): Die[] {
  return Array.from({ length: 10 }, (_, i) => ({
    id: i,
    value: 1,
    isHeld: false,
  }))
}

/* ── dice pip positions (3x3 grid, positions 1-9) ─────────────── */
/*
  1  2  3
  4  5  6
  7  8  9
*/
const PIP_MAP: Record<number, number[]> = {
  1: [5],
  2: [3, 7],
  3: [3, 5, 7],
  4: [1, 3, 7, 9],
  5: [1, 3, 5, 7, 9],
  6: [1, 3, 4, 6, 7, 9],
}

/* ── dice face component ───────────────────────────────────────── */
function DiceFace({ value, isHeld }: { value: number; isHeld: boolean }) {
  const pips = PIP_MAP[value] || []
  return (
    <div className="grid grid-cols-3 grid-rows-3 gap-[3px] w-full h-full p-[5px]">
      {Array.from({ length: 9 }, (_, i) => {
        const pos = i + 1
        const visible = pips.includes(pos)
        return (
          <div key={pos} className="flex items-center justify-center">
            {visible && (
              <motion.div
                initial={{ scale: 0 }}
                animate={{ scale: 1 }}
                transition={{ type: "spring", stiffness: 500, damping: 25, delay: Math.random() * 0.08 }}
                className={cn(
                  "w-[6px] h-[6px] sm:w-[7px] sm:h-[7px] rounded-full",
                  isHeld
                    ? "bg-black/90"
                    : "bg-white/80"
                )}
              />
            )}
          </div>
        )
      })}
    </div>
  )
}

/* ── glass styles ──────────────────────────────────────────────── */
const glassPanel = [
  "bg-white/[0.03] backdrop-blur-xl",
  "border border-white/[0.08]",
  "shadow-[0_-4px_24px_rgba(0,0,0,0.3),inset_0_1px_0_rgba(255,255,255,0.06)]",
  "rounded-lg relative overflow-hidden",
].join(" ")

/* ── localStorage helpers ──────────────────────────────────────── */
function getBestTime(): number | null {
  if (typeof window === "undefined") return null
  const stored = localStorage.getItem("tenzies-best-time")
  return stored ? parseInt(stored, 10) : null
}

function saveBestTime(seconds: number) {
  if (typeof window === "undefined") return
  const current = getBestTime()
  if (current === null || seconds < current) {
    localStorage.setItem("tenzies-best-time", String(seconds))
  }
}

/* ── component ─────────────────────────────────────────────────── */
export function TenziesGame() {
  const [dice, setDice] = useState<Die[]>(createInitialDice)
  const [rollCount, setRollCount] = useState(0)
  const [isRolling, setIsRolling] = useState(false)
  const [startTime, setStartTime] = useState<number | null>(null)
  const [elapsed, setElapsed] = useState(0)
  const [hasWon, setHasWon] = useState(false)
  const [bestTime, setBestTime] = useState<number | null>(null)
  const rollIntervalRef = useRef<ReturnType<typeof setInterval> | null>(null)

  /* ── load best time on mount ──────────────────────────────────── */
  useEffect(() => {
    setBestTime(getBestTime())
  }, [])

  /* ── win detection ────────────────────────────────────────────── */
  const checkWin = useCallback((d: Die[]) => {
    return d.every((die) => die.isHeld) && d.every((die) => die.value === d[0].value)
  }, [])

  /* ── timer ────────────────────────────────────────────────────── */
  useEffect(() => {
    if (startTime === null || hasWon) return
    const id = setInterval(() => {
      setElapsed(Math.floor((Date.now() - startTime) / 1000))
    }, 100)
    return () => clearInterval(id)
  }, [startTime, hasWon])

  /* ── handle win ───────────────────────────────────────────────── */
  useEffect(() => {
    if (hasWon && elapsed > 0) {
      saveBestTime(elapsed)
      setBestTime(getBestTime())
    }
  }, [hasWon, elapsed])

  /* ── rolling animation: cycle through random values rapidly ──── */
  const startRollingAnimation = useCallback(
    (finalDice: Die[]) => {
      let ticks = 0
      const maxTicks = 6
      rollIntervalRef.current = setInterval(() => {
        ticks++
        if (ticks >= maxTicks) {
          // final values
          if (rollIntervalRef.current) clearInterval(rollIntervalRef.current)
          setDice(finalDice)
          setIsRolling(false)
          if (checkWin(finalDice)) {
            setTimeout(() => setHasWon(true), 200)
          }
          return
        }
        // show random intermediate values on unheld dice
        setDice((prev) =>
          prev.map((die) => (die.isHeld ? die : { ...die, value: randomValue() }))
        )
      }, 60)
    },
    [checkWin]
  )

  /* ── roll dice ────────────────────────────────────────────────── */
  const rollDice = useCallback(() => {
    if (isRolling || hasWon) return

    setIsRolling(true)

    // compute final values
    const finalDice = dice.map((die) =>
      die.isHeld ? die : { ...die, value: randomValue() }
    )

    setRollCount((c) => {
      if (c === 0) setStartTime(Date.now())
      return c + 1
    })

    // start the cycling animation, then land on final values
    startRollingAnimation(finalDice)
  }, [isRolling, hasWon, dice, startRollingAnimation])

  /* ── cleanup rolling interval ─────────────────────────────────── */
  useEffect(() => {
    return () => {
      if (rollIntervalRef.current) clearInterval(rollIntervalRef.current)
    }
  }, [])

  /* ── toggle hold ──────────────────────────────────────────────── */
  const toggleHold = useCallback(
    (id: number) => {
      if (rollCount === 0 || isRolling || hasWon) return
      setDice((prev) => {
        const next = prev.map((die) =>
          die.id === id ? { ...die, isHeld: !die.isHeld } : die
        )
        if (checkWin(next)) {
          setTimeout(() => setHasWon(true), 200)
        }
        return next
      })
    },
    [rollCount, isRolling, hasWon, checkWin]
  )

  /* ── new game ─────────────────────────────────────────────────── */
  const newGame = useCallback(() => {
    if (rollIntervalRef.current) clearInterval(rollIntervalRef.current)
    setDice(createInitialDice())
    setRollCount(0)
    setStartTime(null)
    setElapsed(0)
    setHasWon(false)
    setIsRolling(false)
  }, [])

  /* ── format time ──────────────────────────────────────────────── */
  const fmtTime = (s: number) => (s < 60 ? `${s}s` : `${Math.floor(s / 60)}m${s % 60}s`)
  const timeDisplay = fmtTime(elapsed)
  const bestDisplay = bestTime !== null ? fmtTime(bestTime) : "—"

  return (
    <div role="region" aria-label="Tenzies dice game" className="flex flex-col items-center">
      <motion.div
        className={cn(glassPanel, "w-full max-w-lg p-4 sm:p-5")}
        animate={
          hasWon
            ? {
                borderColor: [
                  "rgba(255,255,255,0.08)",
                  "rgba(255,255,255,0.4)",
                  "rgba(255,255,255,0.08)",
                ],
              }
            : {}
        }
        transition={hasWon ? { duration: 1.5, repeat: 3 } : {}}
      >
        {/* bg grid texture */}
        <div className="absolute inset-0 bg-grid-fine opacity-30 pointer-events-none" />

        {/* ── header label ─────────────────────────────────────────── */}
        <div className="relative text-center mb-3">
          <span className="ascii-label text-[10px]">tenzies &middot; dice</span>
        </div>

        {/* ── stats row ───────────────────────────────────────────── */}
        <div className="relative flex items-center justify-center gap-5 sm:gap-6 mb-4">
          <div className="text-center">
            <p className="text-[10px] text-muted-foreground/50 font-mono uppercase tracking-wider">Rolls</p>
            <p className="font-pixel text-sm text-foreground/90">{rollCount}</p>
          </div>
          <div className="w-px h-6 bg-white/[0.08]" />
          <div className="text-center">
            <p className="text-[10px] text-muted-foreground/50 font-mono uppercase tracking-wider">Best Time</p>
            <p className="font-pixel text-sm text-foreground/90">{bestDisplay}</p>
          </div>
          <div className="w-px h-6 bg-white/[0.08]" />
          <div className="text-center">
            <p className="text-[10px] text-muted-foreground/50 font-mono uppercase tracking-wider">Time</p>
            <p className="font-pixel text-sm text-foreground/90">{timeDisplay}</p>
          </div>
        </div>

        {/* ── win banner ──────────────────────────────────────────── */}
        <AnimatePresence>
          {hasWon && (
            <motion.div
              initial={{ opacity: 0, scale: 0.9, y: 8 }}
              animate={{ opacity: 1, scale: 1, y: 0 }}
              exit={{ opacity: 0, scale: 0.9, y: -8 }}
              transition={{ type: "spring", stiffness: 300, damping: 20 }}
              className="relative mb-4"
            >
              <pre className="text-center text-[10px] leading-tight text-foreground/90 font-mono select-none">
{`╔══════════════════════════╗
║   YOU WIN!  ALL MATCH!   ║
╚══════════════════════════╝`}
              </pre>
              <p className="text-center text-[11px] text-muted-foreground mt-1.5 font-mono">
                {rollCount} rolls &middot; {timeDisplay}
                {bestTime !== null && elapsed <= bestTime && (
                  <span className="ml-2 text-foreground/80">&#9733; new best!</span>
                )}
              </p>
            </motion.div>
          )}
        </AnimatePresence>

        {/* ── dice grid ───────────────────────────────────────────── */}
        <div className="relative grid grid-cols-5 gap-2 sm:gap-3 max-w-[340px] mx-auto mb-4">
          {dice.map((die, i) => (
            <motion.button
              key={die.id}
              onClick={() => toggleHold(die.id)}
              animate={
                hasWon
                  ? {
                      y: [0, -14, 0],
                      transition: {
                        delay: i * 0.06,
                        duration: 0.5,
                        repeat: 2,
                        ease: "easeInOut",
                      },
                    }
                  : isRolling && !die.isHeld
                    ? {
                        rotate: [0, -12, 12, -6, 6, 0],
                        scale: [1, 0.92, 1.08, 0.96, 1.02, 1],
                        transition: {
                          duration: 0.35,
                          ease: "easeInOut",
                          delay: i * 0.02,
                          repeat: Infinity,
                          repeatType: "loop" as const,
                        },
                      }
                    : {
                        rotate: 0,
                        scale: 1,
                        transition: { type: "spring", stiffness: 400, damping: 20 },
                      }
              }
              whileTap={
                rollCount > 0 && !isRolling && !hasWon
                  ? { scale: 0.88 }
                  : undefined
              }
              whileHover={
                rollCount > 0 && !isRolling && !hasWon
                  ? { scale: 1.06, transition: { duration: 0.15 } }
                  : undefined
              }
              aria-label={`Die ${i + 1}: value ${die.value}, ${die.isHeld ? "held" : "not held"}`}
              className={cn(
                "aspect-square rounded-md flex items-center justify-center select-none cursor-pointer",
                "transition-colors duration-200",
                "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-white/30",
                die.isHeld
                  ? [
                      "bg-white/90 border-2 border-white/50",
                      "shadow-[0_0_12px_rgba(255,255,255,0.2),inset_0_1px_2px_rgba(0,0,0,0.05)]",
                    ].join(" ")
                  : [
                      "bg-white/[0.06] border border-white/[0.12]",
                      "hover:bg-white/[0.10] hover:border-white/[0.22]",
                      "shadow-[inset_0_1px_0_rgba(255,255,255,0.06)]",
                    ].join(" ")
              )}
            >
              <DiceFace value={die.value} isHeld={die.isHeld} />
            </motion.button>
          ))}
        </div>

        {/* ── actions ─────────────────────────────────────────────── */}
        <div className="relative flex items-center justify-center gap-3">
          {hasWon ? (
            <BracketButton onClick={newGame} glyph="&#8635;">
              NEW GAME
            </BracketButton>
          ) : (
            <>
              <BracketButton onClick={rollDice} disabled={isRolling}>
                {rollCount === 0 ? "ROLL DICE" : "ROLL"}
              </BracketButton>
              {rollCount > 0 && (
                <BracketButton onClick={newGame} variant="ghost">
                  RESET
                </BracketButton>
              )}
            </>
          )}
        </div>

        {/* ── hint ────────────────────────────────────────────────── */}
        <p className="relative text-center text-[10px] text-muted-foreground/40 mt-3 font-mono select-none leading-relaxed">
          {hasWon
            ? "all dice matched!"
            : rollCount === 0
              ? "roll dice \u00B7 click to freeze \u00B7 match all 10"
              : "click dice to freeze \u00B7 match all 10 to win"}
        </p>
      </motion.div>
    </div>
  )
}
