import type { Metadata } from "next"
import { DocsPageShell } from "@/components/docs/PageShell"
import { H2, H3, P, Code, Ul, Li } from "@/components/docs/Prose"
import { CodeBlock } from "@/components/docs/CodeBlock"

export const metadata: Metadata = {
  title: "Callback payload",
  description: "The exact 72-byte wire format your dice_callback receives.",
}

const TOC = [
  { id: "layout", label: "Byte-level layout", level: 2 as const },
  { id: "discriminator", label: "Discriminator", level: 2 as const },
  { id: "channel-key", label: "channel_key", level: 2 as const },
  { id: "randomness", label: "randomness", level: 2 as const },
  { id: "accounts-passed", label: "Accounts passed", level: 2 as const },
  { id: "decoder", label: "Off-chain decoder", level: 2 as const },
]

const SEGMENTS = [
  {
    range: "[0 .. 8]",
    name: "discriminator",
    desc: "Anchor route. SHA-256(\"global:dice_callback\")[0..8].",
    color: "bg-sky-400",
  },
  {
    range: "[8 .. 40]",
    name: "channel_key",
    desc: "The DiceChannel PDA that signed this result.",
    color: "bg-[var(--foreground)]",
  },
  {
    range: "[40 .. 72]",
    name: "randomness",
    desc: "32 bytes of hardware-backed entropy.",
    color: "bg-amber-400",
  },
]

const DECODER = `// Off-chain decoder for DICE callback instruction data.
// Useful when you want to show the result in a UI before the chain state
// catches up, or when debugging with raw TXs from the explorer.
import { PublicKey } from "@solana/web3.js"

export type DiceResult = {
  channelKey: PublicKey
  randomness: Uint8Array  // length 32
}

export function decodeDiceCallback(data: Uint8Array): DiceResult {
  if (data.length !== 72) {
    throw new Error(\`expected 72 bytes, got \${data.length}\`)
  }
  // bytes [0..8] is the Anchor discriminator — skip.
  return {
    channelKey: new PublicKey(data.slice(8, 40)),
    randomness: data.slice(40, 72),
  }
}`

export default function PayloadPage() {
  return (
    <DocsPageShell
      href="/docs/reference/payload"
      title="Callback payload"
      lead="When DICE's deliver_callback CPIs into your program, the instruction data is exactly 72 bytes. Here's the layout."
      toc={TOC}
    >
      <H2 id="layout">Byte-level layout</H2>
      <P>
        Three contiguous fields, no padding. Anchor deserialises these into
        a <Code>DiceResult</Code> struct for you — this table is only
        relevant if you're writing a client-side decoder or debugging a CPI
        failure.
      </P>

      {/* Filmstrip */}
      <div className="my-6 grid grid-cols-3 overflow-hidden rounded-sm border border-border">
        {SEGMENTS.map((s) => (
          <div
            key={s.name}
            className="flex flex-col border-r border-border last:border-r-0"
          >
            <div className={`${s.color} h-1`} />
            <div className="flex flex-col gap-1 p-3">
              <span className="font-mono text-[11px] text-muted-foreground">
                {s.range}
              </span>
              <span className="font-mono text-[13.5px] font-semibold text-foreground">
                {s.name}
              </span>
              <span className="text-[12.5px] leading-[1.45] text-muted-foreground">
                {s.desc}
              </span>
            </div>
          </div>
        ))}
      </div>

      <div className="my-4 overflow-x-auto rounded-sm border border-border">
        <table className="w-full text-left text-[14px]">
          <thead className="bg-muted/40 text-foreground">
            <tr>
              <th className="px-3 py-2 font-medium">Field</th>
              <th className="px-3 py-2 font-medium">Offset</th>
              <th className="px-3 py-2 font-medium">Size</th>
              <th className="px-3 py-2 font-medium">Type</th>
            </tr>
          </thead>
          <tbody className="text-muted-foreground">
            <tr className="border-t border-border">
              <td className="px-3 py-2 font-mono text-[#7cf9c1]">discriminator</td>
              <td className="px-3 py-2">0</td>
              <td className="px-3 py-2">8</td>
              <td className="px-3 py-2 font-mono">[u8; 8]</td>
            </tr>
            <tr className="border-t border-border">
              <td className="px-3 py-2 font-mono text-[#7cf9c1]">channel_key</td>
              <td className="px-3 py-2">8</td>
              <td className="px-3 py-2">32</td>
              <td className="px-3 py-2 font-mono">Pubkey</td>
            </tr>
            <tr className="border-t border-border">
              <td className="px-3 py-2 font-mono text-[#7cf9c1]">randomness</td>
              <td className="px-3 py-2">40</td>
              <td className="px-3 py-2">32</td>
              <td className="px-3 py-2 font-mono">[u8; 32]</td>
            </tr>
          </tbody>
        </table>
      </div>

      <H2 id="discriminator">Discriminator</H2>
      <P>
        The first 8 bytes are an Anchor instruction discriminator — the
        routing tag Anchor uses to pick which handler runs. Its value is{" "}
        <Code>SHA-256(&quot;global:dice_callback&quot;)[0..8]</Code>, which
        means the name{" "}
        <Code>dice_callback</Code> is load-bearing: rename the handler and
        the CPI will miss.
      </P>

      <H2 id="channel-key">channel_key</H2>
      <P>
        A standard 32-byte Solana <Code>Pubkey</Code> — the channel PDA
        that produced this round. Your handler cross-checks it against the{" "}
        <Code>dice_channel</Code> account passed in <Code>Accounts</Code>:
      </P>
      <CodeBlock
        code={`require_keys_eq!(
    ctx.accounts.dice_channel.key(),
    result.channel_key,
    GameError::WrongChannel,
);`}
        lang="rust"
      />

      <H2 id="randomness">randomness</H2>
      <P>
        32 uniformly-distributed bytes. Think of it as a{" "}
        <Code>[u8; 32]</Code> that you can reinterpret as a <Code>u64</Code>,
        a <Code>u32</Code>, a hash seed, whatever fits your game. The
        bytes are derived from{" "}
        <Code>SHA-256(reveal_1 ‖ reveal_2 ‖ … ‖ reveal_n)</Code>, so they are
        not only uniform but publicly reproducible from chain data.
      </P>

      <H2 id="accounts-passed">Accounts passed</H2>
      <P>
        DICE's <Code>deliver_callback</Code> CPIs your program with exactly
        one named account plus <Code>remaining_accounts</Code>:
      </P>
      <Ul>
        <Li>
          <b>Account 0</b> — your game PDA, passed into your Accounts
          struct as the first field (conventionally named{" "}
          <Code>game</Code>). Anchor validates it through{" "}
          <Code>seeds = [...]</Code> in your constraint.
        </Li>
        <Li>
          <b>Account 1</b> — <Code>dice_channel</Code>, the channel PDA.
          You declare this second in your struct.
        </Li>
      </Ul>
      <P>
        If you need extra accounts (an oracle, a mint, a token account),
        add them after these two in your <Code>Accounts</Code> struct and
        teach your client code to pass them as{" "}
        <Code>remainingAccounts</Code> on the original{" "}
        <Code>request_randomness_auto</Code> call. DICE forwards them
        verbatim.
      </P>

      <H2 id="decoder">Off-chain decoder</H2>
      <P>
        Handy if you're debugging or previewing callbacks in a UI:
      </P>
      <CodeBlock code={DECODER} lang="typescript" filename="lib/dice-decoder.ts" />
    </DocsPageShell>
  )
}
