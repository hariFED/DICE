"use client"

import { useEffect, useLayoutEffect, useState, useCallback, useRef } from "react"
import { motion } from "framer-motion"
import { Logo } from "./Logo"
import { BracketButton } from "./BracketButton"
import { BRAND } from "@/lib/constants"

const STORAGE_KEY = "dice_portal_seen"

/**
 * Full-screen portal gate shown to first-time visitors.
 *
 * Lifecycle:
 *   mount → check localStorage → show overlay or bail
 *   user clicks Enter → staggered exit animation → scroll to top → unmount
 *   return visitor → never rendered at all
 */
export function PortalGate() {
  // null = not determined yet (SSR/first render); true = show; false = hide
  const [show, setShow] = useState<boolean | null>(null)
  const [exiting, setExiting] = useState(false)
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  // Determine visibility on mount — runs before paint to avoid flash
  useLayoutEffect(() => {
    try {
      if (localStorage.getItem(STORAGE_KEY)) {
        setShow(false)
        return
      }
    } catch {}
    setShow(true)
  }, [])

  // Scroll lock while overlay is visible and not exiting
  useEffect(() => {
    if (show !== true || exiting) return
    document.documentElement.style.overflow = "hidden"
    return () => {
      document.documentElement.style.overflow = ""
    }
  }, [show, exiting])

  // Cleanup timer
  useEffect(() => () => { if (timerRef.current) clearTimeout(timerRef.current) }, [])

  const handleEnter = useCallback(() => {
    try {
      localStorage.setItem(STORAGE_KEY, "1")
    } catch {}
    setExiting(true)
    // Reliable fallback — remove overlay after animation completes
    timerRef.current = setTimeout(() => {
      window.scrollTo({ top: 0, behavior: "instant" })
      setShow(false)
    }, 900)
  }, [])

  // Not determined yet or returning visitor — render nothing
  if (show !== true) return null

  return (
    <motion.div
      role="dialog"
      aria-modal="true"
      aria-label="DICE Portal"
      className="fixed inset-0 z-[100] flex items-center justify-center"
      style={{
        backgroundColor: "#0a0a0a",
        pointerEvents: exiting ? "none" : "auto",
      }}
      animate={{ opacity: exiting ? 0 : 1 }}
      transition={{ delay: exiting ? 0.4 : 0, duration: exiting ? 0.45 : 0, ease: "easeInOut" }}
    >
      {/* Grid texture */}
      <div className="bg-grid-fine absolute inset-0 opacity-30 pointer-events-none" />

      {/* Scanlines */}
      <div className="scanlines absolute inset-0 pointer-events-none" />

      {/* Radial glow behind logo */}
      <div
        className="absolute pointer-events-none"
        style={{
          width: "clamp(300px, 50vw, 600px)",
          height: "clamp(300px, 50vw, 600px)",
          borderRadius: "50%",
          background: "radial-gradient(circle, rgba(255,255,255,0.04) 0%, transparent 70%)",
        }}
      />

      {/* Content */}
      <div className="relative z-10 flex flex-col items-center gap-6 px-6">
        {/* Logo */}
        <motion.div
          initial={{ opacity: 0, y: 16 }}
          animate={{
            opacity: exiting ? 0 : 1,
            y: exiting ? -20 : 0,
            scale: exiting ? 0.92 : 1,
          }}
          transition={{
            delay: exiting ? 0.2 : 0.1,
            duration: exiting ? 0.3 : 0.5,
            ease: "easeOut",
          }}
        >
          <Logo size={140} showWordmark={false} className="text-white" />
        </motion.div>

        {/* Wordmark */}
        <motion.h1
          className="font-pixel text-[clamp(32px,6vw,52px)] tracking-tight text-white leading-none"
          initial={{ opacity: 0, y: 12 }}
          animate={{
            opacity: exiting ? 0 : 1,
            y: exiting ? -24 : 0,
          }}
          transition={{
            delay: exiting ? 0.15 : 0.3,
            duration: exiting ? 0.25 : 0.4,
            ease: "easeOut",
          }}
        >
          {BRAND.name}
        </motion.h1>

        {/* Tagline */}
        <motion.p
          className="font-mono text-[clamp(11px,1.8vw,14px)] text-neutral-500 text-center max-w-md"
          initial={{ opacity: 0, y: 8 }}
          animate={{
            opacity: exiting ? 0 : 1,
            y: exiting ? -16 : 0,
          }}
          transition={{
            delay: exiting ? 0.1 : 0.5,
            duration: exiting ? 0.2 : 0.35,
            ease: "easeOut",
          }}
        >
          {BRAND.tagline}
        </motion.p>

        {/* Enter button */}
        <motion.div
          className="mt-2"
          initial={{ opacity: 0, y: 8 }}
          animate={{
            opacity: exiting ? 0 : 1,
            y: exiting ? -10 : 0,
          }}
          transition={{
            delay: exiting ? 0 : 0.65,
            duration: exiting ? 0.15 : 0.3,
            ease: "easeOut",
          }}
        >
          <BracketButton
            onClick={handleEnter}
            className="text-sm px-5 py-2.5 border-neutral-600 bg-white text-black hover:bg-neutral-200 hover:text-black"
            aria-label="Enter the DICE portal"
            autoFocus
          >
            Enter_Portal
          </BracketButton>
        </motion.div>
      </div>
    </motion.div>
  )
}

export default PortalGate
