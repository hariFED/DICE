"use client"

import { useEffect, useRef } from "react"
import {
  useInView,
  useMotionValue,
  useTransform,
  motion,
  animate,
} from "framer-motion"
import { cn } from "@/lib/utils"

interface AnimatedCounterProps {
  value: number
  suffix?: string
  prefix?: string
  decimals?: number
  className?: string
}

export function AnimatedCounter({
  value,
  suffix = "",
  prefix = "",
  decimals = 0,
  className,
}: AnimatedCounterProps) {
  const ref = useRef<HTMLSpanElement>(null)
  const isInView = useInView(ref, { once: true, margin: "-50px" })
  const motionValue = useMotionValue(0)
  const rounded = useTransform(motionValue, (latest) => {
    const formatted = latest.toFixed(decimals)
    // Add thousand separators
    const parts = formatted.split(".")
    parts[0] = parts[0].replace(/\B(?=(\d{3})+(?!\d))/g, ",")
    return `${prefix}${parts.join(".")}${suffix}`
  })

  useEffect(() => {
    if (!isInView) return

    const controls = animate(motionValue, value, {
      duration: 2,
      ease: "easeOut",
    })

    return () => controls.stop()
  }, [isInView, motionValue, value])

  return (
    <motion.span
      ref={ref}
      className={cn("text-4xl font-bold text-white tabular-nums", className)}
    >
      {rounded}
    </motion.span>
  )
}
