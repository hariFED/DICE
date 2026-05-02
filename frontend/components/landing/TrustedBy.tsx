"use client"

const ITEMS = [
  "solana",
  "anchor",
  "esp32-s3",
  "secp256k1",
  "sha-256",
  "rust",
  "typescript",
  "tokio",
  "rustls",
]

export function TrustedBy() {
  const duplicated = [...ITEMS, ...ITEMS, ...ITEMS]

  return (
    <section className="py-12 border-b border-border overflow-hidden">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 mb-6">
        <p className="ascii-label">stack · built · on</p>
      </div>

      <div className="relative">
        {/* Edge fade — solid color stops in light + dark */}
        <div className="absolute left-0 top-0 bottom-0 w-24 z-10 pointer-events-none" style={{ background: "linear-gradient(to right, var(--background), transparent)" }} />
        <div className="absolute right-0 top-0 bottom-0 w-24 z-10 pointer-events-none" style={{ background: "linear-gradient(to left, var(--background), transparent)" }} />

        <div className="flex">
          <div className="flex gap-3 shrink-0" style={{ animation: "marquee 40s linear infinite" }}>
            {duplicated.map((item, i) => (
              <span
                key={`${item}-${i}`}
                className="shrink-0 font-mono text-xs uppercase tracking-wider text-muted-foreground border border-border px-4 py-1.5 whitespace-nowrap"
              >
                <span className="text-muted-foreground/50 mr-1">›</span>{item}
              </span>
            ))}
          </div>
          <div className="flex gap-3 shrink-0" style={{ animation: "marquee 40s linear infinite" }} aria-hidden>
            {duplicated.map((item, i) => (
              <span
                key={`dup-${item}-${i}`}
                className="shrink-0 font-mono text-xs uppercase tracking-wider text-muted-foreground border border-border px-4 py-1.5 whitespace-nowrap"
              >
                <span className="text-muted-foreground/50 mr-1">›</span>{item}
              </span>
            ))}
          </div>
        </div>
      </div>

      {/* keyframes shipped via globals.css's @keyframes marquee — verify it's still there. */}
      <style jsx>{`
        @keyframes marquee {
          from { transform: translateX(0); }
          to { transform: translateX(-100%); }
        }
      `}</style>
    </section>
  )
}

export default TrustedBy
