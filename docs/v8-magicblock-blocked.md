# v8 MagicBlock POC — BLOCKED at Phase 1

> **Status:** 🔴 Blocked. Cannot proceed.
> **Blocker found:** 2026-04-28 (~45 min into POC, well within the 3-day budget)
> **Branch:** `v8-magicblock-integration` (kept open for future re-attempt)
> **Recommendation:** Pivot to **DICE Stream (WS push)** for the ms-latency story.

---

## TL;DR

`ephemeral-rollups-sdk 0.10.0` through `0.11.2` (current) all pull in
`magicblock-magic-program-api 0.8.x` as a transitive dependency. That crate
calls `Pubkey::as_array()` — a method introduced in **`solana-pubkey 4.x`**.

`anchor-lang` (every released version including the latest `1.0.1`) hard-locks
to **`solana-pubkey 3.x`**, which has no `as_array()` method.

Cargo cannot resolve the dual-version graph because the `Pubkey` returned by
the magic-program-api's own `crate::ID` constant comes from one version,
while the impl block visible at the call site comes from the other. They are
two distinct types at the compile level. **No version of MagicBlock's SDK is
currently compatible with any released Anchor version.**

This isn't a configuration problem. It's a SDK-side compat gap that has to
be resolved by MagicBlock or by Anchor — not by us.

---

## What we tried

1. **`ephemeral-rollups-sdk 0.11.2` (latest) + `anchor-lang 1.0.0`** →
   `error[E0599]: no method named 'as_array' found for struct 'Pubkey'`
   in `magicblock-magic-program-api-0.8.8/src/pda.rs:7`.

2. **`ephemeral-rollups-sdk 0.10.9`** → same error. Cargo still pulls
   `magicblock-magic-program-api 0.8.8` (only `0.8.x` line published).

3. **`anchor-lang 1.0.1` (latest patch)** → no change.
   `cargo tree --invert solana-pubkey@3.0.0` confirmed Anchor 1.0.1 itself
   pulls solana-pubkey 3.x directly:
   ```
   solana-pubkey v3.0.0
   ├── anchor-lang v1.0.1
   ```

4. **`ephemeral-rollups-sdk 0.10.0` (oldest 0.10)** → still gets
   magic-program-api 0.8.8 via SDK's own `^0.8` constraint.

---

## Why the version-pin tricks won't fix it

| Approach | Why it doesn't work |
|---|---|
| Pin older ER SDK | All 0.10.x and 0.11.x versions of ER SDK depend on `magicblock-magic-program-api ^0.8`. Cargo always picks 0.8.8 (latest 0.8.x). |
| Pin older `magicblock-magic-program-api` directly | ER SDK rejects pre-0.8 magic-program-api at compile (different API surface). Would require ER SDK <= 0.8.x, which is from before the macro-based `#[delegate]` pattern landed. We'd lose the ergonomics that make this integration a 200-line job. |
| Force `solana-pubkey 4.2.0` via `[patch.crates-io]` | Cargo's `[patch.crates-io]` only accepts path or git overrides, not different crates.io versions. Even if we forked solana-pubkey to a git URL, anchor-lang's macros would break against the different Pubkey type. |
| Bump `anchor-lang` past 1.0.1 | No version exists. 1.0.1 is the current head. Anchor maintainers haven't yet bumped to solana-pubkey 4.x. |
| Drop `solana-program = "3"` workspace pin | Doesn't help — anchor-lang itself pulls solana-pubkey 3.x as a direct dep regardless of our workspace setting. |

---

## What this means for the partnership pitch

The pitch from `docs/v8-magicblock-poc.md` (the strategic case) is still
valid. The technical execution simply has a 1–4 week external-dependency wait
attached to it, and that wait isn't something we can engineer around.

**Two paths forward:**

### Path A — Wait for MagicBlock to fix the SDK

What needs to happen on their side (any one resolves it):
- They release `magicblock-magic-program-api 0.9.x` that uses `solana-pubkey 3.x` for compat
- They publish a `solana-pubkey-compat` shim crate that re-exports both Pubkey types
- They ship an Anchor-native crate that bypasses magic-program-api entirely
- Anchor publishes a 1.1.0 that bumps to solana-pubkey 4.x — at which point
  ER SDK 0.11.x compiles

**Estimated wait:** 2–8 weeks. Past patterns suggest fixes ship within a
patch cycle once a real customer reports it. Filing an issue against
`https://github.com/magicblock-labs/ephemeral-rollups-sdk` with this exact
diagnosis would accelerate it.

### Path B — Pivot to DICE Stream (recommended)

Already designed in our earlier latency analysis. WS push from coord to
dApp clients with on-chain audit trail at our existing cadence.

| | Path A (wait for ER SDK fix) | Path B (DICE Stream now) |
|---|---|---|
| Time to ms-latency demo | 2–8 weeks (their schedule) | 1 week (our schedule) |
| Trust model | MagicBlock validator infra | DICE coordinator |
| Latency (game frontend) | ~50–200 ms via ER blocks | ~30–80 ms via WS |
| Decentralization story | Better — uses Solana SVM exec | Worse — coord is a single point |
| Audit trail | Auto via undelegate | Via existing publish_feed_value |
| Blocker risk | External (we can't unblock) | Zero (we own everything) |

The latency targets are essentially the same. **The trust model is
different but explainable** ("trust DICE coord live, audit on chain
post-hoc" — same pattern Pyth Hermes uses).

---

## Recommendation

1. **Pivot to DICE Stream** for the ms-latency demo. Ship in 1 week. Same
   marketing claim ("first hardware-backed sub-100ms VRF on Solana") with
   different execution.
2. **File a GitHub issue** on `magicblock-labs/ephemeral-rollups-sdk`
   describing this exact compat gap. Cite the dep tree. Ask if they have a
   fix in flight.
3. **Keep the `v8-magicblock-integration` branch open** — `docs/v8-magicblock-poc.md`
   captures the full integration plan. When the SDK gap closes, this branch
   resumes from Phase 1.
4. **Don't lead the partnership pitch with the ER integration.** Lead with
   what's live: the running fleet, the live network. ER is a "we'll add
   this in 4 weeks" footnote, not the headline.

The "first DePIN on MagicBlock + first hardware-backed VRF on ER" pitch
remains the right partnership story long-term, but right now we don't have
the integration to back it. Pitching ER integration before we've shipped it
risks the partnership conversation if/when their team tries to verify and
hits the same wall.

---

## Re-attempt protocol

When MagicBlock publishes a fix:

1. Watch crates.io for `magicblock-magic-program-api 0.9.0+` OR
   `ephemeral-rollups-sdk 0.12.0+` OR `anchor-lang 1.1.0+`
2. On any of those, retry the Phase 1 compile probe:
   ```bash
   cargo add ephemeral-rollups-sdk --features anchor
   cargo check --release -p dice
   ```
3. If green, the rest of the v8 plan in `docs/v8-magicblock-poc.md` is
   shovel-ready (instruction stubs, test plan, demo script all written).

---

## What we kept from this attempt

- `docs/v8-magicblock-poc.md` — the full integration plan (still relevant)
- `docs/v8-magicblock-blocked.md` — this doc, the blocker analysis
- v8 branch — preserved for re-attempt
- ~1 hour of investigation that would have otherwise been spent in week 2
  of a doomed build

The 3-day go/no-go gate worked exactly as designed. Worst-case avoided.

---

*Last updated: 2026-04-28*
*Branch: v8-magicblock-integration (kept open)*
*Next move: pivot to `docs/v8-stream-poc.md` — the WS push design.*
