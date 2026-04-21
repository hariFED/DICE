"use client"

import { useState } from "react"
import Link from "next/link"
import { usePathname } from "next/navigation"
import { motion, AnimatePresence } from "framer-motion"
import { NAV_LINKS, SITE } from "@/lib/constants"
import { cn } from "@/lib/utils"
import { Logo } from "@/components/shared/Logo"

const glass = [
  "bg-white/[0.04] backdrop-blur-2xl",
  "border border-white/[0.12]",
  "shadow-[0_8px_32px_rgba(0,0,0,0.4),0_0_0_1px_rgba(255,255,255,0.04),inset_0_1px_0_rgba(255,255,255,0.12),inset_0_-1px_0_rgba(255,255,255,0.04)]",
  "before:absolute before:inset-0 before:rounded-[inherit] before:bg-gradient-to-b before:from-white/[0.08] before:to-transparent before:to-50% before:pointer-events-none",
  "after:absolute after:inset-x-4 after:-bottom-2 after:h-4 after:rounded-full after:bg-white/[0.02] after:blur-xl after:pointer-events-none",
  "relative overflow-visible",
].join(" ")

const glassBtn = [
  "bg-white/[0.08] backdrop-blur-md",
  "border border-white/[0.15]",
  "shadow-[inset_0_1px_0_rgba(255,255,255,0.15),0_2px_8px_rgba(0,0,0,0.2)]",
  "hover:bg-white/[0.16] hover:border-white/[0.25] hover:shadow-[inset_0_1px_0_rgba(255,255,255,0.2),0_4px_16px_rgba(0,0,0,0.3)]",
  "transition-all duration-200",
].join(" ")

export function Header() {
  const pathname = usePathname()
  const [mobileOpen, setMobileOpen] = useState(false)

  return (
    <motion.header
      initial={{ y: -10, opacity: 0 }}
      animate={{ y: 0, opacity: 1 }}
      transition={{ duration: 0.4, ease: "easeOut" }}
      className="sticky top-0 z-50 w-full"
    >
      {/* Glass navbar container with margin and rounding */}
      <div className={cn("mx-3 sm:mx-6 mt-3 rounded-[20px] px-4 sm:px-5 py-0.5", glass)}>
        <div className="mx-auto flex h-12 max-w-7xl items-center justify-between">
          {/* Logo */}
          <Link href="/" className="flex items-center gap-2.5 group">
            <Logo size={24} showWordmark={false} className="text-white" />
            <span className="font-pixel text-[13px] tracking-tight text-white/90 leading-none">
              {SITE.name}
            </span>
            <span className="hidden sm:inline font-mono text-[9px] uppercase tracking-wider text-white/30 border border-white/[0.08] rounded-full px-2 py-0.5 leading-none">
              v7.7
            </span>
          </Link>

          {/* Desktop nav */}
          <nav className="hidden md:flex items-center gap-1">
            {NAV_LINKS.map((link) => {
              const isActive = pathname === link.href
              return (
                <Link
                  key={link.href}
                  href={link.href}
                  className={cn(
                    "relative px-3.5 py-1.5 rounded-lg font-mono text-[11px] uppercase tracking-wider transition-all duration-200",
                    isActive
                      ? "text-white bg-white/[0.12] border border-white/[0.18] shadow-[inset_0_1px_0_rgba(255,255,255,0.15),0_2px_8px_rgba(0,0,0,0.2)]"
                      : "text-white/50 hover:text-white/80 hover:bg-white/[0.08] border border-transparent"
                  )}
                >
                  {link.label}
                </Link>
              )
            })}
          </nav>

          {/* Right cluster */}
          <div className="flex items-center gap-2">
            <Link
              href={SITE.launchAppUrl}
              className={cn(
                "hidden md:inline-flex items-center gap-2 rounded-lg px-4 py-1.5 font-mono text-[11px] uppercase tracking-wider text-white/90",
                glassBtn
              )}
            >
              Launch App
            </Link>

            {/* Mobile hamburger */}
            <button
              type="button"
              className={cn(
                "md:hidden rounded-lg px-2.5 py-1.5 font-mono text-xs text-white/80",
                glassBtn
              )}
              onClick={() => setMobileOpen((o) => !o)}
              aria-label="Toggle menu"
            >
              {mobileOpen ? "✕" : "≡"}
            </button>
          </div>
        </div>
      </div>

      {/* Mobile menu */}
      <AnimatePresence>
        {mobileOpen && (
          <motion.div
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: "auto", opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            transition={{ duration: 0.2 }}
            className={cn(
              "md:hidden overflow-hidden mx-3 sm:mx-6 mt-1.5 rounded-xl",
              glass
            )}
          >
            <nav className="flex flex-col gap-0.5 px-3 py-3">
              {NAV_LINKS.map((link) => {
                const isActive = pathname === link.href
                return (
                  <Link
                    key={link.href}
                    href={link.href}
                    onClick={() => setMobileOpen(false)}
                    className={cn(
                      "px-3 py-2.5 rounded-lg font-mono text-sm transition-all duration-200",
                      isActive
                        ? "text-white bg-white/[0.08]"
                        : "text-white/50 hover:text-white/80 hover:bg-white/[0.04]"
                    )}
                  >
                    {link.label}
                  </Link>
                )
              })}
              <Link
                href={SITE.launchAppUrl}
                onClick={() => setMobileOpen(false)}
                className={cn(
                  "mt-2 text-center rounded-lg px-4 py-2.5 font-mono text-sm text-white/90",
                  glassBtn
                )}
              >
                Launch App
              </Link>
            </nav>
          </motion.div>
        )}
      </AnimatePresence>
    </motion.header>
  )
}
