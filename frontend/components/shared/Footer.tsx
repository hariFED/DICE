import Link from "next/link"
import { SOCIAL_LINKS, SITE } from "@/lib/constants"

export function Footer() {
  return (
    <footer className="border-t border-dashed border-border bg-background mt-24">
      <div className="mx-auto max-w-7xl px-4 py-12 sm:px-6 lg:px-8 font-mono text-sm">
        {/* Terminal exit banner */}
        <pre aria-hidden className="text-muted-foreground/40 select-none mb-10 text-[10px] leading-tight overflow-hidden">
{`╔══════════════════════════════════════════════════════════════════════════════════════╗
║   end · of · transmission                                                            ║
╚══════════════════════════════════════════════════════════════════════════════════════╝`}
        </pre>

        <div className="grid grid-cols-1 sm:grid-cols-3 gap-10">
          {/* Brand */}
          <div>
            <Link href="/" className="inline-flex items-center gap-1">
              <span className="text-muted-foreground">[</span>
              <span className="text-muted-foreground/50">▣</span>
              <span className="font-semibold text-foreground tracking-wider px-1">{SITE.name}</span>
              <span className="text-muted-foreground/60 text-[11px]">v7.7</span>
              <span className="text-muted-foreground">]</span>
            </Link>
            <p className="mt-3 text-xs text-muted-foreground max-w-xs leading-relaxed">
              {SITE.tagline}
            </p>
            <p className="mt-2 ascii-label text-[10px]">from · dicelabs</p>
          </div>

          {/* Sitemap */}
          <div>
            <p className="ascii-label mb-3">// sitemap</p>
            <ul className="space-y-1.5 text-xs">
              {[
                ["/", "index.html"],
                ["/explorer", "explorer/"],
                ["/docs", "docs/"],
                ["/preorder", "preorder.sh"],
              ].map(([href, label]) => (
                <li key={href}>
                  <Link href={href} className="text-muted-foreground hover:text-foreground transition-colors">
                    <span className="text-muted-foreground/60 mr-1.5">›</span>{label}
                  </Link>
                </li>
              ))}
            </ul>
          </div>

          {/* External */}
          <div>
            <p className="ascii-label mb-3">// external</p>
            <ul className="space-y-1.5 text-xs">
              <li>
                <a href={SOCIAL_LINKS.github} target="_blank" rel="noopener noreferrer" className="text-muted-foreground hover:text-foreground transition-colors">
                  <span className="text-muted-foreground/60 mr-1.5">›</span>github.com/dicelabsnetwork
                </a>
              </li>
              <li>
                <a href={SOCIAL_LINKS.twitter} target="_blank" rel="noopener noreferrer" className="text-muted-foreground hover:text-foreground transition-colors">
                  <span className="text-muted-foreground/60 mr-1.5">›</span>x.com/dicelabsnetwork
                </a>
              </li>
            </ul>
          </div>
        </div>

        {/* Bottom row — terminal prompt */}
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
