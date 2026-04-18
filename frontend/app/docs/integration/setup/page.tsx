import type { Metadata } from "next"
import { DocsPageShell } from "@/components/docs/PageShell"
import { H2, H3, P, Code, Ul, Li } from "@/components/docs/Prose"
import { CodeBlock } from "@/components/docs/CodeBlock"
import { Callout } from "@/components/docs/Callout"

export const metadata: Metadata = {
  title: "Channel setup",
  description:
    "Create and fund the DICE channel that binds your program to the oracle. Runs once at deploy time.",
}

const TOC = [
  { id: "what-is-a-channel", label: "What is a channel?", level: 2 as const },
  { id: "one-time-script", label: "One-time setup script", level: 2 as const },
  { id: "funding", label: "Funding", level: 2 as const },
  { id: "declare-id-again", label: "Wait — declare_id again?", level: 2 as const },
  { id: "sanity-check", label: "Sanity check", level: 2 as const },
]

const INIT = `// scripts/init-dice-channel.ts
// Run ONCE, at deploy time. Idempotent — rerunning does nothing if the
// channel already exists.
import * as anchor from "@coral-xyz/anchor"
import { PublicKey, Keypair, Connection } from "@solana/web3.js"
import { Dice } from "../target/types/dice"
import { MyGame } from "../target/types/my_game"

async function main() {
  const provider = anchor.AnchorProvider.env()
  anchor.setProvider(provider)

  const dice = anchor.workspace.Dice as anchor.Program<Dice>
  const myGame = anchor.workspace.MyGame as anchor.Program<MyGame>

  // Channel index: use 0 unless you need multiple channels per program.
  const channelIndex = 0
  const maxNodes = 16 // upper bound on per-request node_count

  // The coordinator pubkey — devnet. For mainnet this will be different;
  // always confirm against the Explorer before copying into production code.
  const coordinator = new PublicKey(
    "3df8FZoosdv3mrYwWS82TEqQps97qAdmnnijUNhz6tp9",
  )

  await dice.methods
    .initChannel(channelIndex, maxNodes, myGame.programId, coordinator)
    .accounts({
      authority: provider.wallet.publicKey,
      systemProgram: anchor.web3.SystemProgram.programId,
    })
    .rpc()

  console.log("Channel created for", myGame.programId.toBase58())
}

main().catch((e) => {
  console.error(e)
  process.exit(1)
})
`

const FUND = `// scripts/fund-dice-channel.ts
import * as anchor from "@coral-xyz/anchor"
import { LAMPORTS_PER_SOL } from "@solana/web3.js"

const provider = anchor.AnchorProvider.env()
anchor.setProvider(provider)

const dice = anchor.workspace.Dice as anchor.Program

// 1 SOL buys ~500 rounds at 0.002 SOL each.
const amount = BigInt(1 * LAMPORTS_PER_SOL)

await dice.methods.fundChannel(amount).rpc()
console.log("Channel topped up with", amount.toString(), "lamports")
`

export default function SetupPage() {
  return (
    <DocsPageShell
      href="/docs/integration/setup"
      title="Channel setup"
      lead="A channel is a long-lived PDA that tells DICE which program to call back and which wallet holds the pre-paid round budget. You create it once and fund it whenever it runs low."
      toc={TOC}
    >
      <H2 id="what-is-a-channel">What is a channel?</H2>
      <P>
        A DICE <b>channel</b> stores three pieces of per-dApp config:
      </P>
      <Ul>
        <Li>
          Your program's pubkey — DICE CPIs into this address when a round
          finalises.
        </Li>
        <Li>
          An upper bound on nodes per round (<Code>max_nodes</Code>), so a
          channel can't be drained by griefers requesting 50-node rounds.
        </Li>
        <Li>
          A small SOL balance used to pay for rounds. Every request debits{" "}
          <Code>0.002 SOL</Code>; you top up when it gets low.
        </Li>
      </Ul>
      <P>
        The channel address is a PDA of{" "}
        <Code>[&quot;channel&quot;, authority, channel_index]</Code>. Your
        deploy script computes it, creates it, and saves the pubkey for{" "}
        <Code>request_randomness_auto</Code> to pass later.
      </P>

      <H2 id="one-time-script">One-time setup script</H2>
      <P>
        The canonical pattern is an Anchor script that runs{" "}
        <Code>initChannel</Code> the first time. It's idempotent — if the
        channel already exists the call is a no-op.
      </P>
      <CodeBlock code={INIT} lang="typescript" filename="scripts/init-dice-channel.ts" />

      <Callout type="info" title="Where does the coordinator pubkey come from?">
        <p>
          The DICE coordinator is the off-chain daemon that observes the
          channel and drives rounds. Its pubkey is published by operations
          and visible on the{" "}
          <a
            href="/explorer"
            className="text-[#00FF85] underline decoration-[#00FF85]/30 underline-offset-4"
          >
            explorer
          </a>
          . On devnet, this is a stable, well-known key.
        </p>
      </Callout>

      <H2 id="funding">Funding</H2>
      <P>
        A fresh channel has zero balance. It needs SOL before any{" "}
        <Code>request_randomness_auto</Code> will succeed — if not, that call
        fails with <Code>EscrowInsufficient</Code>. Top up with{" "}
        <Code>fundChannel</Code>:
      </P>
      <CodeBlock code={FUND} lang="typescript" filename="scripts/fund-dice-channel.ts" />
      <P>
        You can also fund from any wallet, not just the channel authority.
        This is useful for ops — an automation job can refill every channel
        when a balance threshold trips.
      </P>

      <H3 id="fund-math">How much to fund?</H3>
      <Ul>
        <Li>
          <b>Dev / test</b> — 0.1 SOL funds ~50 rounds. Enough for a week of
          integration work.
        </Li>
        <Li>
          <b>Production</b> — 1 SOL funds ~500 rounds. Add a watcher that
          refills when the channel drops below 0.05 SOL.
        </Li>
      </Ul>

      <H2 id="declare-id-again">Wait — <Code>declare_id</Code> again?</H2>
      <P>
        No. You did that already in the Quickstart. The thing to double-check
        here is that <Code>myGame.programId</Code> in the init script above
        matches the <Code>declare_id!(...)</Code> in your{" "}
        <Code>lib.rs</Code>. If those disagree, Anchor will happily deploy
        your program but every round will land callbacks on a different
        address — and the CPI will fail silently.
      </P>

      <Callout type="warn" title="Common mistake">
        <p>
          Regenerating your keypair after calling <Code>initChannel</Code>.
          The channel stores <i>the old</i> program id; your new program can
          never receive a callback until you either redeploy at the original
          address or call a re-target instruction.
        </p>
      </Callout>

      <H2 id="sanity-check">Sanity check</H2>
      <P>
        Three lines of TypeScript confirm everything is wired up correctly:
      </P>
      <CodeBlock
        code={`const channel = await dice.account.diceChannel.fetch(channelPubkey)
console.log("callback_program:", channel.callbackProgramId.toBase58())
console.log("balance_lamports:", channel.balance.toString())`}
        lang="typescript"
      />
      <P>
        If <Code>callback_program_id</Code> matches <Code>declare_id!</Code>,
        and <Code>balance</Code> is non-zero, you're ready to request
        randomness.
      </P>
    </DocsPageShell>
  )
}
