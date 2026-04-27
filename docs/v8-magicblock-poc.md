# DICE v8 — MagicBlock Ephemeral Rollup POC

> **Branch:** `v8-magicblock-integration`
> **Status:** scope locked, implementation pending
> **Goal:** ship a working DICE round in under 200 ms by delegating
> `DiceChannel` into a MagicBlock ER for the duration of a round. Existing
> NodeVault economics and L1 audit trail unchanged.
> **Timebox:** 3-day "go / no-go" sprint. If green, 1.5–2 week full build.
> **Owner:** CEO + program engineer.

---

## 1. Why this branch exists

DICE rounds on L1 Solana run at ~6–8 s end-to-end (commit + reveal +
finalize × ~1.5 s confirmation each). That's the L1 BFT floor — physics, not
a bug. Anyone shipping sub-second VRF on Solana is doing one of: lying,
running off-chain push, or running on a sidechain / rollup.

MagicBlock built **the rollup**. Single-validator SVM-compat sidechain, ~10–50 ms
blocks, programs run unchanged, state commits back to L1 periodically. They
ship a software VRF today (`/developer-tools/vrf` — "provably fair within
a second, for free"). They have no hardware-backed randomness in their
ecosystem. They have an `onchain-dice` template using their software VRF
(ironically named, perfect for our co-marketing).

The strategic case (per `marketing/strategy/two-lane-launch.md`-to-be):
DICE-on-MagicBlock is the **first hardware-backed VRF on Solana ER**. Same
speed range as their software VRF, but with real-entropy provenance and
a live DePIN node fleet. Two MagicBlock gaps closed simultaneously: VRF
upgrade + first DePIN project in their ecosystem.

This branch is the engineering proof for that pitch.

---

## 2. What we know about MagicBlock's API (from docs)

### 2.1 Crate

```toml
[dependencies]
ephemeral-rollups-sdk = { version = "<latest>", features = ["anchor"] }
```

Imports:
```rust
use ephemeral_rollups_sdk::anchor::{commit, delegate, ephemeral};
use ephemeral_rollups_sdk::cpi::DelegateConfig;
use ephemeral_rollups_sdk::ephem::MagicIntentBundleBuilder;
```

### 2.2 Three instructions a host program adds

**Delegate** — transfer PDA ownership to MagicBlock's delegation program:

```rust
#[delegate]
#[derive(Accounts)]
pub struct DelegateInput<'info> {
    pub payer: Signer<'info>,
    #[account(mut, del)]
    pub pda: AccountInfo<'info>,
}

pub fn delegate(ctx: Context<DelegateInput>) -> Result<()> {
    ctx.accounts.delegate_pda(
        &ctx.accounts.payer,
        &[CHANNEL_SEED],
        DelegateConfig {
            validator: ctx.remaining_accounts.first().map(|acc| acc.key()),
            ..Default::default()
        },
    )?;
    Ok(())
}
```

**Commit** — sync state from ER → L1, account stays delegated:

```rust
pub fn commit(ctx: Context<...>) -> Result<()> {
    MagicIntentBundleBuilder::new(
        ctx.accounts.payer.to_account_info(),
        ctx.accounts.magic_context.to_account_info(),
        ctx.accounts.magic_program.to_account_info(),
    )
    .commit(&[ctx.accounts.channel.to_account_info()])
    .build_and_invoke()?;
    Ok(())
}
```

**Undelegate** — commit state AND return ownership to the program:

```rust
pub fn undelegate(ctx: Context<...>) -> Result<()> {
    MagicIntentBundleBuilder::new(...)
        .commit_and_undelegate(&[ctx.accounts.channel.to_account_info()])
        .build_and_invoke()?;
    Ok(())
}
```

### 2.3 Client-side: switch RPC, that's it

Base-layer code keeps using normal Solana RPC. ER calls go to a different
endpoint:

```typescript
const baseProvider = new AnchorProvider(
  new Connection("https://api.devnet.solana.com", { commitment: "confirmed" }),
  wallet,
);
const erProvider = new AnchorProvider(
  new Connection("https://devnet-as.magicblock.app/", { commitment: "confirmed" }),
  wallet,
);
```

Or use **Magic Router** (auto-routes by account state):
```typescript
const connection = new ConnectionMagicRouter(
  "https://devnet-router.magicblock.app/",
);
```

### 2.4 ER block time, latency

Per their marketing: "real-time, zero-fee" and "within a second" claims.
Block time isn't published precisely; community-reported as ~10–50 ms.

### 2.5 Account-size limits

Not published. Docs reference a "Resize PDA" example. Our `DiceChannel` at
`max_nodes=50` is **9853 bytes** (verified in `state/dice_channel.rs:209`),
under L1's 10 KB init limit. Very likely under any ER limit too.

### 2.6 Validator endpoints we can target

- Mainnet: AS / EU / US / TEE regions
- **Devnet AS** (closest to our 5 boards in India): `https://devnet-as.magicblock.app/`
- Localnet: `localhost:7799` for local dev

### 2.7 Composability with L1 during delegation: "Magic Actions"

MagicBlock's "Magic Actions" lets a delegated program "automatically execute
base-layer actions while delegated." Relevant because some of DICE's flow
(NodeVault payouts, treasury credits) MUST land on L1 even mid-round. Need
to read the Magic Actions doc before deciding whether NodeVault credits
happen during ER-time or only post-undelegate.

---

## 3. DICE → ER architecture (the integration plan)

### 3.1 What stays on L1, what moves to ER

| Concern | Lives where | Why |
|---|---|---|
| `programs/dice/` (the Anchor program code) | Same binary, runs on both L1 + ER unchanged | SVM-compat. The trick is pure delegation. |
| Long-lived state: `DeviceRegistry`, `NodeVault`, `RandomnessFeed` | **L1 only**, never delegated | Persistent identity + economics; ER is per-round-ephemeral. |
| `DiceChannel` (per-customer round container) | **Delegated to ER for the duration of a round, returned to L1 after finalize** | This is where the speed comes from. Channel hops in for 200 ms, hops out. |
| Treasury / reserve wallets | L1 | Long-lived. |
| `claim_rewards_v2` (NodeVault credits) | L1 (post-undelegate) | Settlement step — happens after round, in regular L1 time. |
| `submit_round_v2` (commit + reveal + finalize bundle) | **ER (during delegation)** | The hot path — runs at ER speed. |

**Key invariant:** the channel is delegated for ~200 ms (one round), not
delegated as a long-lived ER-resident account. Each round = one
delegate→run→undelegate cycle.

### 3.2 New round lifecycle (compared to v7.7)

```
v7.7 (current, all on L1, ~6-8s):
  client → init_channel             [L1, one-time]
       → request_randomness_auto    [L1, ~1.5s confirm]
                                    [coord runs commit/reveal off-chain]
       → submit_round_v2 (coord)    [L1, ~1.5s confirm]
       → claim_rewards_v2 (coord)   [L1, ~1.5s confirm]

v8 (this branch, channel delegated, ~50-200ms):
  client → init_channel             [L1, one-time]
       → delegate_channel_for_round [L1, ~1.5s — only paid once per round]
       → request_randomness_auto    [ER, ~50ms]
                                    [coord runs commit/reveal off-chain]
       → submit_round_v2 (coord)    [ER, ~50ms]   ← the speed win
       → undelegate_and_finalize    [L1, ~1.5s — commits state back]
       → claim_rewards_v2 (coord)   [L1, ~1.5s — payouts]
```

**Net latency comparison:**

| Phase | v7.7 (L1) | v8 (ER) | Delta |
|---|---|---|---|
| Channel state mutations during round | 3 × ~1.5s = 4.5s | 3 × ~50ms = 150ms | **−4.35s** |
| Delegate + undelegate hops | n/a | 2 × ~1.5s = 3s | **+3s** |
| **Net per-round end-to-end** | **~6–8s** | **~3–4s** (single-round amortized cost) | **~50% faster** |

**Important nuance:** the delegate/undelegate hops are L1 transactions
(~1.5s each). Single round on ER ≈ same total time as L1 round. **The win
shows up only when you run multiple rounds within one delegation window.**

### 3.3 The amortization play: persistent ER channels

Real win comes from **delegating once, running many rounds, undelegating
once.** For high-throughput dApps (live dice game with rolls every few
seconds), the delegate/undelegate cost amortizes:

- 1 round in ER = ~3-4s end-to-end (no win vs L1)
- 10 rounds in ER = ~3s setup + 10 × 200ms + 1.5s teardown = **~6.5 s for 10 rounds**
- 100 rounds in ER = ~3s setup + 100 × 200ms + 1.5s teardown = **~24 s for 100 rounds**

vs L1 today: 100 rounds × 7s = 700 s. **30× faster at high volume.**

This shapes the product. We sell two channel modes:

1. **L1 channels** (today's mode) — each round fully on L1. For low-volume
   dApps that just need a roll once an hour. Decentralized end-to-end.
2. **ER channels** (new) — channel delegated continuously, rounds at ER speed.
   For game frontends doing many rolls per minute. Off the L1 BFT path during
   the delegation window; commits back periodically + on undelegate.

The second mode is what attracts MagicBlock's gaming partners.

---

## 4. New instructions to add to `programs/dice/`

Three new entrypoints. All thin wrappers around `ephemeral-rollups-sdk` —
the heavy lifting is the SDK, not new logic.

### 4.1 `delegate_channel`

```rust
// programs/dice/src/instructions/delegate_channel.rs

use anchor_lang::prelude::*;
use ephemeral_rollups_sdk::anchor::delegate;
use ephemeral_rollups_sdk::cpi::DelegateConfig;
use crate::constants::SEED_CHANNEL;

#[delegate]
#[derive(Accounts)]
pub struct DelegateChannel<'info> {
    pub authority: Signer<'info>,
    /// CHECK: validated by macro + delegate_pda call below
    #[account(mut, del)]
    pub channel: AccountInfo<'info>,
}

pub fn handler(ctx: Context<DelegateChannel>) -> Result<()> {
    let auth = ctx.accounts.authority.key();
    // Reconstruct PDA seeds — channel_index lives inside the account, but
    // we can't read it here without first deserializing. The simpler call:
    // pass the channel_index through ctx if we need to assert seeds.
    let validator = ctx.remaining_accounts.first().map(|a| a.key());
    ctx.accounts.delegate_pda(
        &ctx.accounts.authority,
        &[SEED_CHANNEL, auth.as_ref()],   // partial seed — verify exact form
        DelegateConfig {
            validator,
            ..Default::default()
        },
    )?;
    Ok(())
}
```

⚠️ **Open question:** PDA seeds for `DiceChannel` are
`[SEED_CHANNEL, authority, channel_index.to_le_bytes()]`. The `delegate_pda`
helper needs the canonical seeds. Likely we need to pass `channel_index` as
an instruction arg so the macro can rebuild the seed array.

### 4.2 `commit_channel` (mid-round state sync — likely unused for our flow)

We probably don't need explicit `commit` as a separate ix. State is committed
on undelegate. Skip unless the test plan reveals a need.

### 4.3 `undelegate_channel`

```rust
// programs/dice/src/instructions/undelegate_channel.rs

use anchor_lang::prelude::*;
use ephemeral_rollups_sdk::ephem::MagicIntentBundleBuilder;

#[derive(Accounts)]
pub struct UndelegateChannel<'info> {
    pub authority: Signer<'info>,
    #[account(mut)]
    pub channel: AccountInfo<'info>,
    /// CHECK: MagicBlock-defined accounts
    pub magic_context: AccountInfo<'info>,
    /// CHECK: MagicBlock program
    pub magic_program: AccountInfo<'info>,
}

pub fn handler(ctx: Context<UndelegateChannel>) -> Result<()> {
    MagicIntentBundleBuilder::new(
        ctx.accounts.authority.to_account_info(),
        ctx.accounts.magic_context.to_account_info(),
        ctx.accounts.magic_program.to_account_info(),
    )
    .commit_and_undelegate(&[ctx.accounts.channel.to_account_info()])
    .build_and_invoke()?;
    Ok(())
}
```

### 4.4 Existing instructions: NO CHANGE

- `submit_round_v2` runs unchanged. Inside the ER, the channel is mutated
  via the same `Account<'info, DiceChannel>` borrow pattern.
- `init_channel`, `fund_channel`, `claim_rewards_v2`, `withdraw_balance`,
  `close_channel` — all stay on L1.

---

## 5. Coordinator changes

### 5.1 RPC routing per channel

The coord today has ONE `solana_rpc::SolanaRpc` instance pointed at Helius
devnet. For v8, it needs:

- **L1 RPC** (Helius devnet) — for delegate/undelegate/init/claim
- **ER RPC** (`https://devnet-as.magicblock.app/`) — for `submit_round_v2`
  during the delegation window
- A small per-channel state machine that knows which one to use right now

Implementation: add `coordinator/src/solana_rpc_er.rs` (parallel to
`solana_rpc.rs`). Adapter shape: `enum CoordRpc { L1(SolanaRpc), ER(SolanaErRpc) }`.
Or use MagicBlock's "Magic Router" endpoint as a single connection that
auto-routes — simpler, ship that first, optimize later if it's slow.

### 5.2 Channel-mode flag

Add a column to whatever per-channel record the coord keeps (currently in
`solana_ws.rs::run_dice_channel_poller`):

```rust
enum ChannelMode {
    L1Only,           // current behavior
    ErDelegated,      // new: rounds run on ER
}
```

The coord's poller checks: is this channel in `ErDelegated` mode? If yes,
use ER RPC for submit_round_v2; otherwise L1.

### 5.3 Delegate/undelegate orchestration

Two new TX flows. Simplest: trigger them from the **dApp client** (not
coord), via new SDK helpers. Coord doesn't initiate delegation.

```typescript
// SDK new helpers (sdk/dice-vrf-ts/)
await client.delegateChannelForRound({ channelIndex });
// ... rounds happen at ER speed via existing request_randomness_auto ...
await client.undelegateChannelAfterRound({ channelIndex });
```

### 5.4 Keep coord-side mTLS WS stuff unchanged

Devices and coord still talk over the same mTLS WebSocket. The ER detail
is invisible to the device firmware — it only signs commits/reveals;
where they get submitted on chain is the coord's problem.

---

## 6. SDK changes

`sdk/dice-vrf-ts/`:

- New helper: `client.delegateChannelForRound({ channelIndex, validator? })`
- New helper: `client.undelegateChannelAfterRound({ channelIndex })`
- Helper: `client.useEr({ rpc?: string })` — switches the SDK's connection
  to an ER endpoint. Used between delegate and undelegate.
- (Optional) A higher-level wrapper: `client.runErRound()` does the whole
  delegate→request→wait→undelegate flow, hiding the modal dance.

Same for `sdk/dice-vrf/` (Rust SDK) — mirror the helpers.

---

## 7. Test plan

Three tiers, each strictly gating the next.

### Tier 1 — local dev validator (1 day)

- Spin up `solana-test-validator` + a local MagicBlock localnet at
  `localhost:7799`.
- Add `ephemeral-rollups-sdk` to `programs/dice/Cargo.toml`.
- Wire `delegate_channel` + `undelegate_channel` instructions.
- Write a Rust test in `tests/harness/er_smoke/` that:
  1. Inits a channel on L1
  2. Delegates to local ER
  3. Calls `submit_round_v2` against the ER RPC (mocking 4 device contributions)
  4. Undelegates back to L1
  5. Verifies `channel.randomness` is non-zero on L1 after undelegate
- **Success criteria:** test passes; round happens; account state visible
  on both L1 (post-undelegate) and ER (during delegation).
- **Failure modes:** delegate seeds wrong, account size rejected by ER,
  CPI to MagicBlock program fails — each is a fixable concern, not a
  go/no-go signal.

### Tier 2 — devnet ER, mock devices (1–2 days)

- Deploy the v8 dice program to devnet.
- Point harness at `https://devnet-as.magicblock.app/`.
- Run `tests/harness/er_smoke/` against real MagicBlock devnet ER.
- Measure end-to-end round timing.
- **Success criteria:**
  - Round completes in <500 ms on ER (excluding delegate/undelegate hops)
  - State commits back to L1 correctly after undelegate
  - No regressions in existing v7.7 flow against same devnet program
- **Go/no-go:** if Tier 2 passes, commit to the full build. If ER round time
  is >2 s or undelegate is unreliable, halt and reconsider.

### Tier 3 — real devices, ER channel (1 week)

- Update one of the 5 boards' provisioning to point at the v8-deployed
  program (separate channel index to avoid mixing v7.7 channels).
- Run a 50-round stress against an ER-delegated channel with all 5 nodes.
- Compare latency p50/p95/p99 vs the existing
  `tests/v77_live_fleet_results/stress_50.json` baseline.
- **Success criteria:**
  - p50 round time on ER < 500 ms (including device commit/reveal RTT)
  - p95 < 1 s
  - 100% success rate over 50 rounds (or document why not)
  - On-chain state correctness after undelegate (audit trail intact)

---

## 8. Risk register

| Risk | Probability | Impact | Mitigation |
|---|---|---|---|
| ER rejects 9.8 KB DiceChannel | Low | Medium | Pre-flight in Tier 1. If rejected, reduce `MAX_CHANNEL_NODES` from 50 → ~30 (still enough for our use case). |
| `ephemeral-rollups-sdk` has breaking API changes | Low | Low | Pin version. Read the changelog before each upgrade. |
| MagicBlock devnet ER goes down during testing | Medium | Low | Skip blocked tests, retry next session. ER is theirs to maintain. |
| Magic Actions can't fire NodeVault credits during delegation | Medium | Medium | Move all NodeVault credits to L1 post-undelegate (confirmed in §3.1 — already the plan). |
| ER + L1 state diverge (delegate/undelegate race) | Low | High | Add a guard in `submit_round_v2`: refuse to finalize unless channel is currently in `ErDelegated` mode (program reads ownership state). |
| MagicBlock changes their endpoint URLs | Medium | Low | Pull endpoints from env var, not hardcoded. Already the plan for v8. |
| MagicBlock partnership doesn't materialize | High (until pitched) | Medium | This branch is also useful as a "hardware-backed VRF on any SVM rollup" technical proof — the work isn't wasted. |

---

## 9. Demo plan (the partnership pitch material)

Once Tier 3 passes, build a 60-second video:

```
[0–10 s]  Open: a Solana game UI. Player clicks "Roll Dice."
          [Frame: timer starts, "Calling DICE…"]

[10–11 s] Roll resolves. ~100 ms.
          [Frame: confetti, "Rolled: 4 (verifiable)"]

[11–25 s] Cut to side-by-side: same call to DICE on L1 takes 6 s.
          Tagline: "Same hardware. Same verifiability. 60× faster."

[25–55 s] Behind the scenes shot of the actual ESP32 box on a desk.
          Voiceover: "This is where the random comes from. Not a software
          seed. A real chip drawing entropy from physical noise.
          DICE on MagicBlock Ephemeral Rollups: the first hardware-backed
          VRF on Solana ER."

[55–60 s] CTA: "First DePIN+VRF integration on MagicBlock. Live now."
          dicelabs.net / magicblock.gg
```

Render with Remotion (the codebase already has scripts in `marketing/video-scripts/`).
60 s is the right length for crypto-Twitter retweets and partner Discords.

---

## 10. Partnership pitch — the cold email

Send only AFTER Tier 3 passes and the video exists. Target their **DevRel
lead**, not the founders. DevRel cares about ecosystem completeness; they'll
move faster than a founder triaging cold inboxes.

```
Subject: First hardware-backed VRF on MagicBlock ERs — built, working

Hi [DevRel],

Two gaps in MagicBlock's positioning we noticed:

1. Your DePIN page sells real-time DePIN on Solana, but no DePIN project
   is in your ecosystem.
2. You ship a software VRF; your dice template uses it; no hardware-backed
   alternative exists for ER-deployed games.

DICE fills both at once. We're a hardware-backed verifiable randomness
oracle running 5 ESP32-S3 nodes globally on Solana devnet today.

Last week we delegated our DiceChannel into your devnet-as ER and ran
50 rounds at p50=420ms with full on-chain audit on L1 after undelegate.
Existing software VRFs on MagicBlock don't have hardware entropy
provenance; existing hardware VRFs (us, basically) don't run at ER speed.
This unlocks both.

Demo: <link to 60s video>
Source: github.com/hariFED/DICE/tree/v8-magicblock-integration
Live network: dicelabs.net/explorer

We'd like to:
1. Be listed on /ecosystem and /solana-depin (first DePIN project both
   places)
2. Co-publish a "DICE + MagicBlock = sub-500ms hardware VRF" announcement
3. Replace `Templates/onchain-dice` with a DICE-powered version

Open to a 30-min call this week or next.

— Hari, founder, DICE Labs
[contact]
```

Direct, specific, leads with what they need before pitching what we want.
The "we already built it" hook beats every cold pitch made of slides.

---

## 11. Go / no-go criteria — the 3-day decision

After Tier 1 + Tier 2 (3 days max), one of these holds:

**🟢 Green-light full build (1.5–2 weeks):**
- Tier 1 smoke test passes locally
- Tier 2 devnet round happens in <2 s on ER
- No "this is architecturally impossible" findings
- ER doesn't reject our account size

**🟡 Yellow — investigate (extend by 2 days):**
- Tier 1 passes but Tier 2 has flakiness on devnet (could be MagicBlock-side, retry)
- Account size warnings (try with smaller `max_nodes`)
- Magic Actions story for NodeVault credits unclear (read more docs)

**🔴 Red — abandon, keep notes:**
- ER rejects 9.8 KB account and we can't reduce below 5 KB
- Delegate/undelegate latency >5 s (no amortization win)
- ephemeral-rollups-sdk doesn't compile against our anchor-lang 1.0.0 version
- MagicBlock devnet endpoints unstable (>30% failure rate on basic ops)

If red, document the findings in `docs/v8-magicblock-blocked.md`, fold the
branch back to v7.7, and pivot to **DICE Stream (the WS push)** instead —
which we control end-to-end and doesn't depend on MagicBlock's infra.

---

## 12. Implementation checklist (the first 3 days)

### Day 0 (setup, 1 hour)

- [x] Create branch `v8-magicblock-integration`
- [x] Push v7.7 work to origin
- [x] Read MagicBlock docs (ER, VRF, RFP)
- [x] Verify `DiceChannel` size (~9.8 KB at max_nodes=50, well under 10 KB L1 cap)
- [ ] Add `ephemeral-rollups-sdk` to workspace `Cargo.toml`
- [ ] Verify it compiles against current Anchor 1.0.0

### Day 1 (Tier 1 smoke test)

- [ ] Add `delegate_channel` + `undelegate_channel` instructions to `programs/dice/`
- [ ] Wire them into `programs/dice/src/lib.rs`
- [ ] Build via `anchor build --no-idl`
- [ ] Spin up local MagicBlock localnet (`localhost:7799`)
- [ ] Write `tests/harness/er_smoke/` test
- [ ] Run smoke test against localhost ER
- [ ] **Decision point:** does the round happen end-to-end?

### Day 2 (Tier 2 devnet ER)

- [ ] Deploy v8 program to devnet (separate program ID — do NOT redeploy
      over `FMwPuCjkfZXN2MuNJQiUzZC3hnxHcD8mrTuntsqA84XD`)
- [ ] Point smoke test at `https://devnet-as.magicblock.app/`
- [ ] Measure round timing (target <500 ms on ER, <2 s incl. delegate hops)
- [ ] Verify state syncs back to L1 after undelegate
- [ ] **Decision point:** is real-network ER fast + reliable enough?

### Day 3 (decision + scope-out)

- [ ] Write `docs/v8-poc-results.md` — actual numbers, what worked/didn't
- [ ] If green: open `docs/v8-build-plan.md` for the 2-week full integration
- [ ] If yellow/red: document reasons, decide on alternate path
      (DICE Stream WS push)
- [ ] Brief CEO + frontend / SDK leads on findings

---

## 13. What this branch must NOT do

To keep scope tight:

- **Don't fork the dice program.** The same `programs/dice/` binary deploys
  to L1 + runs in ER. One program, two settlement environments.
- **Don't introduce a new token.** The compensation model (per-request fees,
  70/20/10 split) stays exactly as-is. ER speed is the value-add, not new
  economics.
- **Don't migrate existing v7.7 channels.** Only NEW channels opt into ER mode.
  Existing customers keep working unchanged.
- **Don't break the `/explorer` page.** Whatever ER state is delegated, the
  L1 audit trail still has every round's outcome via `commit_and_undelegate`.
- **Don't ship marketing copy that says "fastest VRF on Solana."** Even with
  ER, MagicBlock's own software VRF is comparable speed. Our claim is
  "hardware-backed" + "ER-native," not raw speed bragging.

---

## 14. Single sentence

> **Delegate the channel for a round, run the round at ER speed, undelegate
> when done — the program code and node fleet stay exactly the same.**

Everything in this doc is the bookkeeping around that one move.

---

*Last updated: 2026-04-27*
*Branch: v8-magicblock-integration*
*Target: green-light decision in 72 hours*
