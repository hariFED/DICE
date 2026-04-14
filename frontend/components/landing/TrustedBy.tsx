"use client"

const ITEMS = [
  "Solana",
  "Anchor",
  "ESP32-S3",
  "secp256k1",
  "SHA-256",
  "Rust",
  "TypeScript",
]

export function TrustedBy() {
  // Duplicate items for seamless infinite scroll
  const duplicated = [...ITEMS, ...ITEMS]

  return (
    <section className="py-16 border-y border-white/[0.04] overflow-hidden">
      <p className="text-xs uppercase tracking-[0.2em] text-zinc-600 text-center mb-8">
        Built On
      </p>

      <div className="relative">
        {/* Fade edges */}
        <div className="absolute left-0 top-0 bottom-0 w-24 z-10 bg-gradient-to-r from-black to-transparent pointer-events-none" />
        <div className="absolute right-0 top-0 bottom-0 w-24 z-10 bg-gradient-to-l from-black to-transparent pointer-events-none" />

        {/* Marquee track */}
        <div className="flex">
          <div
            className="flex gap-4 shrink-0"
            style={{
              animation: "marquee 30s linear infinite",
            }}
          >
            {duplicated.map((item, i) => (
              <span
                key={`${item}-${i}`}
                className="shrink-0 border border-white/[0.06] bg-white/[0.02] rounded-full px-6 py-2 text-sm text-zinc-400 whitespace-nowrap"
              >
                {item}
              </span>
            ))}
          </div>
          {/* Second copy for seamless loop */}
          <div
            className="flex gap-4 shrink-0"
            style={{
              animation: "marquee 30s linear infinite",
            }}
          >
            {duplicated.map((item, i) => (
              <span
                key={`dup-${item}-${i}`}
                className="shrink-0 border border-white/[0.06] bg-white/[0.02] rounded-full px-6 py-2 text-sm text-zinc-400 whitespace-nowrap"
              >
                {item}
              </span>
            ))}
          </div>
        </div>
      </div>

    </section>
  )
}

export default TrustedBy
