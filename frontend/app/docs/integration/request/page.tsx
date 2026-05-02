import type { Metadata } from "next"
import { DocsPageShell } from "@/components/docs/PageShell"
import { H2, H3, P, Code, Ul, Li } from "@/components/docs/Prose"
import { CodeBlock } from "@/components/docs/CodeBlock"
import { Callout } from "@/components/docs/Callout"

export const metadata: Metadata = {
  title: "Request randomness",
  description:
    "Add one CPI call to your game instruction. Hardware round starts instantly.",
}

const TOC = [
  { id: "the-cpi-call", label: "The CPI call", level: 2 as const },
  { id: "accounts-struct", label: "The Accounts struct", level: 2 as const },
  { id: "node-count", label: "Picking node_count", level: 2 as const },
  { id: "failure-modes", label: "Failure modes", level: 2 as const },
  { id: "client-side", label: "Client-side invocation", level: 2 as const },
]

const FULL = `use anchor_lang::prelude::*;

// Your own program's pubkey — see Quickstart for why.
declare_id!("7xAbcYourProgramIdHereReplaceMePlease111111");

#[program]
pub mod my_game {
    use super::*;

    pub fn play(ctx: Context<Play>, bet: u8, wager: u64) -> Result<()> {
        // 1. Game setup — record the bet and take the wager.
        let g = &mut ctx.accounts.game;
        g.player = ctx.accounts.player.key();
        g.bet = bet;
        g.wager = wager;
        g.settled = false;

        // 2. Fire the DICE CPI. The hardware round starts the moment this
        // TX confirms; DICE will call us back on the same channel when done.
        dice::cpi::request_randomness_auto(
            CpiContext::new(
                ctx.accounts.dice_program.to_account_info(),
                dice::cpi::accounts::RequestRandomnessAuto {
                    authority:      ctx.accounts.player.to_account_info(),
                    channel:        ctx.accounts.dice_channel.to_account_info(),
                    system_program: ctx.accounts.system_program.to_account_info(),
                },
            ),
            4, // node_count — see below
        )?;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Play<'info> {
    #[account(mut)]
    pub player: Signer<'info>,

    /// The round's game state. Your own PDA.
    #[account(
        init,
        payer = player,
        space = 8 + GameState::INIT_SPACE,
        seeds = [b"game", player.key().as_ref()],
        bump,
    )]
    pub game: Account<'info, GameState>,

    /// CHECK: the DICE channel PDA. DICE verifies it inside the CPI.
    #[account(mut)]
    pub dice_channel: UncheckedAccount<'info>,

    /// CHECK: DICE program id. Anchor's CPI invocation cross-checks it.
    pub dice_program: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}`

const CLIENT = `// Your game's play() call from a TS client.
await myGame.methods
  .play(3, new BN(100_000))       // bet = 3, wager = 0.0001 SOL
  .accounts({
    player: wallet.publicKey,
    game: gamePda,                 // derived from ["game", player]
    diceChannel: diceChannelPda,   // created in init-dice-channel.ts
    diceProgram: dice.programId,
    systemProgram: anchor.web3.SystemProgram.programId,
  })
  .rpc()

// That's it. ~4 seconds later your GameState will be settled.
// Poll or subscribe:
const sub = anchor.AccountClient.subscribe
  ? myGame.account.gameState.subscribe(gamePda)
  : null`

export default function RequestPage() {
  return (
    <DocsPageShell
      href="/docs/integration/request"
      title="Request randomness"
      lead="One CPI call inside whatever instruction starts a round in your game. DICE takes over from there — commits, reveals, aggregation, callback."
      toc={TOC}
    >
      <H2 id="the-cpi-call">The CPI call</H2>
      <P>
        <Code>dice::cpi::request_randomness_auto</Code> takes exactly three
        accounts — the caller (your player), the channel PDA, and the
        System program — plus one argument: how many devices to recruit for
        this round. Here's the full handler in context:
      </P>

      <CodeBlock code={FULL} lang="rust" filename="programs/my_game/src/lib.rs" />

      <Callout type="info" title="Atomic with your game logic">
        <p>
          Because it's a CPI inside your instruction, the DICE request is
          atomic with the rest of your <Code>play()</Code> handler. If
          anything later in the TX reverts, the randomness request reverts
          too. You don't need a two-phase commit in your code.
        </p>
      </Callout>

      <H2 id="accounts-struct">The Accounts struct</H2>
      <P>
        Three accounts on the DICE side. Your struct adds its own — whatever
        your game needs (<Code>game</Code> PDA, <Code>player</Code> signer,
        etc.). Four details worth knowing:
      </P>
      <Ul>
        <Li>
          <Code>authority</Code> is <b>your player</b>, not an admin. DICE
          uses it only as the signer that pre-authorises the channel debit.
        </Li>
        <Li>
          <Code>channel</Code> is the PDA you created in{" "}
          <a
            href="/docs/integration/setup"
            className="text-[var(--foreground)] underline decoration-[var(--foreground)]/30 underline-offset-4"
          >
            channel setup
          </a>
          . Passed as <Code>mut</Code> because the round fee is debited in
          place.
        </Li>
        <Li>
          <Code>dice_program</Code> is the DICE program itself. Pass{" "}
          <Code>dice.programId</Code> from the client.
        </Li>
        <Li>
          <Code>system_program</Code> is standard Solana System; Anchor
          requires it for the fee transfer.
        </Li>
      </Ul>

      <H2 id="node-count">Picking <Code>node_count</Code></H2>
      <P>
        Minimum 4, maximum 50. The security property scales linearly — 1 of
        N honest nodes makes the output unbiasable, so higher N tolerates
        more Byzantine devices. The fee is flat (<Code>0.002 SOL</Code>)
        regardless of N; operators split it proportionally.
      </P>
      <div className="my-4 overflow-x-auto rounded-sm border border-border">
        <table className="w-full text-left text-[14px]">
          <thead className="bg-muted/40 text-foreground">
            <tr>
              <th className="px-3 py-2 font-medium">node_count</th>
              <th className="px-3 py-2 font-medium">Recommended for</th>
              <th className="px-3 py-2 font-medium">Notes</th>
            </tr>
          </thead>
          <tbody className="text-muted-foreground">
            <tr className="border-t border-border">
              <td className="px-3 py-2 font-mono text-[#7cf9c1]">4</td>
              <td className="px-3 py-2">Default · games · mints</td>
              <td className="px-3 py-2">Fastest path. Tolerates 3 faulty.</td>
            </tr>
            <tr className="border-t border-border">
              <td className="px-3 py-2 font-mono text-[#7cf9c1]">8–16</td>
              <td className="px-3 py-2">High-stakes rounds</td>
              <td className="px-3 py-2">
                More devices sign → more costly to collude against.
              </td>
            </tr>
            <tr className="border-t border-border">
              <td className="px-3 py-2 font-mono text-[#7cf9c1]">32–50</td>
              <td className="px-3 py-2">Regulated / RWA draws</td>
              <td className="px-3 py-2">Slight latency bump; maximum assurance.</td>
            </tr>
          </tbody>
        </table>
      </div>

      <H2 id="failure-modes">Failure modes</H2>
      <Ul>
        <Li>
          <Code>InsufficientNodes</Code> — fewer than 4 nodes were online at
          the moment of request. Retry with a smaller node_count or wait.
        </Li>
        <Li>
          <Code>InvalidNodeCount</Code> — you passed &lt; 4 or &gt; 50.
        </Li>
        <Li>
          <Code>EscrowInsufficient</Code> — channel is out of SOL. Fund it.
        </Li>
      </Ul>
      <P>
        A full table of error variants lives on the{" "}
        <a
          href="/docs/reference/errors"
          className="text-[var(--foreground)] underline decoration-[var(--foreground)]/30 underline-offset-4"
        >
          errors reference
        </a>
        .
      </P>

      <H2 id="client-side">Client-side invocation</H2>
      <P>
        Nothing DICE-specific for the caller — your player signs{" "}
        <Code>play()</Code> on your program and the CPI is internal. A
        minimal TS client:
      </P>
      <CodeBlock code={CLIENT} lang="typescript" filename="client.ts" />

      <Callout type="tip" title="Subscribe, don't poll">
        <p>
          Because the callback lands in a separate TX (a few slots later),
          an account subscription on your <Code>GameState</Code> is the
          cleanest way to get an instant UI update when{" "}
          <Code>settled</Code> flips to <Code>true</Code>.
        </p>
      </Callout>

      <H3 id="mock-hardware">Testing without hardware</H3>
      <P>
        <Code>tests/harness/mock_firmware_node</Code> in the DICE repo
        simulates 10 devices running the real CBOR protocol and ECDSA
        signing. Point your devnet coordinator at it and your integration
        tests will exercise every code path without touching a physical
        ESP32.
      </P>
    </DocsPageShell>
  )
}
