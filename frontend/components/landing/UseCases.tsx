"use client"

/**
 * Use-case cards — 3-tile "what you can build" section.
 * Editorial pacing; each tile has a pixel numeral, title, mini diagram,
 * body. Tiles have a hover state that inverts.
 */

type Case = {
  n: string
  tag: string
  title: string
  body: string
  icon: React.ReactNode
}

const CASES: Case[] = [
  {
    n: "A",
    tag: "gaming",
    title: "Provable game outcomes",
    body: "Shuffle a deck, roll dice, pick a raid boss. The result is traceable to the hardware RNG that produced it.",
    icon: (
      <svg viewBox="0 0 80 80" className="w-full h-full" style={{ color: "currentColor" }}>
        <rect x="14" y="14" width="52" height="52" rx="4" fill="none" stroke="currentColor" strokeWidth="1.5" />
        <circle cx="28" cy="28" r="4" fill="currentColor" />
        <circle cx="52" cy="28" r="4" fill="currentColor" />
        <circle cx="28" cy="52" r="4" fill="currentColor" />
        <circle cx="52" cy="52" r="4" fill="currentColor" />
        <circle cx="40" cy="40" r="4" fill="currentColor" />
      </svg>
    ),
  },
  {
    n: "B",
    tag: "defi",
    title: "Lottery + yield raffles",
    body: "Weekly prize pool draws, airdrop lotteries, sweepstakes. One request, one winner, one on-chain signature.",
    icon: (
      <svg viewBox="0 0 80 80" className="w-full h-full" style={{ color: "currentColor" }}>
        <circle cx="40" cy="40" r="24" fill="none" stroke="currentColor" strokeWidth="1.5" />
        <path d="M40 16 L40 64 M16 40 L64 40 M24 24 L56 56 M56 24 L24 56" stroke="currentColor" strokeWidth="0.6" opacity="0.4" />
        <circle cx="40" cy="40" r="5" fill="currentColor" />
      </svg>
    ),
  },
  {
    n: "C",
    tag: "nft",
    title: "Trait reveals + drops",
    body: "Pick the mint order. Assign rarity. Randomize reveal metadata. Each step provably unbiased.",
    icon: (
      <svg viewBox="0 0 80 80" className="w-full h-full" style={{ color: "currentColor" }}>
        <polygon points="40,10 64,24 64,56 40,70 16,56 16,24" fill="none" stroke="currentColor" strokeWidth="1.5" />
        <polygon points="40,22 54,30 54,50 40,58 26,50 26,30" fill="currentColor" opacity="0.15" stroke="currentColor" strokeWidth="0.8" />
        <circle cx="40" cy="40" r="3" fill="currentColor" />
      </svg>
    ),
  },
]

export function UseCases() {
  return (
    <section className="relative border-b border-border">
      <div className="mx-auto w-full max-w-[1400px] px-4 sm:px-6 lg:px-8 py-20 md:py-24">
        <div className="grid grid-cols-1 lg:grid-cols-12 gap-8 mb-12">
          <div className="lg:col-span-3">
            <span className="chapter-num">06</span>
            <p className="mt-2 font-mono text-[11px] uppercase tracking-[0.2em] text-muted-foreground">
              use · cases
            </p>
          </div>
          <div className="lg:col-span-6 lg:col-start-5">
            <h2 className="font-sans text-3xl md:text-4xl lg:text-5xl tracking-[-0.02em] leading-[1.05] text-foreground">
              What you can <span className="italic font-light text-muted-foreground">build.</span>
            </h2>
          </div>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          {CASES.map((c, i) => (
            <div
              key={c.n}
              className="group relative bg-grid border border-border p-6 md:p-8 overflow-hidden transition-colors hover:bg-foreground hover:text-background"
            >
              <div className="flex items-start justify-between mb-6">
                <p className="font-pixel text-[36px] leading-none">{c.n}</p>
                <span className="font-mono text-[10px] uppercase tracking-wider opacity-50">
                  0{i + 1} / 03 · {c.tag}
                </span>
              </div>

              <div className="w-20 h-20 mb-6">{c.icon}</div>

              <h3 className="font-sans text-xl md:text-2xl tracking-[-0.01em] mb-3">
                {c.title}
              </h3>
              <p className="font-mono text-[13px] leading-relaxed opacity-80">
                {c.body}
              </p>
            </div>
          ))}
        </div>
      </div>
    </section>
  )
}

export default UseCases
