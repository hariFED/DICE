import type { Metadata } from "next"
import { DocsBreadcrumbs } from "@/components/docs/Breadcrumbs"
import { H1, H2, Lead } from "@/components/docs/Prose"

export const metadata: Metadata = {
  title: "Glossary",
  description: "A short reference for the terms you'll see throughout the DICE documentation.",
}

const TERMS: { term: string; short: string; body: string }[] = [
  { term: "VRF",           short: "verifiable random function",
    body: "A function that produces a random value and a proof that the value was generated correctly. Anyone can verify the proof; only the owner of a secret can produce it." },
  { term: "Commit",        short: "first half of commit-reveal",
    body: "A cryptographic hash of a secret value, published before the secret itself. Binding (the publisher can't change the secret later) and hiding (nobody else learns it yet)." },
  { term: "Reveal",        short: "second half of commit-reveal",
    body: "The secret value that matches a previous commit. Anyone can re-hash it and confirm the commit was honest." },
  { term: "Round",         short: "one VRF request",
    body: "The full lifecycle of a single randomness request: request → commit → reveal → finalize. A round takes about four seconds on mainnet." },
  { term: "Quorum",        short: "minimum honest nodes",
    body: "For a DICE round to succeed, at least 4 of 6 nodes must submit valid commits and reveals. One dishonest node is tolerated; the rest cancel it out." },
  { term: "Callback",      short: "your program, post-finalize",
    body: "The Solana instruction the DICE program CPIs into with the finalized randomness. You write this function to consume the 32-byte value." },
  { term: "TRNG",          short: "true RNG (hardware)",
    body: "A physical source of entropy — on the ESP32-S3, it samples thermal noise from an analog circuit. Not derivable from any prior state." },
  { term: "mTLS",          short: "mutual TLS",
    body: "A TLS handshake where both server and client present a certificate. DICE nodes authenticate to the coordinator using a client cert baked into their firmware." },
  { term: "Coordinator",   short: "off-chain relayer",
    body: "Our WebSocket server that routes messages between nodes and Solana. Stateless — it cannot mint randomness or sign, only relay." },
  { term: "Callback seed", short: "replay-protection tag",
    body: "A short arbitrary string you pick, embedded in each request. Prevents another program from replaying a randomness result against your callback." },
  { term: "ALT",           short: "address lookup table",
    body: "A Solana v0 transaction feature that compresses account lists. DICE uses ALTs to bundle commits + reveals into one transaction." },
  { term: "Streaming VRF", short: "continuous feed",
    body: "Instead of one-off requests, subscribe to a feed that emits a new random value every N seconds. Handy for games and auction ticks." },
]

export default function GlossaryPage() {
  return (
    <div className="max-w-[880px]">
      <DocsBreadcrumbs href="/docs/getting-started/glossary" />
      <H1>Glossary</H1>
      <Lead>Short definitions for the terms you&apos;ll bump into throughout these docs.</Lead>

      <div className="mt-8 border-t border-border">
        {TERMS.map((t) => (
          <div key={t.term} className="grid grid-cols-[140px_1fr] gap-5 py-5 border-b border-dashed border-border">
            <div>
              <p className="font-pixel text-foreground text-base">{t.term}</p>
              <p className="font-mono text-[10px] uppercase tracking-wider text-muted-foreground mt-1">{t.short}</p>
            </div>
            <div className="font-mono text-[13px] leading-[1.7] text-muted-foreground">
              {t.body}
            </div>
          </div>
        ))}
      </div>
    </div>
  )
}
