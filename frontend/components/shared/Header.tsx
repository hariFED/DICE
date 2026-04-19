"use client"

import { useState } from "react"
import Link from "next/link"
import { usePathname } from "next/navigation"
import { motion, AnimatePresence } from "framer-motion"
import { NAV_LINKS, SITE } from "@/lib/constants"
import { cn } from "@/lib/utils"
import { ThemeToggle } from "@/components/shared/ThemeToggle"
import { BracketLink } from "@/components/shared/BracketButton"
import { Logo } from "@/components/shared/Logo"

export function Header() {
  const pathname = usePathname()
  const [mobileOpen, setMobileOpen] = useState(false)

  return (
    <motion.header
      initial={{ y: -10, opacity: 0 }}
      animate={{ y: 0, opacity: 1 }}
      transition={{ duration: 0.4, ease: "easeOut" }}
      className="sticky top-0 z-50 w-full bg-background/85 backdrop-blur border-b border-dashed border-border"
    >
      <div className="mx-auto flex h-14 max-w-7xl items-center justify-between px-4 sm:px-6 lg:px-8">
        {/* Logo — iso cube + wordmark + version chip */}
        <Link href="/" className="flex items-center gap-2.5 group">
          <Logo size={26} />
          <span className="hidden sm:inline font-mono text-[10px] uppercase tracking-wider text-muted-foreground/70 border border-border rounded-sm px-1.5 py-0.5 leading-none">
            v7.7 · devnet
          </span>
        </Link>

        {/* Desktop nav — bracketed items */}
        <nav className="hidden md:flex items-center gap-1 font-mono text-xs uppercase tracking-wider">
          {NAV_LINKS.map((link) => {
            const isActive = pathname === link.href
            return (
              <Link
                key={link.href}
                href={link.href}
                className={cn(
                  "relative px-3 py-1 transition-colors",
                  isActive ? "text-foreground" : "text-muted-foreground hover:text-foreground"
                )}
              >
                <span className={cn("transition-opacity", isActive ? "opacity-100" : "opacity-0")}>›</span>
                <span className="px-1">{link.label}</span>
              </Link>
            )
          })}
        </nav>

        {/* Right cluster */}
        <div className="flex items-center gap-2">
          <ThemeToggle className="hidden sm:inline-flex" />
          <BracketLink href={SITE.launchAppUrl} variant="primary" className="hidden md:inline-flex">
            Launch_App
          </BracketLink>

          {/* Mobile hamburger */}
          <button
            type="button"
            className="md:hidden font-mono px-2 py-1 text-foreground"
            onClick={() => setMobileOpen((o) => !o)}
            aria-label="Toggle menu"
          >
            {mobileOpen ? "[ ✕ ]" : "[ ≡ ]"}
          </button>
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
            className="md:hidden overflow-hidden border-t border-dashed border-border bg-background"
          >
            <nav className="flex flex-col gap-0.5 px-4 py-3 font-mono text-sm">
              {NAV_LINKS.map((link) => {
                const isActive = pathname === link.href
                return (
                  <Link
                    key={link.href}
                    href={link.href}
                    onClick={() => setMobileOpen(false)}
                    className={cn(
                      "px-2 py-2 transition-colors",
                      isActive ? "text-foreground" : "text-muted-foreground hover:text-foreground"
                    )}
                  >
                    <span className="text-muted-foreground mr-2">{isActive ? "▸" : " "}</span>
                    {link.label}
                  </Link>
                )
              })}
              <div className="mt-3 flex items-center justify-between gap-3">
                <ThemeToggle />
                <BracketLink href={SITE.launchAppUrl} variant="primary">
                  Launch_App
                </BracketLink>
              </div>
            </nav>
          </motion.div>
        )}
      </AnimatePresence>
    </motion.header>
  )
}
