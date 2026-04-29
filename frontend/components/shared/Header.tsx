"use client"

import { useState } from "react"
import Link from "next/link"
import { usePathname } from "next/navigation"
import { motion, AnimatePresence } from "framer-motion"
import { NAV_LINKS } from "@/lib/constants"
import { cn } from "@/lib/utils"
import { Logo } from "@/components/shared/Logo"

/* ── Arrow-up-right icon ─────────────────────────────────────── */
function ArrowUpRight({ className = "" }: { className?: string }) {
  return (
    <svg
      className={className}
      width="20"
      height="20"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <path d="M7 17L17 7" />
      <path d="M7 7h10v10" />
    </svg>
  )
}

export function Header() {
  const pathname = usePathname()
  const [mobileOpen, setMobileOpen] = useState(false)

  return (
    <motion.header
      initial={{ y: -10, opacity: 0 }}
      animate={{ y: 0, opacity: 1 }}
      transition={{ duration: 0.4, ease: "easeOut" }}
      className="fixed top-6 left-0 right-0 z-50 px-8 lg:px-16"
    >
      <div className="mx-auto flex max-w-7xl items-center justify-between">
        {/* ── Left: Logo in liquid-glass circle ─────────────────── */}
        <Link
          href="/"
          className="liquid-glass flex h-12 w-12 items-center justify-center rounded-full shrink-0"
        >
          <Logo size={22} showWordmark={false} className="text-white" />
        </Link>

        {/* ── Center: Glass pill nav (desktop) ─────────────────── */}
        <nav className="hidden md:flex items-center liquid-glass rounded-full px-1.5 py-1.5">
          {NAV_LINKS.map((link) => {
            const isActive = pathname === link.href
            return (
              <Link
                key={link.href}
                href={link.href}
                className={cn(
                  "px-3 py-2 text-sm font-medium transition-colors duration-200 whitespace-nowrap",
                  isActive
                    ? "text-white"
                    : "text-white/70 hover:text-white/90"
                )}
              >
                {link.label}
              </Link>
            )
          })}

          {/* CTA button inside the pill */}
          <Link
            href="/preorder"
            className="ml-1 flex items-center gap-1.5 rounded-full bg-white px-4 py-2 text-sm font-medium text-black whitespace-nowrap transition-opacity duration-200 hover:opacity-90"
          >
            Pre-order
            <ArrowUpRight className="h-4 w-4" />
          </Link>
        </nav>

        {/* ── Right: Spacer to balance logo (desktop) / hamburger (mobile) ── */}
        <div className="hidden md:block h-12 w-12 shrink-0" />

        {/* Mobile hamburger */}
        <button
          type="button"
          className="md:hidden liquid-glass flex h-12 w-12 items-center justify-center rounded-full text-white/80 text-lg"
          onClick={() => setMobileOpen((o) => !o)}
          aria-label="Toggle menu"
        >
          {mobileOpen ? "✕" : "≡"}
        </button>
      </div>

      {/* ── Mobile menu ────────────────────────────────────────── */}
      <AnimatePresence>
        {mobileOpen && (
          <motion.div
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: "auto", opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            transition={{ duration: 0.2 }}
            className="md:hidden overflow-hidden mt-3 rounded-2xl liquid-glass"
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
                      "px-4 py-3 rounded-xl text-sm font-medium transition-colors duration-200",
                      isActive
                        ? "text-white bg-white/[0.08]"
                        : "text-white/60 hover:text-white/90 hover:bg-white/[0.04]"
                    )}
                  >
                    {link.label}
                  </Link>
                )
              })}
              <Link
                href="/preorder"
                onClick={() => setMobileOpen(false)}
                className="mt-2 flex items-center justify-center gap-2 rounded-full bg-white px-4 py-3 text-sm font-medium text-black"
              >
                Pre-order
                <ArrowUpRight className="h-4 w-4" />
              </Link>
            </nav>
          </motion.div>
        )}
      </AnimatePresence>
    </motion.header>
  )
}
