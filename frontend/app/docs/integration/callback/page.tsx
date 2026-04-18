import type { Metadata } from "next"
import { DocsPageShell } from "@/components/docs/PageShell"
import { H2, H3, P, Code, Ul, Li } from "@/components/docs/Prose"
import { CodeBlock } from "@/components/docs/CodeBlock"
import { Callout } from "@/components/docs/Callout"

export const metadata: Metadata = {
  title: "Callback handler",
  description:
    "Define DiceResult, declare the accounts, write the outcome line.",
}

const TOC = [
  { id: "the-three-parts", label: "Three parts", level: 2 as const },
  { id: "dice-result", label: "DiceResult struct", level: 2 as const },
  { id: "accounts", label: "The Accounts struct", level: 2 as const },
  { id: "handler", label: "The handler", level: 2 as const },
  { id: "security-checks", label: "Security checks", level: 2 as const },
  { id: "replay", label: "What if my handler panics?", level: 2 as const },
]

const STRUCT = `use anchor_lang::prelude::*;

/// DICE's \`deliver_callback\` passes this struct as instruction data.
/// Borsh serializes it identically to the positional form
/// \`(channel_key: Pubkey, randomness: [u8; 32])\` — the struct form is a
/// pure DX win (autocomplete + room to extend without breaking callers).
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug)]
pub struct DiceResult {
    /// The DiceChannel PDA that produced this randomness.
    pub channel_key: Pubkey,
    /// 32 bytes of hardware-backed entropy.
    pub randomness: [u8; 32],
}`

const ACCOUNTS = `#[derive(Accounts)]
pub struct DiceCallback<'info> {
    /// The game PDA this callback settles. Anchor binds it via seeds; the
    /// \`constraint\` blocks double-settlement at compile time.
    #[account(
        mut,
        seeds = [b"game", game.player.as_ref()],
        bump,
        constraint = !game.settled @ GameError::AlreadySettled,
    )]
    pub game: Account<'info, GameState>,

    /// CHECK: cross-checked in the handler against result.channel_key.
    /// Must be the channel you configured for this program.
    pub dice_channel: UncheckedAccount<'info>,
}`

const HANDLER = `pub fn dice_callback(ctx: Context<DiceCallback>, result: DiceResult) -> Result<()> {
    // 1. Guard: the passed-in channel account must match the channel that
    //    signed this result. If not, a malicious caller could spoof
    //    results from a different channel.
    require_keys_eq!(
        ctx.accounts.dice_channel.key(),
        result.channel_key,
        GameError::WrongChannel,
    );

    // 2. Outcome formula — the single game-specific line. Dice 1..=6:
    let g = &mut ctx.accounts.game;
    g.outcome = (u32::from_le_bytes([
        result.randomness[0],
        result.randomness[1],
        result.randomness[2],
        result.randomness[3],
    ]) % 6) as u8 + 1;

    // 3. Record the raw 32 bytes so anyone can reproduce your formula.
    g.randomness = result.randomness;
    g.settled = true;
    Ok(())
}

#[error_code]
pub enum GameError {
    #[msg("Game already settled")]
    AlreadySettled,
    #[msg("Wrong channel for this callback")]
    WrongChannel,
}`

export default function CallbackPage() {
  return (
    <DocsPageShell
      href="/docs/integration/callback"
      title="Callback handler"
      lead="This is where your game logic lives. DICE has already done the expensive work — you read 32 bytes and decide what they mean."
      toc={TOC}
    >
      <H2 id="the-three-parts">Three parts</H2>
      <P>
        A DICE callback is three small pieces: the payload type, the
        accounts it operates on, and the handler itself. The first two are
        boilerplate; only the handler has game-specific logic.
      </P>
      <Ul>
        <Li>
          <b>
            <Code>DiceResult</Code>
          </b>{" "}
          — a Borsh-serialisable struct with <Code>channel_key</Code> and a{" "}
          <Code>[u8; 32]</Code>.
        </Li>
        <Li>
          <b>
            <Code>DiceCallback</Code>
          </b>{" "}
          Accounts — your game PDA, the channel for cross-checking, and
          whatever else your handler needs.
        </Li>
        <Li>
          <b>
            <Code>dice_callback</Code>
          </b>{" "}
          — the function Anchor dispatches when DICE CPIs you. Name is
          load-bearing: the discriminator is{" "}
          <Code>SHA-256(&quot;global:dice_callback&quot;)</Code>.
        </Li>
      </Ul>

      <H2 id="dice-result">DiceResult struct</H2>
      <CodeBlock code={STRUCT} lang="rust" filename="programs/my_game/src/lib.rs" />
      <Callout type="tip" title="Why the struct form?">
        <p>
          Wire-compatible with the older positional form — Borsh doesn't
          care. But future versions of DICE may add fields (e.g. a round
          attestation hash). A struct lets us do that without breaking your
          callback signature.
        </p>
      </Callout>

      <H2 id="accounts">The Accounts struct</H2>
      <P>
        Two accounts are required: your game PDA (<Code>mut</Code>, with
        Anchor's seeds + bump check to prove it's the right one), and the{" "}
        <Code>dice_channel</Code> account (unchecked here, cross-checked in
        the handler body).
      </P>
      <CodeBlock code={ACCOUNTS} lang="rust" filename="programs/my_game/src/lib.rs" />

      <Callout type="info" title="remaining_accounts">
        <p>
          DICE passes your game account as{" "}
          <Code>remaining_accounts[0]</Code> when it CPIs your program. That
          means your Anchor <Code>#[derive(Accounts)]</Code> struct consumes
          it as the first positional account — there is no extra
          unpack-from-remaining step in your code.
        </p>
      </Callout>

      <H2 id="handler">The handler</H2>
      <CodeBlock code={HANDLER} lang="rust" filename="programs/my_game/src/lib.rs" />
      <P>
        The only game-specific line is the outcome formula. Everything else
        is boilerplate that applies to every integration. See the{" "}
        <a
          href="/docs/integration/formulas"
          className="text-[#00FF85] underline decoration-[#00FF85]/30 underline-offset-4"
        >
          formulas page
        </a>{" "}
        for ready-to-paste patterns.
      </P>

      <H2 id="security-checks">Security checks</H2>
      <P>
        DICE has already verified that the randomness came from real
        hardware through a commit-reveal round. Your handler adds two
        game-level checks on top:
      </P>
      <Ul>
        <Li>
          <b>Channel identity.</b>{" "}
          <Code>require_keys_eq!(dice_channel.key(), result.channel_key)</Code>{" "}
          prevents a caller from swapping in a different channel account.
          Without this, a malicious integrator of <i>another</i> dApp could
          re-use their own channel's result against yours.
        </Li>
        <Li>
          <b>Single-settlement.</b> The{" "}
          <Code>constraint = !game.settled</Code> in the Accounts struct
          means a replayed callback cannot overwrite a finalised round.
        </Li>
      </Ul>

      <H2 id="replay">What if my handler panics?</H2>
      <P>
        The randomness is not lost. It was written to the{" "}
        <Code>DiceChannel</Code> in a prior transaction — it's still on
        chain, signed by the hardware, publicly readable. You can:
      </P>
      <Ul>
        <Li>Ship a fixed <Code>dice_callback</Code> in a program upgrade.</Li>
        <Li>
          Call <Code>deliver_callback</Code> again with the stored{" "}
          <Code>round_id</Code>. DICE will re-CPI you with the same bytes.
        </Li>
      </Ul>
      <P>
        Because the 32 bytes are content-addressed by the round, the retry
        is deterministic — your handler will see the exact same output the
        first (failed) call would have seen.
      </P>
    </DocsPageShell>
  )
}
