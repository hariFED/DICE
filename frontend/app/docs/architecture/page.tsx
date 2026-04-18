import type { Metadata } from "next"
import { DocsPageShell } from "@/components/docs/PageShell"
import { H2, H3, P, Code, Ul, Li } from "@/components/docs/Prose"
import { Callout } from "@/components/docs/Callout"

export const metadata: Metadata = {
  title: "Architecture",
  description:
    "How DICE produces 32 bytes of verifiable randomness: hardware, protocol, and on-chain program.",
}

const TOC = [
  { id: "three-layers", label: "Three layers", level: 2 as const },
  { id: "hardware", label: "The hardware layer", level: 2 as const },
  { id: "protocol", label: "The commit-reveal protocol", level: 2 as const },
  { id: "onchain", label: "The on-chain program", level: 2 as const },
  { id: "round-lifecycle", label: "A round, end to end", level: 2 as const },
]

export default function ArchitecturePage() {
  return (
    <DocsPageShell
      href="/docs/architecture"
      title="Architecture"
      lead="DICE has three moving pieces: a fleet of ESP32-S3 devices that produce entropy, an off-chain coordinator that shuttles messages, and a Solana program that settles each round atomically."
      toc={TOC}
    >
      <H2 id="three-layers">Three layers</H2>
      <P>
        Every round touches all three layers, in order: hardware generates
        entropy, protocol aggregates it safely, program verifies and
        finalises it. You only interact with layer three — but knowing what
        lives where explains why the guarantees work.
      </P>

      <div className="my-6 grid grid-cols-1 gap-3 sm:grid-cols-3">
        {[
          {
            tag: "1",
            name: "Hardware",
            where: "ESP32-S3 fleet",
            does: "RNG sampling, ECDSA signing, CBOR over USB serial.",
          },
          {
            tag: "2",
            name: "Coordinator",
            where: "Rust daemon",
            does: "Watches chain, requests commits, collects reveals, posts TXs.",
          },
          {
            tag: "3",
            name: "Program",
            where: "Solana · Anchor",
            does: "Verifies signatures, aggregates, pays operators, CPIs into you.",
          },
        ].map((l) => (
          <div
            key={l.tag}
            className="rounded-xl border border-white/[0.08] bg-white/[0.02] p-4"
          >
            <span className="flex h-6 w-6 items-center justify-center rounded-full border border-[#00FF85]/40 text-[11px] font-semibold text-[#00FF85]">
              {l.tag}
            </span>
            <p className="mt-3 text-[15px] font-semibold text-white">
              {l.name}
            </p>
            <p className="mt-0.5 text-[11px] uppercase tracking-[0.14em] text-zinc-500">
              {l.where}
            </p>
            <p className="mt-2 text-[13.5px] leading-[1.55] text-zinc-400">
              {l.does}
            </p>
          </div>
        ))}
      </div>

      <H2 id="hardware">The hardware layer</H2>
      <P>
        Every DICE node is an ESP32-S3 running our firmware. The chip has a
        hardware true-RNG peripheral that samples from analog noise in the
        radio front-end. It also has a small persistent flash region that
        stores one thing: the device's ECDSA private key, written at
        provisioning and never exported.
      </P>

      <H3 id="what-a-node-exposes">What a node exposes</H3>
      <Ul>
        <Li>
          <b>Device ID</b> — deterministic:{" "}
          <Code>SHA-256(device_pubkey)</Code>. The program verifies this on
          every reveal so a device can't pretend to be another.
        </Li>
        <Li>
          <b>Round API</b> — over USB serial, CBOR-framed. The coordinator
          sends a round nonce; the device returns <Code>commit</Code>, then
          later <Code>reveal + signature</Code>.
        </Li>
        <Li>
          <b>Signed payloads</b> — every message crossing the USB boundary
          is signed. The private key never leaves the chip.
        </Li>
      </Ul>

      <Callout type="info" title="Why ESP32-S3 specifically">
        <p>
          Low unit cost, deterministic behaviour, a RTC that survives power
          cycles, and a hardware RNG that independent audits have blessed
          for cryptographic use. We use a subset of the chip — no Wi-Fi, no
          Bluetooth — so the attack surface is small.
        </p>
      </Callout>

      <H2 id="protocol">The commit-reveal protocol</H2>
      <P>
        A naive oracle picks entropy <i>after</i> seeing the request, which
        lets it bias the output. DICE picks entropy <i>before</i> anyone
        sees it, using a two-phase protocol that's been well-understood
        since Rabin.
      </P>

      <H3 id="phase-1">Phase 1 &middot; Commit</H3>
      <P>
        Each selected node generates a local random{" "}
        <Code>entropy_i ∈ {"{0,1}"}^256</Code> plus a round nonce, then
        computes <Code>commit_i = SHA-256(entropy_i ‖ nonce)</Code>. The
        coordinator posts all <Code>commit_i</Code> values to chain in a
        single <Code>submit_round</Code> transaction. At this point the
        entropy is locked — any change would break the pre-image.
      </P>

      <H3 id="phase-2">Phase 2 &middot; Reveal</H3>
      <P>
        After commits land, each node releases{" "}
        <Code>reveal_i = entropy_i</Code> along with its ECDSA signature
        over the reveal. The coordinator submits all reveals in one{" "}
        <Code>submit_reveal</Code> transaction. The program verifies that{" "}
        <Code>SHA-256(reveal_i ‖ nonce) == commit_i</Code> and that the
        signature matches the device's registered pubkey.
      </P>

      <H3 id="phase-3">Phase 3 &middot; Aggregate</H3>
      <P>
        The final randomness is{" "}
        <Code>randomness = SHA-256(reveal_1 ‖ reveal_2 ‖ … ‖ reveal_n)</Code>.
        One honest reveal makes the output uncontrollable by everyone else.
        The program stores this 32-byte value on the channel and CPIs your
        callback with it.
      </P>

      <Callout type="tip" title="Why SHA-256, not XOR?">
        <p>
          XOR is algebraically fragile: a last-mover with knowledge of all
          other shares can grind to a target output. SHA-256 breaks that —
          the adversary would need to find pre-images.
        </p>
      </Callout>

      <H2 id="onchain">The on-chain program</H2>
      <P>
        The DICE program (<Code>FMwPuCjkfZXN2MuNJQiUzZC3hnxHcD8mrTuntsqA84XD</Code>)
        is an Anchor program with four families of instructions:
      </P>
      <Ul>
        <Li>
          <b>Registration</b> — device enrolment, channel creation, coordinator
          binding.
        </Li>
        <Li>
          <b>Rounds</b> — <Code>request_randomness_auto</Code>,{" "}
          <Code>submit_round</Code>, <Code>submit_reveal</Code>, finalize.
        </Li>
        <Li>
          <b>Callback</b> — <Code>deliver_callback</Code> CPIs into your
          program with the <Code>DiceResult</Code> payload.
        </Li>
        <Li>
          <b>Payouts</b> — <Code>claim_rewards_v2</Code> credits node vaults
          from the round escrow; 70/20/10 split.
        </Li>
      </Ul>

      <P>
        Every state transition is an on-chain event, so a full history of
        commits, reveals, and signatures is publicly reproducible. See{" "}
        <a
          href="/docs/advanced/verify"
          className="text-[#00FF85] underline decoration-[#00FF85]/30 underline-offset-4"
        >
          Verifying randomness
        </a>{" "}
        for the recipe.
      </P>

      <H2 id="round-lifecycle">A round, end to end</H2>
      <P>
        Here's the wall-clock view — numbers taken from a 50-round devnet
        bench on real hardware, v7.7 streaming:
      </P>
      <div className="my-5 overflow-x-auto rounded-xl border border-white/[0.08]">
        <table className="w-full text-left text-[14px]">
          <thead className="bg-white/[0.03] text-zinc-300">
            <tr>
              <th className="px-3 py-2 font-medium">t</th>
              <th className="px-3 py-2 font-medium">Layer</th>
              <th className="px-3 py-2 font-medium">Event</th>
            </tr>
          </thead>
          <tbody className="text-zinc-400">
            <tr className="border-t border-white/[0.06]">
              <td className="px-3 py-2 font-mono text-[#7cf9c1]">0 ms</td>
              <td className="px-3 py-2">Your program</td>
              <td className="px-3 py-2">
                <Code>request_randomness_auto</Code> lands. Round is open.
              </td>
            </tr>
            <tr className="border-t border-white/[0.04]">
              <td className="px-3 py-2 font-mono text-[#7cf9c1]">~400 ms</td>
              <td className="px-3 py-2">Coordinator → devices</td>
              <td className="px-3 py-2">4 nodes selected, commit request broadcast.</td>
            </tr>
            <tr className="border-t border-white/[0.04]">
              <td className="px-3 py-2 font-mono text-[#7cf9c1]">~1.5 s</td>
              <td className="px-3 py-2">Chain</td>
              <td className="px-3 py-2">
                <Code>submit_round</Code> lands with all 4 commits.
              </td>
            </tr>
            <tr className="border-t border-white/[0.04]">
              <td className="px-3 py-2 font-mono text-[#7cf9c1]">~3.0 s</td>
              <td className="px-3 py-2">Chain</td>
              <td className="px-3 py-2">
                <Code>submit_reveal</Code> lands with signatures.
              </td>
            </tr>
            <tr className="border-t border-white/[0.04]">
              <td className="px-3 py-2 font-mono text-[#7cf9c1]">~4.0 s</td>
              <td className="px-3 py-2">Chain</td>
              <td className="px-3 py-2">
                <Code>deliver_callback</Code> CPIs your{" "}
                <Code>dice_callback</Code>. Round finalized. Operators paid.
              </td>
            </tr>
          </tbody>
        </table>
      </div>
      <P className="text-[13.5px]">
        From the caller's perspective: submit TX, wait 4 seconds, read your
        settled game state. Streaming mode: avg 4.05 s, p50 3.99 s, p95 4.62
        s over 50 rounds, 98% pass rate.
      </P>
    </DocsPageShell>
  )
}
