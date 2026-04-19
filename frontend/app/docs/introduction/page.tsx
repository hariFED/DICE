import type { Metadata } from "next"
import { DocsPageShell } from "@/components/docs/PageShell"
import { H2, H3, P, Code, Ul, Li } from "@/components/docs/Prose"
import { Callout } from "@/components/docs/Callout"

export const metadata: Metadata = {
  title: "Introduction",
  description:
    "What DICE is, who should use it, and why the trust model is different.",
}

const TOC = [
  { id: "thirty-second-pitch", label: "30-second pitch", level: 2 as const },
  { id: "who-is-this-for", label: "Who is this for?", level: 2 as const },
  { id: "trust-model", label: "Trust model", level: 2 as const },
  { id: "what-you-get", label: "What you get", level: 2 as const },
  { id: "what-this-is-not", label: "What DICE is not", level: 2 as const },
]

export default function IntroductionPage() {
  return (
    <DocsPageShell
      href="/docs/introduction"
      title="Introduction"
      lead="DICE is verifiable randomness for Solana, produced by a small fleet of physical devices running a commit-reveal protocol. One CPI call, one typed callback, and you're done."
      toc={TOC}
    >
      <H2 id="thirty-second-pitch">30-second pitch</H2>
      <P>
        Most VRF oracles answer a question you have to trust: <i>is the
        number really random?</i> They prove it with a cryptographic signature
        from a key held by an entity. If that key leaks — or that entity is
        compromised — the "randomness" stops being random.
      </P>
      <P>
        DICE answers the same question with a physical object. Every round,{" "}
        <Code>4</Code> to <Code>50</Code> real ESP32-S3 devices each sample
        their internal true-RNG peripheral, hash-commit their slice, and then
        reveal. The 32-byte output is the SHA-256 of all revealed entropy
        shares. Any single honest device makes the result uncontrollable by
        anyone else.
      </P>
      <Callout type="tip" title="The guarantee in one line">
        <p>
          1 of N honest devices → the output is unbiasable. You do not need
          to trust DICE the company, the coordinator, or the other devices.
        </p>
      </Callout>

      <H2 id="who-is-this-for">Who is this for?</H2>
      <P>
        Solana developers who need randomness inside a program. Typical
        callers:
      </P>
      <Ul>
        <Li>
          <b>Games</b> — dice, coin flips, card shuffles, loot drops, PvP
          matchmaking.
        </Li>
        <Li>
          <b>NFT launches</b> — fair mint-order reveal, rarity rolls,
          trait mixing.
        </Li>
        <Li>
          <b>Markets &amp; protocols</b> — lottery-style allocations,
          tie-breaking in auctions, rebate raffles, liquidation ordering.
        </Li>
        <Li>
          <b>Real-world-asset platforms</b> — regulated draws, audit-friendly
          decisions, compliance-grade provenance.
        </Li>
      </Ul>

      <H2 id="trust-model">Trust model</H2>
      <P>
        The short version: you trust <b>one physical device</b>, not a
        company. The longer version is three overlapping properties.
      </P>

      <H3 id="commits-before-reveals">1 &middot; Commits land before reveals</H3>
      <P>
        Every node publishes <Code>hash(entropy + nonce)</Code> as an on-chain
        commit before any node reveals its raw entropy. A node that tries to
        pick its share after seeing other reveals would have to break
        SHA-256.
      </P>

      <H3 id="aggregation-is-xor-hash">2 &middot; Aggregation mixes every share</H3>
      <P>
        The final 32 bytes are <Code>SHA-256(e₁ ‖ e₂ ‖ … ‖ eₙ)</Code>. Even
        a single honest share makes the output uncontrollable by all the
        others combined.
      </P>

      <H3 id="hardware-signed">3 &middot; Hardware signs the reveal</H3>
      <P>
        Each reveal is signed by a per-device ECDSA key baked into the
        firmware. Private keys never leave the chip. A compromised
        coordinator cannot forge reveals without physical access to the
        device.
      </P>

      <H2 id="what-you-get">What you get</H2>
      <Ul>
        <Li>
          <b>32 bytes</b> of uniform randomness delivered as a typed{" "}
          <Code>DiceResult</Code> struct.
        </Li>
        <Li>
          A <b>channel</b> PDA that binds your program as the callback target
          — nothing to re-wire per round.
        </Li>
        <Li>
          <b>Streaming</b> mode (~4.0 s avg) for UX-critical flows, or{" "}
          <b>audit</b> mode (+8%) when you need on-chain reproducible device
          selection.
        </Li>
        <Li>
          <b>Atomic payouts</b> — operators are paid in the same transaction
          that finalises your round.
        </Li>
      </Ul>

      <H2 id="what-this-is-not">What DICE is not</H2>
      <P>
        DICE is not a replacement for application-level fairness logic. If
        your game decides outcomes off-chain, DICE cannot help. And DICE
        does not run your game — you still write{" "}
        <Code>dice_callback</Code>, you still own the outcome formula, you
        still decide how to weight the bytes.
      </P>
      <Callout type="warn" title="Latency expectation">
        <p>
          Rounds take ~4 seconds end-to-end. This is fine for turn-based and
          deferred-reveal flows. It is <i>not</i> fine for latency-sensitive
          interactions like frame-by-frame procedural generation. Use a
          streaming feed if you need randomness at a higher rate — see{" "}
          <a href="/docs/advanced/streaming" className="text-[var(--foreground)] underline decoration-[var(--foreground)]/30 underline-offset-4">
            Streaming feed
          </a>
          .
        </p>
      </Callout>
    </DocsPageShell>
  )
}
