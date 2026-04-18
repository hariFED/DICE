import type { Metadata } from "next"
import { DocsPageShell } from "@/components/docs/PageShell"
import { H2, P, Code } from "@/components/docs/Prose"

export const metadata: Metadata = {
  title: "FAQ",
  description:
    "Answers to the questions developers ask most often before shipping.",
}

const TOC = [
  { id: "test-without-hardware", label: "Can I test without hardware?", level: 2 as const },
  { id: "callback-panic", label: "What if my callback panics?", level: 2 as const },
  { id: "provably-unbiased", label: "How do I make it provably unbiased?", level: 2 as const },
  { id: "evm-compat", label: "Is it compatible with my EVM port?", level: 2 as const },
  { id: "mainnet", label: "Is DICE on mainnet?", level: 2 as const },
  { id: "downtime", label: "What if the coordinator goes down?", level: 2 as const },
  { id: "multiple-callbacks", label: "Can one program have multiple callbacks?", level: 2 as const },
  { id: "cost-to-operate", label: "What does it cost to run a node?", level: 2 as const },
]

export default function FaqPage() {
  return (
    <DocsPageShell
      href="/docs/faq"
      title="Frequently asked questions"
      lead="Things developers ask most often — in order of how often."
      toc={TOC}
    >
      <H2 id="test-without-hardware">Can I test without ESP32-S3 hardware?</H2>
      <P>
        Yes. <Code>tests/harness/mock_firmware_node</Code> in the DICE repo
        simulates 10 devices running the real CBOR protocol and ECDSA
        signing. It runs on your laptop, talks to your local validator, and
        exercises every code path the real fleet does. Use it for CI and
        for local development. When you deploy to devnet, point your
        channel at the public DICE devnet coordinator — no hardware
        changes on your side.
      </P>

      <H2 id="callback-panic">What if my <Code>dice_callback</Code> panics?</H2>
      <P>
        The randomness isn't lost. It landed on the{" "}
        <Code>DiceChannel</Code> account in a prior transaction — still on
        chain, still signed by the hardware, still publicly readable. Two
        ways to recover:
      </P>
      <P>
        1. Ship a fixed callback in a program upgrade and call{" "}
        <Code>deliver_callback</Code> again with the stored{" "}
        <Code>round_id</Code>. DICE re-CPIs you with the exact same bytes,
        so the replayed handler sees deterministic input.
      </P>
      <P>
        2. If the panic was a transient account state issue (e.g. your
        game PDA wasn't initialised yet), fix the precondition and retry —
        same result.
      </P>

      <H2 id="provably-unbiased">How do I make it provably unbiased?</H2>
      <P>
        Three properties already hold:
      </P>
      <P>
        <b>1.</b> Any one honest device in the N-of-N set makes the output
        uncontrollable by all others combined —{" "}
        <Code>SHA-256</Code> aggregation, not XOR.
      </P>
      <P>
        <b>2.</b> Commits land on chain before any reveal is published —
        no node can pick its share after seeing others.
      </P>
      <P>
        <b>3.</b> Every reveal is signed by hardware — software-only
        impersonation is not possible.
      </P>
      <P>
        For the strongest version of the guarantee, switch to{" "}
        <a
          href="/docs/reference/modes"
          className="text-[#00FF85] underline decoration-[#00FF85]/30 underline-offset-4"
        >
          audit mode
        </a>
        . It adds on-chain device selection so the whole round is
        reproducible from chain data.
      </P>

      <H2 id="evm-compat">Is the randomness compatible with my EVM port?</H2>
      <P>
        Yes. DICE emits a 32-byte <Code>uint256</Code>-sized randomness in
        the same shape Chainlink VRF and Pyth Entropy use on EVM chains.
        If you're porting a game from Ethereum, the outcome formula is
        usually a copy-paste — the only thing that changes is the callback
        signature (EVM uses a function selector + ABI-encoded args; Solana
        uses an Anchor discriminator + Borsh).
      </P>

      <H2 id="mainnet">Is DICE on mainnet?</H2>
      <P>
        The program is deployed and audited on devnet with the IDs listed
        on the{" "}
        <a
          href="/docs"
          className="text-[#00FF85] underline decoration-[#00FF85]/30 underline-offset-4"
        >
          overview page
        </a>
        . Mainnet rollout is gated on fleet size — we aim for 20+ physical
        devices online before flipping the switch. Track the current
        status on the{" "}
        <a
          href="/explorer"
          className="text-[#00FF85] underline decoration-[#00FF85]/30 underline-offset-4"
        >
          explorer
        </a>
        .
      </P>

      <H2 id="downtime">What happens if the coordinator goes down?</H2>
      <P>
        Your caller experiences round timeouts — <Code>RoundTimedOut</Code>{" "}
        after the configured window. The channel balance is not debited on
        a timed-out round, so you don't pay for a round that didn't
        finish. New requests will queue and resume when the coordinator
        comes back up.
      </P>
      <P>
        The protocol supports coordinator rotation: the channel stores a
        single coordinator pubkey that can be re-bound by the authority.
        We run redundant coordinators in production.
      </P>

      <H2 id="multiple-callbacks">Can one program have multiple callbacks?</H2>
      <P>
        Yes. The callback method name is always <Code>dice_callback</Code>{" "}
        (because the discriminator is baked into DICE's CPI), but you can
        have multiple channels per program, each with its own{" "}
        <Code>channel_index</Code>, and dispatch inside{" "}
        <Code>dice_callback</Code> based on <Code>result.channel_key</Code>.
        Use this pattern to split low-stakes and high-stakes flows, or to
        isolate different games inside a single program binary.
      </P>

      <H2 id="cost-to-operate">What does it cost to run a node?</H2>
      <P>
        The ESP32-S3 dev board is about $10 at bulk; our production
        housing adds ~$40. A node needs a USB host, which is any Linux
        server with a spare port — a Raspberry Pi 4 works. The coordinator
        talks to the node over USB serial only; no public IP required.
      </P>
      <P>
        Operator economics: at 4 nodes per round,{" "}
        <Code>0.002 SOL × 70% ÷ 4 = 0.00035 SOL</Code> per round. At 2,500
        rounds/day and $150 SOL, a single node earns ~$130/month before
        compute. A Raspberry Pi + internet runs ~$10/month — operators
        break even within weeks at healthy throughput.
      </P>
    </DocsPageShell>
  )
}
