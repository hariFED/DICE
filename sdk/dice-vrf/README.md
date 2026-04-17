# dice-vrf

Rust SDK for integrating with DICE — hardware-backed VRF oracle on Solana.

DICE runs a commit-reveal protocol across physical ESP32-S3 devices for
provable randomness, bills 0.002 SOL per request, and settles on-chain via
the `78Qv6cyKkRZN2YngiLSSBCe2iyRc6jgtCs3incCaMRcv` program. This crate
gives Rust clients the instruction builders, PDA derivations, and account
decoders they need to consume that service.

## v1 vs v2: pick the right path

DICE has shipped two generations of randomness request APIs. They are
**not equivalent** and new code should **always** target v2 — v1 is kept
on chain for historical rounds but the economics are broken (see below)
and the coordinator only drives v2 today.

### v2 — channel path (use this)

A `DiceChannel` is a long-lived account created once per dApp. Each round
reuses the same PDA, which means:

- No per-round PDA creation cost.
- Coordinator and callback program are bound at `init_channel` time, so
  every round inside the channel is verifiably gated to the same oracle
  and game contract.
- Finalized rounds atomically fan out payouts via
  `claim_rewards_v2` — the coordinator passes every contributing
  `NodeVault` in `remaining_accounts` and the split lands in one TX.
- Optional CPI callback: after finalize, `deliver_callback` can fire a
  CPI into the dApp's own program. Channels with `callback_program_id ==
  default()` just transition Finalized → Idle.

Instructions you care about:

| Step | Instruction                              | Signer        |
| ---- | ---------------------------------------- | ------------- |
| 1    | `init_channel(channel_index, max_nodes, callback_program_id, coordinator)` | dApp authority |
| 2    | `fund_channel(amount)`                   | dApp authority |
| 3    | `request_randomness_auto(node_count)`    | dApp authority |
| 4    | *(coordinator)* submit_commit_v2 × N     | coordinator    |
| 5    | *(coordinator)* submit_reveal_v2 × N     | coordinator    |
| 6    | *(coordinator)* `finalize_v2` + `claim_rewards_v2` (bundled) | coordinator |
| 7    | `deliver_callback(round_id, remaining_accounts)` | coordinator or dApp |

Driver example: `tests/harness/coin_toss_driver` runs this full sequence
against live devnet hardware — read it as the canonical v2 frontend
reference.

Passive subscriber variant: the streaming VRF pattern lets dApps read a
`RandomnessFeed` PDA as a read-only account input without any
commit-reveal roundtrip. See `programs/pulse/` and
`sdk/dice-vrf/examples/subscribe_to_feed.rs`. The coordinator's feed
crank pushes new values into the feed whenever its bound channel
finalizes a new round.

### v1 — legacy request path (do not use)

The v1 API issues one `RandomnessRequest` PDA per round with matching
`CommitRecord`, `RevealRecord`, `RandomnessResult`, and `EscrowAccount`
PDAs. It works for single rounds but has two production-breaking
defects:

- **`claim_rewards` is deprecated and returns
  `V1ClaimRewardsDeprecated`**. Its per-node-call API and
  escrow-scoped `is_claimed` flag combine to pay at most one
  contributing node per round. There is no in-place fix that preserves
  the existing account schema. Use `claim_rewards_v2` instead.
- PDA cost per round is high (5 PDAs created/destroyed each request).
  Long-running dApps bleed SOL to rent if they use v1 exclusively.

The v1 instructions are still exported for historical reads and for any
third-party tooling that already depends on them. Newly written code
should treat v1 as read-only and direct all writes through the v2
channel path.

## Quick start

```rust
use dice_vrf::{DICE_PROGRAM_ID_PUBKEY, pda::channel_pda};
use solana_sdk::pubkey::Pubkey;

let authority: Pubkey = /* dApp authority */;
let channel_index: u16 = 0;

let (channel, _bump) = channel_pda(&authority, channel_index, &DICE_PROGRAM_ID_PUBKEY);
```

Use `DICE_PROGRAM_ID_PUBKEY` (compile-time `Pubkey`) rather than
`DICE_PROGRAM_ID.parse().unwrap()` — no runtime panic, no allocation,
and the SDK unit tests assert the two resolve to the same value.

## Live deployments

- devnet program ID: `78Qv6cyKkRZN2YngiLSSBCe2iyRc6jgtCs3incCaMRcv`
- example dApps: `coin_toss` (v2 request/callback),
  `pulse` (streaming subscriber)
- coordinator: hardware-backed, pinned to `rustls 0.21` and `solana-sdk
  1.18.26`; mTLS WebSocket on 8443, REST on 8080, Neon Postgres for
  round/reveal history.
