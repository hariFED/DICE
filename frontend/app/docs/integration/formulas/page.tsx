import type { Metadata } from "next"
import { DocsPageShell } from "@/components/docs/PageShell"
import { H2, H3, P, Code } from "@/components/docs/Prose"
import { CodeBlock } from "@/components/docs/CodeBlock"
import { Callout } from "@/components/docs/Callout"

export const metadata: Metadata = {
  title: "Outcome formulas",
  description:
    "Five copy-paste formulas that cover 90% of VRF use-cases: dice, coin, wheel, lottery, threshold.",
}

const TOC = [
  { id: "rules", label: "Two rules", level: 2 as const },
  { id: "dice", label: "Dice · 1 to 6", level: 2 as const },
  { id: "coin", label: "Coin flip", level: 2 as const },
  { id: "lottery", label: "Lottery winner", level: 2 as const },
  { id: "wheel", label: "Weighted wheel", level: 2 as const },
  { id: "threshold", label: "Threshold binary", level: 2 as const },
  { id: "reuse-bytes", label: "Reusing bytes across decisions", level: 2 as const },
]

const DICE = `// 1 to 6, uniform, biased by at most 2^-29 — safe for any game.
let r = result.randomness;
let roll = (u32::from_le_bytes([r[0], r[1], r[2], r[3]]) % 6) as u8 + 1;`

const COIN = `// 0 or 1 — exactly uniform because we use a single bit.
let r = result.randomness;
let side: u8 = r[0] & 1;
// or if you want heads/tails labels:
let heads = (r[0] & 1) == 0;`

const LOTTERY = `// Pick one of n_players. Uniform modulo bias is 2^-57 for n ≤ 2^10,
// which is invisible to any human-scale lottery.
let r = result.randomness;
let pool: u64 = u64::from_le_bytes([
    r[0], r[1], r[2], r[3], r[4], r[5], r[6], r[7],
]);
let winner_index: usize = (pool % n_players as u64) as usize;`

const WHEEL = `// Weighted segments that sum to \`total\`. Walk until the accumulator
// passes the draw. Works for any integer weights.
let r = result.randomness;
let draw = u32::from_le_bytes([r[0], r[1], r[2], r[3]]) % total;
let mut acc = 0u32;
let mut picked: usize = 0;
for (i, w) in segment_weights.iter().enumerate() {
    acc = acc.saturating_add(*w);
    if draw < acc {
        picked = i;
        break;
    }
}`

const THRESH = `// A fair coin, but with higher resolution (useful if you want to support
// biased thresholds like "yes with 37% probability" in the future).
let r = result.randomness;
let draw = u64::from_le_bytes([
    r[0], r[1], r[2], r[3], r[4], r[5], r[6], r[7],
]);
let yes = draw > (u64::MAX / 2);

// For a biased threshold p (as a fraction of u64::MAX):
let yes_biased = draw < p_as_u64;`

export default function FormulasPage() {
  return (
    <DocsPageShell
      href="/docs/integration/formulas"
      title="Outcome formulas"
      lead="Five patterns cover almost every VRF application. Pick one, paste it into your dice_callback, change the constants."
      toc={TOC}
    >
      <H2 id="rules">Two rules</H2>
      <P>
        Before the formulas, two things that apply to all of them.
      </P>
      <P>
        <b>1 · Always use{" "}</b>
        <Code>u32::from_le_bytes</Code> / <Code>u64::from_le_bytes</Code>
        <b> for larger reductions.</b> Reading bytes one-at-a-time and
        folding them yourself is an easy way to introduce modulo bias. The
        formulas below use the first 4 or 8 bytes as a single little-endian
        integer before taking the modulus.
      </P>
      <P>
        <b>2 · 32 bytes is a lot of randomness.</b> Don't feel like you have
        to reuse the whole payload for one decision. If you need{" "}
        <i>multiple</i> random choices per round (e.g. deal 5 cards), use
        disjoint byte windows — bytes 0..4 for card 1, 4..8 for card 2, and
        so on.
      </P>

      <H2 id="dice">Dice · 1 to 6</H2>
      <P>
        The canonical example. A <Code>u32</Code> modulo 6, plus 1.
      </P>
      <CodeBlock code={DICE} lang="rust" />

      <Callout type="info" title="What about modulo bias?">
        <p>
          A <Code>u32</Code> has <Code>2^32</Code> possible values. 6 does
          not divide <Code>2^32</Code> evenly — four values map to {"{1,2,3,4}"} but
          only two map to {"{5,6}"}, off by a factor of roughly{" "}
          <Code>2^-29</Code>. No human game cares at that level. If you
          need stronger uniformity (e.g. for a cryptographic protocol), use
          rejection sampling instead.
        </p>
      </Callout>

      <H2 id="coin">Coin flip</H2>
      <P>
        The cleanest one — a single bit is exactly uniform.
      </P>
      <CodeBlock code={COIN} lang="rust" />

      <H2 id="lottery">Lottery winner</H2>
      <P>
        Pick one of <Code>n</Code> entrants. Use <Code>u64</Code> so the
        pool is large enough that modulo bias stays negligible for any
        practical n.
      </P>
      <CodeBlock code={LOTTERY} lang="rust" />

      <H2 id="wheel">Weighted wheel</H2>
      <P>
        Loot drops, tier distributions, prize tables. Each segment has an
        integer weight and the sum is <Code>total</Code>. Walk the segments
        until the accumulated weight exceeds the draw.
      </P>
      <CodeBlock code={WHEEL} lang="rust" />

      <H3 id="wheel-example">Concrete example</H3>
      <P>
        Suppose you're rolling loot with weights{" "}
        <Code>[60, 30, 9, 1]</Code> for common/uncommon/rare/legendary. Sum
        is 100. A <Code>draw = 87</Code> lands in the third segment (60 +
        30 = 90 &gt; 87 after the second iteration) → picks index{" "}
        <Code>2</Code>, the "rare" bucket.
      </P>

      <H2 id="threshold">Threshold binary</H2>
      <P>
        Yes/no decisions with controllable probability. For 50/50, compare
        to <Code>u64::MAX / 2</Code>. For arbitrary probabilities, compute{" "}
        <Code>p_as_u64 = (p * u64::MAX as f64) as u64</Code> off-chain and
        pass it in.
      </P>
      <CodeBlock code={THRESH} lang="rust" />

      <H2 id="reuse-bytes">Reusing bytes across decisions</H2>
      <P>
        If a single round has to make several independent decisions (e.g.
        "pick a card, then pick a modifier, then decide if it crits"),
        split the 32 bytes into disjoint windows. This guarantees the
        decisions are mutually independent.
      </P>
      <CodeBlock
        code={`// Card (6-range) + modifier (4-range) + crit (50/50), all independent.
let r = result.randomness;
let card = (u32::from_le_bytes([r[0],  r[1],  r[2],  r[3]])  % 6)  as u8;
let mod_ = (u32::from_le_bytes([r[4],  r[5],  r[6],  r[7]])  % 4)  as u8;
let crit =  u64::from_le_bytes([r[8],  r[9],  r[10], r[11],
                                r[12], r[13], r[14], r[15]]) > u64::MAX / 2;`}
        lang="rust"
      />
      <Callout type="tip" title="Budget check">
        <p>
          32 bytes = 256 bits. That's 32 independent 1/256 decisions or 8
          independent 1-in-a-billion choices. Almost every game fits
          comfortably within a single round's payload.
        </p>
      </Callout>
    </DocsPageShell>
  )
}
