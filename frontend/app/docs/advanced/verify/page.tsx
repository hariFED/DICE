import type { Metadata } from "next"
import { DocsPageShell } from "@/components/docs/PageShell"
import { H2, H3, P, Code, Ul, Li, Ol } from "@/components/docs/Prose"
import { CodeBlock } from "@/components/docs/CodeBlock"
import { Callout } from "@/components/docs/Callout"

export const metadata: Metadata = {
  title: "Verifying randomness",
  description:
    "Reproduce the 32-byte output from chain data. One-line check: SHA-256 over the revealed entropy shares.",
}

const TOC = [
  { id: "one-line-recipe", label: "One-line recipe", level: 2 as const },
  { id: "gather-inputs", label: "Gather the inputs", level: 2 as const },
  { id: "reproduce", label: "Reproduce the output", level: 2 as const },
  { id: "signatures", label: "Verifying signatures", level: 2 as const },
  { id: "what-this-proves", label: "What this proves", level: 2 as const },
]

const TS = `// Reproduce a DICE round's 32-byte randomness from chain data.
// All inputs are public — anyone with a Solana RPC connection can run this.
import { createHash } from "node:crypto"

type RevealShare = {
  deviceId: string        // hex-encoded sha256(device_pubkey)
  entropy: Uint8Array     // 32 bytes
  signature: Uint8Array   // secp256k1 DER — optional for this check
}

export function reproduceRandomness(shares: RevealShare[]): Uint8Array {
  // DICE orders shares by device_id ascending. This is enforced on-chain
  // so a malicious reorder would have been rejected at submit time.
  const sorted = [...shares].sort((a, b) =>
    a.deviceId.localeCompare(b.deviceId),
  )

  const h = createHash("sha256")
  for (const s of sorted) {
    if (s.entropy.length !== 32) throw new Error("share must be 32 bytes")
    h.update(s.entropy)
  }
  return h.digest()
}

// Usage:
const computed = reproduceRandomness(sharesFromChain)
const onChain = channelAccount.randomness
if (!bytesEq(computed, onChain)) {
  throw new Error("randomness does not match — investigate")
}`

const SIGCHECK = `// Optional: verify every reveal signature against its registered pubkey.
import { ecdsaVerify } from "secp256k1"
import { createHash } from "node:crypto"

function verifyShare(
  share: RevealShare,
  devicePubkey: Uint8Array,
  roundId: bigint,
): boolean {
  // DICE signs \`sha256("dice-reveal-v1" || round_id_le || entropy)\`.
  const preimage = Buffer.concat([
    Buffer.from("dice-reveal-v1"),
    Buffer.from(toLeBytes(roundId, 8)),
    Buffer.from(share.entropy),
  ])
  const msgHash = createHash("sha256").update(preimage).digest()
  return ecdsaVerify(share.signature, msgHash, devicePubkey)
}`

export default function VerifyPage() {
  return (
    <DocsPageShell
      href="/docs/advanced/verify"
      title="Verifying randomness"
      lead="DICE's 32-byte output is a pure function of public chain data. You don't have to trust the coordinator — you can reproduce it yourself."
      toc={TOC}
    >
      <H2 id="one-line-recipe">One-line recipe</H2>
      <P>
        The aggregation rule is:
      </P>
      <div className="my-4 rounded-lg border border-white/[0.08] bg-white/[0.02] px-4 py-3 font-mono text-[14px] text-zinc-300">
        randomness = SHA-256(reveal_1 ‖ reveal_2 ‖ … ‖ reveal_n)
      </div>
      <P>
        Where <Code>reveal_i</Code> is the 32-byte entropy each selected
        device published in <Code>submit_reveal</Code>, concatenated in
        ascending <Code>device_id</Code> order. Run that SHA-256 locally,
        compare to the value stored on the <Code>DiceChannel</Code>. If
        they match, the round is authentic.
      </P>

      <Callout type="info" title="Why this is enough">
        <p>
          Every reveal is (a) pre-committed, (b) signed by hardware, and
          (c) on chain. There is no secret input to the function. Anyone
          with a Solana RPC connection can reconstruct the output from
          first principles.
        </p>
      </Callout>

      <H2 id="gather-inputs">Gather the inputs</H2>
      <Ol>
        <Li>
          Find the round's <Code>submit_reveal</Code> transaction. The DICE
          explorer links it directly from the channel's round history.
        </Li>
        <Li>
          Decode the TX's instruction data — each share is a{" "}
          <Code>(device_id, entropy, signature)</Code> triple.
        </Li>
        <Li>
          Fetch the <Code>DiceChannel</Code> account to get the stored{" "}
          <Code>randomness</Code> (your comparison target) and{" "}
          <Code>round_id</Code>.
        </Li>
      </Ol>

      <H2 id="reproduce">Reproduce the output</H2>
      <P>
        Plain TypeScript, no DICE SDK required. This is the canonical
        check:
      </P>
      <CodeBlock code={TS} lang="typescript" filename="verify.ts" />

      <P>
        <b>Sort order matters.</b> The program enforces{" "}
        <Code>device_id</Code>-ascending concatenation inside{" "}
        <Code>submit_reveal</Code>. A verifier that concatenates in a
        different order will compute a different hash and (correctly)
        conclude the round looks wrong. Always sort by{" "}
        <Code>device_id</Code> as hex before hashing.
      </P>

      <H2 id="signatures">Verifying signatures</H2>
      <P>
        The SHA-256 check only proves the bytes on chain are internally
        consistent. To also prove they came from real hardware, verify each
        reveal signature against the device's registered public key. The
        signed message is{" "}
        <Code>SHA-256(&quot;dice-reveal-v1&quot; ‖ round_id ‖ entropy)</Code>.
      </P>
      <CodeBlock code={SIGCHECK} lang="typescript" filename="verify.ts" />
      <P>
        Device pubkeys are stored on <Code>NodeRegistry</Code> accounts
        and keyed by <Code>device_id = SHA-256(device_pubkey)</Code>. The
        program enforces this identity at registration time, so a matching
        <Code>device_id</Code> guarantees you're checking against the
        correct key.
      </P>

      <H2 id="what-this-proves">What this proves</H2>
      <Ul>
        <Li>
          <b>Integrity</b> — the 32-byte result is exactly the SHA-256 of
          the revealed shares, not some other value injected later.
        </Li>
        <Li>
          <b>Authenticity</b> — every share was signed by a device whose
          pubkey is registered on chain. No software-only impersonation.
        </Li>
        <Li>
          <b>Unbiasability</b> — commits landed before reveals (you can
          check this from the TX timestamps), so no node could pick its
          share after seeing the others.
        </Li>
      </Ul>
      <P>
        Taken together, reproducing randomness yourself is the "don't
        trust — verify" story for DICE. It replaces the question "do I
        trust this oracle?" with "do I trust Solana's consensus?"
      </P>
    </DocsPageShell>
  )
}
