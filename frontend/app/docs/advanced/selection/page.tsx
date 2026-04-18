import type { Metadata } from "next"
import { DocsPageShell } from "@/components/docs/PageShell"
import { H2, H3, P, Code, Ul, Li } from "@/components/docs/Prose"
import { CodeBlock } from "@/components/docs/CodeBlock"
import { Callout } from "@/components/docs/Callout"

export const metadata: Metadata = {
  title: "On-chain selection",
  description:
    "Audit mode internals: how Fisher-Yates seeded by SlotHashes picks which devices run a round, and how to reproduce the pick from chain data.",
}

const TOC = [
  { id: "problem", label: "The problem audit mode solves", level: 2 as const },
  { id: "seed-source", label: "Seed source · SlotHashes", level: 2 as const },
  { id: "shuffle", label: "Fisher-Yates on-chain", level: 2 as const },
  { id: "reproduce", label: "Reproducing the selection", level: 2 as const },
  { id: "reference", label: "Reference implementation", level: 2 as const },
]

const TS = `// Reproduce audit-mode device selection from chain data.
// Same algorithm the on-chain program runs — writing it here for verifiers.
import { createHash } from "node:crypto"

export function reproduceSelection(
  deviceList: string[],        // NodeRegistry-ordered device_ids
  seed: Uint8Array,            // 32 bytes from SlotHashes at request-slot
  nodeCount: number,           // the node_count passed to request_randomness_auto
): string[] {
  if (nodeCount > deviceList.length) {
    throw new Error("node_count exceeds registered devices")
  }

  // Work on a copy; Fisher-Yates shuffles in place.
  const arr = [...deviceList]
  const n = arr.length

  // Expand the 32-byte seed into a keystream as needed using SHA-256
  // in counter mode. This is what the program does — identical bytes out.
  let counter = 0n
  let buf = new Uint8Array(0)
  let offset = 0
  const nextU32 = (): number => {
    while (buf.length - offset < 4) {
      buf = sha256(concat(seed, u64le(counter)))
      counter += 1n
      offset = 0
    }
    const v =
      (buf[offset] | (buf[offset + 1] << 8) |
       (buf[offset + 2] << 16) | (buf[offset + 3] << 24)) >>> 0
    offset += 4
    return v
  }

  // Partial Fisher-Yates: only shuffle the first nodeCount positions.
  for (let i = 0; i < nodeCount; i++) {
    const j = i + (nextU32() % (n - i))
    const tmp = arr[i]
    arr[i] = arr[j]
    arr[j] = tmp
  }
  return arr.slice(0, nodeCount)
}

/* --- tiny helpers --- */
function sha256(b: Uint8Array): Uint8Array {
  return new Uint8Array(createHash("sha256").update(b).digest())
}
function concat(a: Uint8Array, b: Uint8Array): Uint8Array {
  const out = new Uint8Array(a.length + b.length)
  out.set(a, 0); out.set(b, a.length)
  return out
}
function u64le(n: bigint): Uint8Array {
  const out = new Uint8Array(8)
  for (let i = 0; i < 8; i++) { out[i] = Number(n & 0xffn); n >>= 8n }
  return out
}`

export default function SelectionPage() {
  return (
    <DocsPageShell
      href="/docs/advanced/selection"
      title="On-chain selection"
      lead="Audit mode picks the device set on-chain, using a Fisher-Yates shuffle seeded by the SlotHashes sysvar. The selection is a pure function of chain state, which means anyone can replay it."
      toc={TOC}
    >
      <H2 id="problem">The problem audit mode solves</H2>
      <P>
        In streaming mode, the coordinator picks which devices participate
        in a round. That's fine for UX-critical flows — the result is
        still cryptographically unbiasable because of commit-reveal — but
        it's <i>not</i> reproducible from chain data alone. An auditor who
        asks "why these four devices and not those other four?" gets the
        answer "coordinator internal state." Defensible, but not by
        construction.
      </P>
      <P>
        Audit mode replaces coordinator authority with a deterministic
        algorithm run inside <Code>select_nodes</Code>. The input is
        on-chain; the output is on-chain; anyone can rerun it.
      </P>

      <H2 id="seed-source">Seed source · <Code>SlotHashes</Code></H2>
      <P>
        The program reads the most recent entry of Solana's{" "}
        <Code>SlotHashes</Code> sysvar at the slot the request lands. This
        sysvar is a ring-buffer of recent slot → hash mappings, maintained
        by the runtime. Its value at any given slot is:
      </P>
      <Ul>
        <Li>
          <b>Unpredictable</b> — depends on all leader-produced blocks up
          to that slot; no-one knows in advance what will hash to what.
        </Li>
        <Li>
          <b>Public</b> — any RPC node exposes it. Verifiers can fetch the
          same seed you used.
        </Li>
        <Li>
          <b>Non-grindable by the requester</b> — the seed is chosen by the
          program at request time, not by the caller.
        </Li>
      </Ul>

      <Callout type="info" title="Why not use a commit-reveal for selection too?">
        <p>
          We could, but it doubles the number of on-chain transactions per
          round — two commits, two reveals — for a property{" "}
          <Code>SlotHashes</Code> already gives us for free. The{" "}
          <i>result</i> randomness is still from commit-reveal; only{" "}
          <i>who runs</i> the round needs the cheaper property.
        </p>
      </Callout>

      <H2 id="shuffle">Fisher-Yates on-chain</H2>
      <P>
        The program keeps a canonical, ordered list of registered devices
        in the <Code>NodeRegistry</Code> account. Given the seed, it runs a
        partial Fisher-Yates — shuffle the first <Code>node_count</Code>{" "}
        positions — and emits that prefix as the selected set. The
        algorithm is uniform over all <Code>nCr(total, node_count)</Code>
        possible subsets given the seed.
      </P>

      <P>
        "Partial" matters: we don't waste compute shuffling elements we'll
        never pick. For a 100-device registry with{" "}
        <Code>node_count = 4</Code>, the shuffle does exactly 4 swaps.
      </P>

      <H2 id="reproduce">Reproducing the selection</H2>
      <P>
        Three inputs, one output. An auditor needs:
      </P>
      <Ul>
        <Li>
          The registered-device list at the request slot (from{" "}
          <Code>NodeRegistry</Code>).
        </Li>
        <Li>
          The <Code>SlotHashes</Code> entry for the request slot.
        </Li>
        <Li>
          The <Code>node_count</Code> argument (from the request TX).
        </Li>
      </Ul>
      <P>
        Feed those into the same Fisher-Yates and confirm the output
        matches the <Code>selected_devices</Code> recorded on the round
        account.
      </P>

      <H2 id="reference">Reference implementation</H2>
      <P>
        Runnable TypeScript that mirrors the on-chain shuffle byte-for-byte:
      </P>
      <CodeBlock code={TS} lang="typescript" filename="reproduce-selection.ts" />

      <Callout type="tip" title="This is what auditors run">
        <p>
          When a regulator asks "reproduce this round's device selection
          from chain data" — they point their own verifier at an RPC, fetch
          the three inputs above, and run code shaped like this. If the
          output matches, the selection is confirmed.
        </p>
      </Callout>

      <H3 id="gotchas">Gotchas</H3>
      <Ul>
        <Li>
          <b>NodeRegistry mutations</b> — if devices are added / removed
          between the request and the audit, use the registry snapshot
          from the request slot, not the current one. The program stores a
          registry version on each round for exactly this reason.
        </Li>
        <Li>
          <b>Seed encoding</b> — the SlotHashes entry is 32 bytes. Pass it
          as raw bytes, not hex-encoded.
        </Li>
        <Li>
          <b>Counter length</b> — we use <Code>u64</Code> little-endian for
          the SHA-256 keystream counter. Tests should stress past 2^32
          requests against a single seed to catch off-by-one bugs.
        </Li>
      </Ul>
    </DocsPageShell>
  )
}
