"use client"

import { motion } from "framer-motion"
import type { ReactNode } from "react"

/**
 * Wraps page content with a smooth fade + subtle upward slide on mount.
 * Used inside template.tsx files so it re-runs on every navigation.
 */
export function PageTransition({ children }: { children: ReactNode }) {
  return (
    <motion.div
      initial={{ opacity: 0, y: 8 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.3, ease: [0.25, 0.4, 0.25, 1] }}
    >
      {children}
    </motion.div>
  )
}
