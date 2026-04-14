"use client"

import { useState } from "react"
import Link from "next/link"
import { usePathname } from "next/navigation"
import { motion, AnimatePresence } from "framer-motion"
import { NAV_LINKS, SITE } from "@/lib/constants"
import { cn } from "@/lib/utils"

export function Header() {
  const pathname = usePathname()
  const [mobileOpen, setMobileOpen] = useState(false)

  return (
    <motion.header
      initial={{ y: -20, opacity: 0 }}
      animate={{ y: 0, opacity: 1 }}
      transition={{ duration: 0.5, ease: "easeOut" }}
      className="sticky top-0 z-50 w-full bg-black/50 backdrop-blur-xl border-b border-white/[0.06]"
    >
      <div className="mx-auto flex h-16 max-w-7xl items-center justify-between px-4 sm:px-6 lg:px-8">
        {/* Logo */}
        <Link href="/" className="flex items-center gap-2 group">
          <span className="inline-block h-2 w-2 rounded-full bg-[#00FF85] group-hover:shadow-[0_0_8px_rgba(0,255,133,0.6)] transition-shadow duration-300" />
          <span className="text-xl font-bold text-metallic tracking-tight">
            {SITE.name}
          </span>
        </Link>

        {/* Desktop nav */}
        <nav className="hidden md:flex items-center gap-8">
          {NAV_LINKS.map((link) => {
            const isActive = pathname === link.href
            return (
              <Link
                key={link.href}
                href={link.href}
                className={cn(
                  "relative py-1 text-sm font-medium transition-colors duration-200",
                  isActive
                    ? "text-white"
                    : "text-zinc-400 hover:text-white"
                )}
              >
                {link.label}
                {isActive && (
                  <motion.span
                    layoutId="nav-underline"
                    className="absolute inset-x-0 -bottom-0.5 h-0.5 bg-[#00FF85] rounded-full"
                    transition={{ type: "spring", stiffness: 380, damping: 30 }}
                  />
                )}
              </Link>
            )
          })}
        </nav>

        {/* Desktop CTA */}
        <div className="hidden md:block">
          <Link
            href={SITE.launchAppUrl}
            className="inline-flex items-center justify-center rounded-lg border border-[#00FF85]/50 px-4 py-2 text-sm font-medium text-[#00FF85] transition-all duration-200 hover:bg-[#00FF85] hover:text-black hover:shadow-[0_0_20px_rgba(0,255,133,0.2)]"
          >
            Launch App
          </Link>
        </div>

        {/* Mobile hamburger */}
        <button
          type="button"
          className="md:hidden flex flex-col items-center justify-center gap-1.5 p-2"
          onClick={() => setMobileOpen((o) => !o)}
          aria-label="Toggle menu"
        >
          <span
            className={cn(
              "block h-0.5 w-5 bg-zinc-300 transition-all duration-300",
              mobileOpen && "translate-y-[4px] rotate-45"
            )}
          />
          <span
            className={cn(
              "block h-0.5 w-5 bg-zinc-300 transition-all duration-300",
              mobileOpen && "opacity-0"
            )}
          />
          <span
            className={cn(
              "block h-0.5 w-5 bg-zinc-300 transition-all duration-300",
              mobileOpen && "-translate-y-[4px] -rotate-45"
            )}
          />
        </button>
      </div>

      {/* Mobile menu */}
      <AnimatePresence>
        {mobileOpen && (
          <motion.div
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: "auto", opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            transition={{ duration: 0.25 }}
            className="md:hidden overflow-hidden border-t border-white/[0.06] bg-black/80 backdrop-blur-xl"
          >
            <nav className="flex flex-col gap-1 px-4 py-4">
              {NAV_LINKS.map((link) => {
                const isActive = pathname === link.href
                return (
                  <Link
                    key={link.href}
                    href={link.href}
                    onClick={() => setMobileOpen(false)}
                    className={cn(
                      "rounded-lg px-3 py-2 text-sm font-medium transition-colors",
                      isActive
                        ? "bg-white/[0.06] text-white"
                        : "text-zinc-400 hover:bg-white/[0.04] hover:text-white"
                    )}
                  >
                    {isActive && (
                      <span className="mr-2 inline-block h-1.5 w-1.5 rounded-full bg-[#00FF85]" />
                    )}
                    {link.label}
                  </Link>
                )
              })}
              <Link
                href={SITE.launchAppUrl}
                onClick={() => setMobileOpen(false)}
                className="mt-2 inline-flex items-center justify-center rounded-lg border border-[#00FF85]/50 px-4 py-2 text-sm font-medium text-[#00FF85] transition-all duration-200 hover:bg-[#00FF85] hover:text-black"
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
