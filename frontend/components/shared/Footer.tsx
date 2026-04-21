import Link from "next/link"
import { SOCIAL_LINKS, SITE } from "@/lib/constants"
import { TenziesGame } from "@/components/shared/TenziesGame"
import { Logo } from "@/components/shared/Logo"

export function Footer() {
  return (
    <footer className="border-t border-dashed border-border bg-background mt-0">
      <div className="mx-auto max-w-7xl px-4 py-12 sm:px-6 lg:px-8 font-mono text-sm">
        {/* ── Big DICE branding ────────────────────────────────────── */}
        <div className="flex flex-col items-center gap-3 mb-8">
          <Link href="/" className="flex items-center gap-3 group">
            <Logo size={36} showWordmark={false} className="text-foreground/80 group-hover:text-foreground transition-colors" />
            <span className="font-pixel text-3xl sm:text-4xl text-foreground tracking-[0.15em]">
              DICE
            </span>
            <span className="hidden sm:inline font-mono text-[9px] uppercase tracking-wider text-muted-foreground/50 border border-white/[0.08] rounded-full px-2 py-0.5 leading-none">
              v7.7
            </span>
          </Link>
          <p className="text-[11px] text-muted-foreground/50 font-mono text-center">
            {SITE.tagline}
          </p>
        </div>

        {/* ── Terminal exit banner ─────────────────────────────────── */}
        <pre
          aria-hidden
          className="text-muted-foreground/30 select-none mb-10 text-[10px] leading-tight overflow-hidden"
        >
{`╔══════════════════════════════════════════════════════════════════════════════════════╗
║   end · of · transmission                                                            ║
╚══════════════════════════════════════════════════════════════════════════════════════╝`}
        </pre>

        {/* ── Main 3-column grid: brand | game | links ────────────── */}
        <div className="grid grid-cols-1 lg:grid-cols-[1fr_1fr_1fr] gap-10 lg:gap-6 items-start">
          {/* ── Left column: Brand + Sitemap ──────────────────────── */}
          <div className="space-y-6">
            {/* Brand blurb */}
            <div>
              <p className="ascii-label mb-2 text-[10px]">from · dicelabs</p>
              <p className="text-xs text-muted-foreground max-w-xs leading-relaxed">
                Hardware-backed verifiable randomness for Solana.
                20 physical nodes, commit-reveal protocol, 0.002 SOL per request.
              </p>
            </div>

            {/* Sitemap */}
            <div>
              <p className="ascii-label mb-3 text-[10px]">// sitemap</p>
              <ul className="space-y-1.5 text-xs">
                {[
                  ["/", "index.html"],
                  ["/explorer", "explorer/"],
                  ["/docs", "docs/"],
                  ["/preorder", "preorder.sh"],
                ].map(([href, label]) => (
                  <li key={href}>
                    <Link
                      href={href}
                      className="text-muted-foreground hover:text-foreground transition-colors"
                    >
                      <span className="text-muted-foreground/60 mr-1.5">›</span>
                      {label}
                    </Link>
                  </li>
                ))}
              </ul>
            </div>
          </div>

          {/* ── Center column: Tenzies Game ───────────────────────── */}
          <div className="w-full flex justify-center">
            <TenziesGame />
          </div>

          {/* ── Right column: External + Protocol info ────────────── */}
          <div className="space-y-6 lg:text-right">
            {/* External links */}
            <div>
              <p className="ascii-label mb-3 text-[10px]">// external</p>
              <ul className="space-y-1.5 text-xs">
                <li>
                  <a
                    href={SOCIAL_LINKS.github}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-muted-foreground hover:text-foreground transition-colors"
                  >
                    <span className="text-muted-foreground/60 mr-1.5">›</span>
                    github.com/dicelabsnetwork
                  </a>
                </li>
                <li>
                  <a
                    href={SOCIAL_LINKS.twitter}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-muted-foreground hover:text-foreground transition-colors"
                  >
                    <span className="text-muted-foreground/60 mr-1.5">›</span>
                    x.com/dicelabsnetwork
                  </a>
                </li>
              </ul>
            </div>

            {/* Protocol info */}
            <div>
              <p className="ascii-label mb-3 text-[10px]">// protocol</p>
              <ul className="space-y-1.5 text-xs text-muted-foreground">
                <li>
                  <span className="text-muted-foreground/60 mr-1.5">›</span>
                  20 physical ESP32 nodes
                </li>
                <li>
                  <span className="text-muted-foreground/60 mr-1.5">›</span>
                  commit-reveal protocol
                </li>
                <li>
                  <span className="text-muted-foreground/60 mr-1.5">›</span>
                  0.002 SOL per request
                </li>
                <li>
                  <span className="text-muted-foreground/60 mr-1.5">›</span>
                  solana mainnet
                </li>
              </ul>
            </div>
          </div>
        </div>

        {/* ── Bottom row — terminal prompt ─────────────────────────── */}
        <div className="mt-10 pt-4 border-t border-dashed border-border flex items-center justify-between text-xs text-muted-foreground">
          <span>
            <span className="text-muted-foreground/60 mr-1">$</span>
            echo &quot;© {new Date().getFullYear()} DICE Network&quot;
          </span>
          <span className="hidden sm:inline">
            <span className="text-muted-foreground/60 mr-1">$</span>
            exit 0
          </span>
        </div>
      </div>
    </footer>
  )
}
