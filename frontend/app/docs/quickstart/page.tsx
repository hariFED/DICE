import type { Metadata } from "next"
import { DocsPageShell } from "@/components/docs/PageShell"
import { H2, H3, P, Code, Ul, Li } from "@/components/docs/Prose"
import { CodeBlock } from "@/components/docs/CodeBlock"
import { Callout } from "@/components/docs/Callout"

export const metadata: Metadata = {
  title: "Quickstart",
  description:
    "Ship verifiable randomness in 60 seconds of reading. Three code changes — that's it.",
}

const TOC = [
  { id: "three-things", label: "The three things you change", level: 2 as const },
  { id: "step-1-declare", label: "1 · declare_id!", level: 2 as const },
  { id: "step-2-request", label: "2 · Request randomness", level: 2 as const },
  { id: "step-3-callback", label: "3 · Handle the callback", level: 2 as const },
  { id: "outcome-formula", label: "Your outcome formula", level: 2 as const },
  { id: "next-steps", label: "Next steps", level: 2 as const },
]

const CARGO = `# One dependency. The \`cpi\` feature pulls in dice::cpi::request_randomness_auto.
# \`no-entrypoint\` prevents the entrypoint symbol collision inside your binary.
[dependencies]
anchor-lang = "0.30"
dice = { git = "https://github.com/hariFED/DICE", features = ["cpi", "no-entrypoint"] }
`

const DECLARE_ID = `// programs/my_game/src/lib.rs
use anchor_lang::prelude::*;

// Replace with YOUR program's pubkey — solana-keygen pubkey target/deploy/my_game-keypair.json
declare_id!("7xAbcYourProgramIdHereReplaceMePlease111111");

#[program]
pub mod my_game {
    use super::*;
    // handlers go here
}
`

const REQUEST = `use anchor_lang::prelude::*;

#[program]
pub mod my_game {
    use super::*;

    pub fn play(ctx: Context<Play>, bet: u8, wager: u64) -> Result<()> {
        // 1. Your game setup: store the bet, take the wager.
        let g = &mut ctx.accounts.game;
        g.player = ctx.accounts.player.key();
        g.bet = bet;
        g.wager = wager;
        g.settled = false;

        // 2. Ask DICE for randomness. Hardware round starts instantly.
        dice::cpi::request_randomness_auto(
            CpiContext::new(
                ctx.accounts.dice_program.to_account_info(),
                dice::cpi::accounts::RequestRandomnessAuto {
                    authority:      ctx.accounts.player.to_account_info(),
                    channel:        ctx.accounts.dice_channel.to_account_info(),
                    system_program: ctx.accounts.system_program.to_account_info(),
                },
            ),
            4, // node_count: minimum 4, maximum 50
        )?;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Play<'info> {
    #[account(mut)]
    pub player: Signer<'info>,

    #[account(
        init,
        payer = player,
        space = 8 + GameState::INIT_SPACE,
        seeds = [b"game", player.key().as_ref()],
        bump,
    )]
    pub game: Account<'info, GameState>,

    /// CHECK: validated by the DICE program.
    #[account(mut)]
    pub dice_channel: UncheckedAccount<'info>,

    /// CHECK: the DICE program itself — Anchor verifies program id via CPI.
    pub dice_program: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

#[account]
#[derive(InitSpace)]
pub struct GameState {
    pub player: Pubkey,
    pub bet: u8,
    pub wager: u64,
    pub outcome: u8,
    pub randomness: [u8; 32],
    pub settled: bool,
}
`

const CALLBACK = `use anchor_lang::prelude::*;

/// Typed payload DICE sends into your callback as instruction data.
/// Borsh-compatible with the positional \`(channel_key, randomness)\` form
/// but with better IDE autocomplete and room for future fields.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug)]
pub struct DiceResult {
    /// Which DiceChannel produced this randomness.
    pub channel_key: Pubkey,
    /// 32 bytes of hardware-backed entropy.
    pub randomness: [u8; 32],
}

#[derive(Accounts)]
pub struct DiceCallback<'info> {
    /// The game PDA this callback settles. Anchor binds it via seeds;
    /// the constraint blocks double-settlement at compile time.
    #[account(
        mut,
        seeds = [b"game", game.player.as_ref()],
        bump,
        constraint = !game.settled @ GameError::AlreadySettled,
    )]
    pub game: Account<'info, GameState>,

    /// CHECK: cross-checked in the handler against result.channel_key.
    pub dice_channel: UncheckedAccount<'info>,
}

pub fn dice_callback(ctx: Context<DiceCallback>, result: DiceResult) -> Result<()> {
    // Guard against spoofed callbacks from a different channel.
    require_keys_eq!(
        ctx.accounts.dice_channel.key(),
        result.channel_key,
        GameError::WrongChannel,
    );

    let g = &mut ctx.accounts.game;
    // Map the 32 random bytes to a 1..=6 die roll.
    g.outcome = (u32::from_le_bytes([
        result.randomness[0],
        result.randomness[1],
        result.randomness[2],
        result.randomness[3],
    ]) % 6) as u8 + 1;
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
}
`

export default function QuickstartPage() {
  return (
    <DocsPageShell
      href="/docs/quickstart"
      title="Quickstart"
      lead="Add verifiable randomness to an Anchor program in three copy-paste changes. Total integration weight: one dependency, one CPI call, one callback handler."
      toc={TOC}
    >
      <Callout type="info" title="Before you start">
        <p>
          You need an Anchor ≥ 0.30 project, a Solana devnet keypair with ~2
          SOL, and somewhere to deploy. If you don't have those yet, run
          through the{" "}
          <a
            href="https://www.anchor-lang.com/docs/installation"
            className="text-[#00FF85] underline decoration-[#00FF85]/30 underline-offset-4"
          >
            Anchor installation guide
          </a>{" "}
          first.
        </p>
      </Callout>

      <H2 id="three-things">The three things you change</H2>
      <P>
        DICE is not a framework. It is three small mutations to a normal
        Anchor program:
      </P>
      <Ul>
        <Li>
          <b>
            <Code>declare_id!</Code>
          </b>{" "}
          — point Anchor at <i>your</i> program's pubkey so the runtime
          identity check passes.
        </Li>
        <Li>
          <b>Outcome formula</b> — the one line inside your callback that
          maps 32 random bytes to whatever your game needs (a die face, a
          winner index, a boolean).
        </Li>
        <Li>
          <b>
            <Code>GameState</Code> struct
          </b>{" "}
          — your own account shape (what a "round" means in your game).
        </Li>
      </Ul>
      <P>
        Everything else — the commit-reveal round, node selection, hardware
        signatures, fee routing — is handled by the DICE program and the
        coordinator. You do not see it in your code.
      </P>

      <H2 id="step-1-declare">1 · declare_id!</H2>
      <P>
        Add DICE as a dependency, then swap the placeholder program id for
        your own.
      </P>

      <CodeBlock code={CARGO} lang="toml" filename="Cargo.toml" />

      <H3 id="why-declare-id">Why <Code>declare_id!</Code>?</H3>
      <P>
        Every Anchor program has <Code>declare_id!(&quot;…&quot;)</Code> at
        the top of its <Code>lib.rs</Code>. At runtime Anchor does an
        identity check — <i>am I deployed at the address I was compiled
        for?</i> If the deployed address doesn't match, every instruction
        fails with <Code>DeclaredProgramIdMismatch</Code>. This prevents
        anyone from redeploying your bytecode at a different address and
        passing it off as yours.
      </P>
      <P>
        So when you copy the DICE quickstart, you replace the placeholder
        with <b>your own program's pubkey</b> — the public key of{" "}
        <Code>target/deploy/&lt;your-program&gt;-keypair.json</Code>. DICE's
        own program id stays inside the <Code>dice</Code> crate; you never
        touch it.
      </P>

      <CodeBlock
        code={`# Generate your program's keypair once, at project init.
solana-keygen new -o target/deploy/my_game-keypair.json

# Print the pubkey to paste into lib.rs.
solana-keygen pubkey target/deploy/my_game-keypair.json`}
        lang="bash"
        filename="shell"
      />

      <CodeBlock code={DECLARE_ID} lang="rust" filename="programs/my_game/src/lib.rs" />

      <H2 id="step-2-request">2 · Request randomness</H2>
      <P>
        Inside whatever instruction starts a round (<Code>play</Code>,{" "}
        <Code>mint</Code>, <Code>draw</Code> — whatever you call it), add one
        CPI call to <Code>dice::cpi::request_randomness_auto</Code>. The
        hardware round begins the moment this instruction lands.
      </P>

      <CodeBlock code={REQUEST} lang="rust" filename="programs/my_game/src/lib.rs" />

      <Callout type="tip" title="Pick your node count">
        <p>
          <Code>node_count = 4</Code> is the floor — fast and cheap. Use{" "}
          <Code>8</Code> or <Code>16</Code> for high-stakes rounds where you
          want more physical devices signing the result. Maximum is{" "}
          <Code>50</Code>.
        </p>
      </Callout>

      <H2 id="step-3-callback">3 · Handle the callback</H2>
      <P>
        When the round finishes, DICE CPIs into your program with the name{" "}
        <Code>dice_callback</Code> and a single argument — a typed{" "}
        <Code>DiceResult</Code> struct. Define the struct, declare the
        accounts, write your outcome line. That's the full integration.
      </P>

      <CodeBlock code={CALLBACK} lang="rust" filename="programs/my_game/src/lib.rs" />

      <Callout type="info" title="Why a typed struct and not positional args?">
        <p>
          Wire-format-identical. Borsh serialises a two-field struct the
          exact same way as <Code>(Pubkey, [u8; 32])</Code> positionally. The
          struct form gives you cleaner autocomplete, future-proof fields,
          and forces you to acknowledge <Code>channel_key</Code> exists
          rather than prefixing it with <Code>_</Code> and ignoring it.
        </p>
      </Callout>

      <H2 id="outcome-formula">Your outcome formula</H2>
      <P>
        The line{" "}
        <Code>
          g.outcome = (u32::from_le_bytes(&hellip;) % 6) as u8 + 1
        </Code>{" "}
        is the only game-specific logic in the whole integration. Change it
        and you've shipped a different game. We collected the five formulas
        that cover ~90% of use-cases on the{" "}
        <a
          href="/docs/integration/formulas"
          className="text-[#00FF85] underline decoration-[#00FF85]/30 underline-offset-4"
        >
          formulas page
        </a>
        .
      </P>

      <H2 id="next-steps">Next steps</H2>
      <Ul>
        <Li>
          Read{" "}
          <a
            href="/docs/integration/setup"
            className="text-[#00FF85] underline decoration-[#00FF85]/30 underline-offset-4"
          >
            Channel setup
          </a>{" "}
          — the one-time PDA you create at deploy-time.
        </Li>
        <Li>
          Skim the{" "}
          <a
            href="/docs/reference/pricing"
            className="text-[#00FF85] underline decoration-[#00FF85]/30 underline-offset-4"
          >
            pricing breakdown
          </a>{" "}
          so channel funding doesn't surprise you.
        </Li>
        <Li>
          If you're in a regulated environment, switch from streaming to{" "}
          <a
            href="/docs/reference/modes"
            className="text-[#00FF85] underline decoration-[#00FF85]/30 underline-offset-4"
          >
            audit mode
          </a>{" "}
          — +8% latency, fully reproducible device selection.
        </Li>
      </Ul>
    </DocsPageShell>
  )
}
