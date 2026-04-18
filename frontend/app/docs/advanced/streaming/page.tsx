import type { Metadata } from "next"
import { DocsPageShell } from "@/components/docs/PageShell"
import { H2, H3, P, Code, Ul, Li } from "@/components/docs/Prose"
import { CodeBlock } from "@/components/docs/CodeBlock"
import { Callout } from "@/components/docs/Callout"

export const metadata: Metadata = {
  title: "Streaming feed",
  description:
    "A long-lived PDA that publishes fresh randomness on a cadence — cheap reads for consumers, one hardware round per interval.",
}

const TOC = [
  { id: "why-a-feed", label: "Why a feed?", level: 2 as const },
  { id: "shape", label: "Shape of a RandomnessFeed", level: 2 as const },
  { id: "crank", label: "The publish crank", level: 2 as const },
  { id: "consuming", label: "Consuming the feed", level: 2 as const },
  { id: "staleness", label: "Staleness guards", level: 2 as const },
]

const CONSUMER = `use anchor_lang::prelude::*;
use dice::state::RandomnessFeed;

#[derive(Accounts)]
pub struct RollFromFeed<'info> {
    pub feed: Account<'info, RandomnessFeed>,
    /* ... your own accounts ... */
}

pub fn roll_from_feed(ctx: Context<RollFromFeed>) -> Result<()> {
    let feed = &ctx.accounts.feed;

    // Staleness: reject readings older than ~5 seconds (12 slots).
    let now = Clock::get()?.slot;
    require!(
        now.saturating_sub(feed.last_slot) <= 12,
        MyError::StaleFeed,
    );

    // Use the latest published randomness directly.
    let r = feed.randomness;
    let roll = (u32::from_le_bytes([r[0], r[1], r[2], r[3]]) % 6) as u8 + 1;
    // ...store/settle roll...
    Ok(())
}`

export default function StreamingPage() {
  return (
    <DocsPageShell
      href="/docs/advanced/streaming"
      title="Streaming feed"
      lead="When you need randomness faster than a full request-callback round-trip, a RandomnessFeed PDA offers a cheap read against a value that refreshes every few seconds."
      toc={TOC}
    >
      <H2 id="why-a-feed">Why a feed?</H2>
      <P>
        The standard <Code>request_randomness_auto</Code> path produces one
        round per request, takes ~4 seconds end-to-end, and charges{" "}
        <Code>0.002 SOL</Code>. That's fine for turn-based games. It's not
        fine if you have a consumer that needs randomness at sub-second
        resolution or that doesn't want to pay per read — an auction
        settlement, a high-frequency prediction market, a randomness-backed
        index.
      </P>
      <P>
        The <b>streaming feed</b> solves this. It's a PDA account that
        holds a 32-byte <Code>randomness</Code> field, refreshed by a
        "crank" on a fixed slot cadence. Consumers pay nothing to read —
        they just account-load the PDA and use whatever value is there.
        Only the crank pays.
      </P>

      <Callout type="warn" title="Trade-off">
        <p>
          A feed's randomness is correlated with <i>wall clock</i>. If your
          game uses the same feed reading twice inside a single second, the
          result will be the same. This is fine for independent queries
          from different users, not fine for multiple decisions inside one
          transaction.
        </p>
      </Callout>

      <H2 id="shape">Shape of a RandomnessFeed</H2>
      <P>
        A feed is a PDA of{" "}
        <Code>[&quot;feed&quot;, authority, feed_index]</Code> bound to a
        specific <Code>DiceChannel</Code>. The fields you read from the
        consumer side:
      </P>
      <Ul>
        <Li>
          <Code>randomness: [u8; 32]</Code> — most recent output.
        </Li>
        <Li>
          <Code>round_id: u64</Code> — which round produced it.
        </Li>
        <Li>
          <Code>last_slot: u64</Code> — slot at which the feed was last
          published. Use this for staleness checks.
        </Li>
        <Li>
          <Code>publish_interval_slots: u64</Code> — how often the crank
          expects to run.
        </Li>
        <Li>
          <Code>is_active: bool</Code> — false if the feed is paused or
          closed.
        </Li>
      </Ul>

      <H2 id="crank">The publish crank</H2>
      <P>
        A small off-chain loop (the crank) watches the bound channel for
        the next finalised round, then calls <Code>publish_feed_value</Code>{" "}
        once the interval has elapsed. If the crank calls too early, the
        program rejects with <Code>FeedPublishTooSoon</Code>. If it calls
        with the wrong channel, <Code>FeedChannelMismatch</Code>. See the{" "}
        <a
          href="/docs/reference/errors"
          className="text-[#00FF85] underline decoration-[#00FF85]/30 underline-offset-4"
        >
          errors reference
        </a>
        .
      </P>
      <P>
        You can run your own crank against your own feed — it's a few
        dozen lines of Rust / TS. The DICE-operated coordinator runs cranks
        for public feeds.
      </P>

      <H2 id="consuming">Consuming the feed</H2>
      <P>
        From a consumer program, a feed is just another account. You
        declare it in your <Code>Accounts</Code> struct, then read{" "}
        <Code>feed.randomness</Code> like any other field. No CPI, no
        callback, no channel funding.
      </P>
      <CodeBlock code={CONSUMER} lang="rust" filename="programs/my_auction/src/lib.rs" />

      <H2 id="staleness">Staleness guards</H2>
      <P>
        Because reads are free, a malicious client could try to wait for a
        favourable published value and call your program repeatedly with
        the same "stale" value. Defend with a slot-age check — reject reads
        older than some small multiple of the feed's configured interval.
      </P>
      <Callout type="tip" title="Picking your staleness bound">
        <p>
          A feed publishing every 8 slots (~3 s on mainnet) should be
          considered "fresh" for up to ~12 slots. Past that, wait for the
          next publish. Set a hard ceiling of ~40 slots even on low-traffic
          feeds.
        </p>
      </Callout>

      <H3 id="when-not-to-use-a-feed">When not to use a feed</H3>
      <Ul>
        <Li>
          <b>Single-shot high-stakes draws.</b> Use a per-request round;
          the 0.002 SOL is worth the isolation.
        </Li>
        <Li>
          <b>Per-user randomness.</b> Feeds are shared — two users reading
          at the same slot get the same bytes. Fine for public values
          (global RNG), wrong for per-user seeds.
        </Li>
      </Ul>
    </DocsPageShell>
  )
}
