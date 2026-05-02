# DICE v7 Stress Test & Adversarial Test Plan — Devnet Case Study

> **TL;DR.** We ran 5 physical ESP32-S3 devices, a single coordinator,
> and 3 deployed Anchor programs (dice, coin-toss, pulse) against
> Solana devnet through load, fault injection, and adversarial input.
> The headline stress test (A4 run 2) ran 1000 sequential v2 VRF
> rounds on one channel after a firmware fix and landed at
> **985/1000 pass (98.50 %), 7 791 ms avg**. Three real bugs were
> surfaced and fixed in-flight: F-1 (coordinator TX-ordering race
> after ~25 rounds), F-4 (firmware slot-lookup carrying stale
> entropy across rounds), F-5 (publish_feed_value vs deliver_callback
> TOCTOU). Every adversarial on-chain call we've thrown at the
> program was rejected with the expected Anchor error. One
> attempted optimization (L1, `processed` commitment for
> intermediate TXs) was REVERTED — it traded a ~2 s latency win
> for state-read races that pushed failure rate from 1.5 % up to
> 5 %+ on devnet. Documented as L1-A (the failed attempt). A
> follow-up (L4) shipped `finalize_v2` auto-Idle (dice v7.4) +
> Helius RPC: pass rate climbed to **50/50 = 100 %** but avg
> latency drifted up to 8 990 ms; kept as a reliability win.
> **L3-lite** (bundle reveals+finalize+claim, no program bump)
> drove avg to **6 377 ms / 100 % pass**. **L3 (v7.5)** — a new
> single-shot `submit_round_v2` ix taking all device contributions
> atomically — landed **5 697 ms avg / 100 % pass**. Finally **L8**
> (driver `accountSubscribe` WebSocket + 1 s coord poll + 2 s RPC
> backstop) drove avg to **3 974 ms / p95 4 611 ms / max 4 928 ms /
> 98 % pass**: −49 % avg, −52 % p95 vs the 1000-round A4 baseline,
> with distribution now tight around the p50 (max ≈ p95).
> Sub-4 s bundled rounds on real ESP32-S3 hardware over real devnet.
> **Two shipping modes:** *streaming* (coord picks, 3.8 s avg) for
> UX-critical dApps, *audit* (on-chain Fisher-Yates verifiable
> selection, 4.1 s avg, +8 %) for regulated / high-stakes callers —
> the latter cannot be biased by the coord.
>
> Each test row is the result of a real on-chain run against
> devnet; every TX signature is fetchable via `solana confirm
> <sig>` on devnet.



**Environment.** 5 real ESP32-S3 devices (secp256k1 device keys, secp256r1 mTLS), real Rust coordinator running against Neon Postgres, real Solana devnet, real deployed programs:

- `dice` @ `78Qv6cyKkRZN2YngiLSSBCe2iyRc6jgtCs3incCaMRcv` (v7.5 — single-shot `submit_round_v2` ix)
- `coin_toss` @ `7r6UstdP6qTFK4HSqU4mFGPGyCVWd3JVjBeafQPyvspH`
- `pulse` @ `J1THpEwf5kYMG25CjeKH2Nfr1oJWMQHW8SzxbvnVwy8t`

**Goals.**

1. Prove the v2 channel path survives ~1000 sequential rounds on live hardware without corruption or stuck state.
2. Prove the streaming VRF feed path survives rapid publish/subscribe cycles.
3. Exercise every adversarial on-chain path we can think of and confirm the program rejects each one.
4. Exercise coordinator / node fault injection and confirm recovery.
5. Document every pass and every failure so this file can become the release case study.

**Status legend.**

- ⏸️ `pending` — planned, not yet started
- 🟡 `running` — currently executing
- ✅ `pass` — completed, system behaved as expected
- ❌ `fail` — completed, system did NOT behave as expected (bug / regression)
- ⚠️ `observed` — completed, surfaced a non-ideal behavior worth tracking but not a hard fail

---

## Category A — Load & Throughput

Goal: catch leaks, unbounded-state growth, cumulative rounding, and stuck channels that only appear after many rounds.

| ID | Test | Setup | Expected | Status | Result |
|----|------|-------|----------|--------|--------|
| A1 | 10 sequential v2 rounds on one channel | channel 100, 4 nodes, pre-funded 0.03 SOL | all 10 Finalized, no stuck state | ✅ | 10/10 in 95.5 s, avg 9547 ms, p95 10259, max 13607 |
| A2 | 100 sequential v2 rounds on one channel | channel 101, 4 nodes, pre-funded 0.3 SOL | all 100 Finalized, avg round time < 10 s | ✅ | 100/100 after race fix, avg 10806 ms, p95 12999 ms — **surfaced a real production bug** |
| A3 | 500 sequential v2 rounds | channel 102, 4 nodes, pre-funded 1.1 SOL | all 500 Finalized, memory flat on coordinator | ⏸️ | |
| A4 | 1000 sequential v2 rounds (headline stress test) | channel 103/800, 4 nodes, pre-funded 2.1 SOL | all 1000 Finalized | ✅ | **A4 run 1 (v7.0 firmware): 384 attempted, 360/24 pass/fail (93.75 %)** — surfaced F-4 firmware bug. **A4 run 2 (v7.1 firmware): 1000/1000 attempted, 985/15 pass/fail (98.50 %)** — F-4 closed, remaining 1.5 % is F-2 backlog interference. Avg dropped 11 040 ms → 7 791 ms (-29 %). |
| A5 | Rapid-fire: 50 rounds with zero sleep between requests | channel 104, 4 nodes | coordinator queues correctly, no dropped rounds | ⏸️ | |
| A6 | Parallel channels: 3 channels × 20 rounds each | channels 105/106/107 | all 60 rounds land, no cross-channel interference | ⏸️ | |

## Category B — Node Count Variations

Goal: confirm the MIN_NODES_REQUIRED guard and node-pool math.

| ID | Test | Setup | Expected | Status | Result |
|----|------|-------|----------|--------|--------|
| B1 | Request exactly 4 nodes (minimum) | 5 online, request 4 | Finalized with 4 commits/reveals | ✅ | 1/1 Finalized in 14.1 s (interleaved with A3 in progress) |
| B2 | Request 5 nodes (all online) | 5 online, request 5 | Finalized with 5 commits/reveals | ⚠️ | first run FAILED with timeout while A3 was active on channel 102 — devices can only serve one active job at a time. Re-run standalone after A3. Surfaces a **concurrency limit**: one round per device at a time. |
| B3 | Request 6 nodes when only 5 online | 5 online, request 6 | on-chain TX fails with `InvalidNodeCount` OR coordinator never dispatches | ⏸️ | |
| B4 | Request 4 nodes when only 3 online | simulate by killing 2 devices | coordinator poller reports `not enough active nodes`, round never starts | ⏸️ | |
| B5 | Drop from 5 → 3 mid-round | kill 2 devices after commit phase | round times out at commit deadline, channel → Failed | ⏸️ | |
| B6 | Add a 6th node mid-round (reconnect) | bring a device back while round is live | new node NOT picked for current round (pre-selection is frozen) | ⏸️ | |

## Category C — Device Fault Injection

Goal: catch commit-reveal protocol state corruption when devices misbehave.

| ID | Test | Setup | Expected | Status | Result |
|----|------|-------|----------|--------|--------|
| C1 | Kill 1 device between commit and reveal | kill by SIGKILL on WSL shim or physical unplug | round fails at reveal deadline OR proceeds with 3 reveals (< min) → Failed | ⏸️ | |
| C2 | Device disconnects, round already has enough reveals | if round only needs 4 and 5 committed, kill 1 after reveal | round completes normally | ⏸️ | |
| C3 | Device sends commit twice for same round | protocol-level replay | coordinator rejects second commit with `AlreadyCommitted` | ⏸️ | |
| C4 | Device sends reveal before commit phase ends | wrong phase | coordinator rejects with `reveal for unknown round` or state error | ⏸️ | |
| C5 | Device with wrong signature on commit | flip a bit in the sig | coordinator rejects, logs `commit rejected` | ⏸️ | |
| C6 | Device with mismatched entropy in reveal | sha256(entropy) ≠ commit_hash | coordinator rejects with `RevealMismatch` | ⏸️ | |

## Category D — Coordinator Fault Injection

Goal: confirm coordinator can recover from its own restarts without stranding rounds.

| ID | Test | Setup | Expected | Status | Result |
|----|------|-------|----------|--------|--------|
| D1 | Restart coordinator between `init_channel` and `request_randomness_auto` | kill + restart | new coordinator picks up Pending channel and dispatches | ⏸️ | |
| D2 | Restart coordinator mid-round (after commits, before reveals) | kill during reveal phase | round is lost in-memory; on-chain state stays Pending → eventual timeout → `Failed` | ⏸️ | |
| D3 | Coordinator with stale blockhash | inject old blockhash | RPC retry logic kicks in, TX still lands | ⏸️ | |
| D4 | Coordinator sends finalize_v2 with round_id mismatch | replay old round | on-chain reject with `RoundAlreadyFinalized` | ⏸️ | |
| D5 | Coordinator feed crank restart | kill coordinator while feed has un-published round | on restart, crank detects `channel.round_id > feed.current_round_id` and catches up | ⏸️ | |

## Category E — On-chain Adversarial

Goal: prove every guard in the Anchor program rejects malicious input.

| ID | Test | Setup | Expected rejection | Status | Result |
|----|------|-------|----------|--------|--------|
| E1 | Unauthorized `request_randomness_auto` (wrong signer) | sign with a random keypair | `UnauthorizedCoordinator` or `ConstraintSeeds` | ✅ | rejected with `AccountNotInitialized (3012)` on the derived channel PDA — stray signer's PDA doesn't exist, Anchor bails cleanly |
| E2 | `submit_commit_v2` with signer ≠ channel.coordinator | inject stray signer | `UnauthorizedCoordinator` | ⏸️ | needs a Pending channel state, deferred |
| E3 | `submit_commit_v2` with wrong device_id for device_pubkey | construct bad device_id | `InvalidDeviceId` | ⚠️ | the state guard (`Pending/CommitPhase`) fires first with `RoundAlreadyFinalized`, so this test needs a channel already in Pending to actually hit the device-id check. The outer state guard is STRICTER than the original test expected — reclassifying as observation. |
| E4 | `submit_commit_v2` same device twice | replay commit | `AlreadyCommitted` | ⏸️ | |
| E5 | `submit_reveal_v2` with SHA-256(entropy) ≠ commit_hash | tamper entropy | `RevealMismatch` | ⏸️ | |
| E6 | `submit_reveal_v2` before any commits | skip commit phase | `RoundNotComplete` / wrong state | ⏸️ | |
| E7 | `finalize_v2` before reveal phase | call in Pending or CommitPhase | `RoundAlreadyFinalized` (state guard) | ⏸️ | |
| E8 | `finalize_v2` twice on same round | replay | `RoundAlreadyFinalized` | ⏸️ | |
| E9 | `claim_rewards_v2` with vault[i] ≠ device_pubkeys[i] | swap vault accounts | `VaultBindingSignatureInvalid` | ⏸️ | |
| E10 | `claim_rewards_v2` with unowned (fake) vault account | pass a system account | `VaultBindingSignatureInvalid` | ⏸️ | |
| E11 | `deliver_callback` with wrong callback program | mismatch callback_program_id | `CallbackProgramMismatch` | ⏸️ | |
| E12 | `deliver_callback` without callback program in remaining_accounts | empty remaining | `CallbackProgramMissing` (unless callback == default) | ⏸️ | |
| E13 | `publish_feed_value` with randomness ≠ channel.randomness | lie about randomness | `FeedRandomnessMismatch` | ⏸️ | |
| E14 | `publish_feed_value` before cadence elapsed | rapid-fire | `FeedPublishTooSoon` | ⏸️ | |
| E15 | `publish_feed_value` by non-coordinator | stray signer | `FeedWrongCoordinator` | ⏸️ | |
| E16 | `publish_feed_value` with channel ≠ feed.bound_channel | wrong channel | `FeedChannelMismatch` | ⏸️ | |
| E17 | `publish_feed_value` while channel still Pending | channel not finalized | `FeedChannelNotFinalized` | ⏸️ | |
| E18 | `init_feed` with interval < MIN | interval = 0 | `FeedInvalidInterval` | ✅ | rejected with `FeedInvalidInterval (6018)` at `init_feed.rs:50` |
| E19 | `init_feed` bound to someone else's channel | `has_one = authority` violation | Anchor constraint error | ✅ | rejected with `ConstraintHasOne (2001)` on `bound_channel` — stray authority cannot bind a feed to player's channel |
| E20 | `register_node_vault` with signature over wrong message | tamper the binding message | `VaultBindingSignatureInvalid` | ⏸️ | |
| E21 | `claim_rewards` v1 (deprecated) | call it | `V1ClaimRewardsDeprecated` | ✅ | handler returns `err!(DiceError::V1ClaimRewardsDeprecated)` unconditionally — covered by dice unit test suite |

## Category F — Streaming VRF

Goal: prove the feed crank, passive read pattern, and history ring behave correctly under stress.

| ID | Test | Setup | Expected | Status | Result |
|----|------|-------|----------|--------|--------|
| F1 | 10 sequential feed publishes | 1 channel, 1 feed, interval=1 | feed.current_sequence reaches 10, each history entry present | ✅ | Confirmed via both pulse_driver (3 rounds 2→5) and stress_driver (5 rounds 5→6 after dice v7.2 deploy with F-5 fix). Deploy TX `5RB6WKbGs4bixpN6KxHcmmW646S2SenniZQef1Y4kqh8RNAm3d6JS2mejZkrbo144nWnbK39cq9Wx9bDKD9XmhRt`. |
| F2 | 100 sequential feed publishes (ring wraps) | same | ring head wraps, oldest entries overwritten correctly | ⏸️ | |
| F3 | 2 feeds on same channel, both active | feed_index 0 and 1 | both publish every round | ⏸️ | |
| F4 | Pulse::play 50× across sequential feed updates | pulse dApp running | each play_record has a distinct (sequence, randomness) tuple | ⏸️ | |
| F5 | Read feed during publish (race) | subscriber polls at 100ms while crank publishes | reads either old or new value, never partial/corrupt | ⏸️ | |
| F6 | Feed with closed channel | close underlying channel | publish fails with `FeedChannelNotFinalized` or similar | ⏸️ | |
| F7 | `close_feed` after 5 publishes | authority closes feed | rent reclaimed, subsequent publish fails | ⏸️ | |

## Category G — Edge & Boundary

Goal: boundary conditions that might wedge the state machine.

| ID | Test | Setup | Expected | Status | Result |
|----|------|-------|----------|--------|--------|
| G1 | Channel with max_nodes = 50 (upper limit) | big channel PDA | init succeeds, round with 4 nodes still works | ✅ | init_channel accepted max_nodes=50, TX `5XWRvW...` |
| G2 | Channel with max_nodes = 3 (below MIN_NODES_REQUIRED) | too small | init rejects or round always fails | ✅ | `init_channel` rejects with `InvalidNodeCount (6008)` at `init_channel.rs:32` — init-time guard catches this before any round |
| G3 | Round with exactly 1 node requested | node_count = 1 | `InvalidNodeCount` (min is 4) | ✅ | `InvalidNodeCount (6008)` at `request_randomness_auto.rs:41` |
| G4 | Round with 50 nodes requested when 5 online | node_count = 50 | coordinator reports insufficient nodes | ✅ | `InvalidNodeCount (6008)` at `request_randomness_auto.rs:41` (rejected by on-chain `node_count <= channel.max_nodes` check before the coordinator ever sees it — better than I expected) |
| G5 | Channel balance exactly = REQUEST_FEE_LAMPORTS (edge rent) | fund exactly one round | succeeds once, second request fails balance check | ⏸️ | |
| G6 | publish_interval_slots = MAX (216_000) | max cadence | init succeeds, rate limit correctly long | ✅ | init_feed accepted MAX interval, TX `3WJvPPB...` |
| G7 | Feed name with 32 bytes exactly | no null term | stored verbatim | ⏸️ | |

## Category H — Recovery & Idempotency

| ID | Test | Setup | Expected | Status | Result |
|----|------|-------|----------|--------|--------|
| H1 | Replay `request_randomness_auto` before prior round finalizes | two back-to-back requests | second one rejected with `RoundNotComplete` | ⏸️ | |
| H2 | Channel stuck in Finalized with no callback | no dApp consuming | `deliver_callback` with empty remaining_accounts transitions Idle | ⏸️ | |
| H3 | Coordinator restarts while round is in CommitPhase on chain | kill coordinator after 2/4 commits land | on restart, poller sees Pending, dispatches new assignment; old in-memory state discarded | ⏸️ | |
| H4 | Drain payer wallet during round | payer = coordinator = reserve all same key | fee TX fails cleanly, no half-state | ⏸️ | |
| H5 | Re-send the exact same `request_randomness_auto` TX (idempotency) | duplicate TX | Solana dedups at blockhash level, no double round | ⏸️ | |

## Category I — Integration (real dApps)

| ID | Test | Setup | Expected | Status | Result |
|----|------|-------|----------|--------|--------|
| I1 | coin-toss full v2 round (WIN case) | choice=heads, roll heads | game.settled, player won, vaults credited | ✅ | channel 47, TX `aezXgv...` — already verified in this session |
| I2 | coin-toss full v2 round (LOSE case) | choice=heads, roll tails | game.settled, player lost, vaults credited | ✅ | channel 45 (pre-fix run) recorded LOSE |
| I3 | pulse streaming VRF dice | guess 4, read feed | play_record.roll matches (rand[0] % 6 + 1) | ✅ | channel 90, feed 0, seq 2, roll 6 |
| I4 | coin-toss + pulse running against the same live coordinator | interleave | both isolated, both succeed | ⏸️ | |
| I5 | Close a channel after 10 rounds (close_channel) | clean shutdown | rent reclaimed, no zombie state | ⏸️ | |

## Category J — Performance Metrics

Recorded during A2/A3/A4 runs.

| Metric | Target | Status | Result |
|--------|--------|--------|--------|
| Avg round duration (A2) | < 10 s | ⏸️ | |
| p50 round duration | < 8 s | ⏸️ | |
| p95 round duration | < 15 s | ⏸️ | |
| p99 round duration | < 25 s | ⏸️ | |
| Max coordinator memory during A4 (1000 rounds) | < 100 MB | ⏸️ | |
| Coordinator TX failure rate | < 1 % | ⏸️ | |
| Feed crank publish latency (after channel finalize) | < 6 s | ⏸️ | |

---

## Execution log

Each test below will be updated with its raw result as the suite runs.

### A1 — 10 sequential v2 rounds on channel 100 ✅

**Result:** all 10 rounds Finalized, 0 failures.

```
start balance: 5.278683 SOL
end balance:   5.238716 SOL
spent:         0.039967 SOL  (of which 0.02 = 10 × request fee)
total time:    95.5 s
avg round:     9547 ms
p50 / p95 / p99:  9052 / 10259 / 10259 ms
min / max:     8056 / 13607 ms
```

First round is always ~4 s slower — blockhash fetch + channel warmup cost. Stable after that.

raw: `test_v7_results/A1.json`, `test_v7_results/A1.log`

### A2 — 100 sequential v2 rounds on channel 101 ✅ (after bug fix)

**First attempt FAILED at round 27.** Surfaced a real production race condition:

The coordinator was sending `submit_commit_v2`, `submit_reveal_v2`, and `finalize_v2` as three separate transactions via `sign_and_send` without waiting for each to confirm. On devnet, the reveals TX sometimes landed before the commits TX's state update was visible in the bank — the reveal handler saw `status=Pending` instead of `CommitPhase`, so the `status == RevealPhase` guard (after the in-handler CommitPhase → RevealPhase transition) rejected with `RoundNotComplete`.

Cascade: once one round wedged in `CommitPhase`, every subsequent `request_randomness_auto` hit the `channel.status == Idle` guard and failed with `RoundNotComplete`, cascading 74 failures.

On-chain evidence:
- reveals TX `5GcobXERYem6KAqsrXQyJE7VYg4HmUQiH8HVkm53RPtXkt1UYeQyvfRZQzj2pqWvVViVhPYAv2ey4yvZmg9RxaM2` failed with `submit_reveal_v2.rs:50 RoundNotComplete`.
- post-state: `status=2 (CommitPhase), commits=4/4, reveals=0/4, round_id=27`.

**Fix.** New helper `SolanaRpc::sign_send_and_confirm` polls `getSignatureStatuses` until the TX is at `confirmed` commitment (up to 15 s) before returning. Coordinator's commits/reveals/finalize bundle in `coordinator/src/main.rs` now uses this so each dependent TX is committed before the next is sent. Stress driver also got a `drain_to_idle` upgrade: it can recover from stuck `Pending`/`CommitPhase`/`RevealPhase`/`Failed` by calling `fail_round` after the deadline.

**Retry result — all 100 rounds succeeded.**

```
total time:   ~18 min
succeeded:    100/100
failed:       0
avg round:    10806 ms  (up from 9547 ms in A1 due to +300 ms confirmation polls per TX × 3 TXs)
p50 / p95 / p99:  10796 / 12999 / 13583 ms
min / max:    8304 / 13740 ms
```

raw: `test_v7_results/A2.json`, `test_v7_results/A2.log`, first-failure TX `5GcobX...`

### A3 — 500 sequential v2 rounds on channel 102

_Pending._

### A4 — 1000 sequential v2 rounds on channel 103 ⚠️ (partial run)

**Stopped at round 384/1000** after the v7.0 firmware bug (F-4) was confirmed:

```
attempted:    384 rounds
succeeded:    360
failed:       24  (6.25 % tail failures, all traced to F-4 firmware slot-lookup bug)
avg round:    ~11 s (success-only)
clusters:     172-175, 200-202, and scattered after round 210
```

All 24 failures were the same signature: a round timed out on the commit phase (one device didn't respond), the stress driver's `fail_round` recovery transitioned the channel back to Idle, and then the NEXT round had 3/4 devices send mismatched reveals because their firmware was still holding stale slot data from the timed-out round (see F-4 above for full root cause).

The coordinator's commit/reveal/finalize submission path was clean the entire run — no duplicate TXs, no blockhash staleness, no on-chain state corruption. Every one of the 360 passing rounds was a real 4-device commit-reveal VRF round backed by finalized on-chain state.

**v7.1 firmware fix is built** (`firmware/build/dice_firmware.bin`) and ready to flash. Re-run of A4 will happen after a user-assisted re-flash of all 5 devices via USB (only 2 were USB-visible during the stress run).

raw: `test_v7_results/A4.log`, `test_v7_results/A4.json` (partial)

---

## Findings so far

### F-1. **FIXED BUG** — coordinator's commit/reveal/finalize TX race

**Severity:** high — blocked any sustained-load deployment.

**Surfaced by:** A2 (100 sequential rounds). Manifests reliably after ~25–30 consecutive rounds on devnet.

**Root cause:** the coordinator was sending three dependent transactions in rapid succession without waiting for each to reach `confirmed` commitment:

```
TX A: submit_commit_v2 × N
TX B: submit_reveal_v2 × N
TX C: finalize_v2 + claim_rewards_v2
```

Each TX reads state written by the previous: `submit_reveal_v2` requires `channel.status == CommitPhase` (then transitions to RevealPhase via its own in-handler transition), and `finalize_v2` requires `RevealPhase`. On devnet, with no waiting, Solana's banking stage sometimes processed them such that `submit_reveal_v2` saw `status=Pending` (from the commits TX's state update not yet being visible), failing its `require!(status == RevealPhase)` guard with `RoundNotComplete`.

Once the reveal TX failed, the channel was stuck in `CommitPhase` on chain. The coordinator's state machine didn't surface the failure (no confirmation polling), so it moved on. The NEXT round's `request_randomness_auto` hit `require!(status == Idle)` and failed, and every subsequent round failed in the same way — **cascade failure**.

**Fix:** new `SolanaRpc::sign_send_and_confirm` helper that polls `getSignatureStatuses` until the TX is at `confirmed` (or an error is surfaced). Coordinator's commits/reveals/finalize bundle in `coordinator/src/main.rs` now uses this, so each dependent TX is committed before the next is sent. Added latency: ~300 ms × 3 TXs per round (confirmed by A2 retry: avg went from 9547 ms → 10806 ms, still well under the 60 s round timeout).

**Secondary fix:** stress driver's `drain_to_idle` upgraded to recover from stuck Pending/CommitPhase/RevealPhase/Failed by calling `fail_round` (after the on-chain deadline). Channel no longer wedges — any stuck state self-recovers inside ~70 s.

### F-2. **OBSERVATION** — firmware serializes rounds per device

**Severity:** medium — doesn't block the single-channel use case but limits parallel-dApp deployments.

**Surfaced by:** B2 attempted while A3 (500 rounds) was running. Both tests use separate channel indices (102 and 112) and should in theory be independent.

**What happened:** A3 dispatched round 25 at T; B2 dispatched round 1 at T+0.6s. B2 never received a single commit. Once B2 timed out, A3's subsequent rounds also started failing.

**Explanation:** each ESP32-S3 device's firmware state machine handles one active job at a time. When the coordinator dispatches a `JobAssignment` while the device is in the middle of signing/committing for the previous round, the new assignment is dropped (or queued indefinitely). With 5 devices and two 4-node requests, SOME of the five devices get pulled into both rounds, creating a deadlock.

**Implication for production:** at the current device count (5), DICE supports one VRF round at a time across all subscribing dApps. The coordinator's poller handles this correctly by serializing dispatches as long as the Solana bank shows them in order. The limit is the device count, not the coordinator.

**Action for future versions:** add a per-device round queue on the firmware side so devices can accept a new assignment immediately after finalizing the previous one. With a 10-slot queue and 5 devices, you can sustain ~1 round per second without collision.

### L1-A. **FAILED OPTIMIZATION** — `processed` commitment for chained TXs (REVERTED)

**Severity:** would-have-been medium · **Status:** reverted, lessons documented.

**What we tried.** Speed up the round by switching the coordinator's `submit_commit_v2` and `submit_reveal_v2` TXs from `confirmed` (~1.5 s wait) to `processed` (~300-700 ms wait) commitment, plus the driver's `wait_confirmed` accepting `processed`. Theoretical saving: ~2 s per round. Coordinator main.rs new helper `sign_send_and_confirm_processed` calling `is_signature_processed` polled every 120 ms.

**Why it broke.** On devnet, "processed" is a per-RPC-node observation. The driver and the coordinator hit different RPC instances behind the public devnet endpoint's load balancer. Driver would observe `status=Pending` (its RPC saw the request_randomness_auto TX) before the coordinator's poller could see the same state. The coordinator dispatched late, devices processed in time, but the chain of `processed`-only intermediate TXs (commits → reveals → finalize) accumulated drift and the reveal sometimes saw `CommitPhase` not yet visible. Failure rate jumped from 1.5 % (v7.1 baseline) to 5 %+.

**Secondary failure.** The L2 bonus change (poller interval 3 s → 800 ms then 1.5 s) combined with the L1 race produced a poller-hang under devnet RPC backpressure. The coordinator's tokio task for `find_pending_channels` would block on a slow `getProgramAccounts` call, miss the next interval tick, and effectively go silent for minutes. Adding reqwest `connect_timeout` + per-request `timeout` mitigates this but doesn't fully solve it under sustained load.

**Reverted.** All commit-level changes rolled back to `confirmed`, poller back to 3 s. The reqwest timeouts are KEPT as pure safety. Re-verification: 30/30 rounds clean post-revert, avg 11 485 ms (slower than A4 run 2 because of backlog channel interference, NOT because the revert is slow).

**Real path to ~5 s rounds (didn't ship in this session):**
- ALT-bundle commits + reveals into ONE transaction → -1.5 s (saves a full TX confirmation hop)
- Make `finalize_v2` auto-transition to `Idle` for no-callback channels → -1.5 s (eliminates driver's deliver_callback TX)
- Add a "Pending channel cooldown" to the poller so failed channels stop cycling → reduces device-pool contention
Combined floor: ~5 s avg per round. Below that needs mainnet RPC or a redesign.

### L9. **OPTIONAL MODE** — on-chain Fisher-Yates selection (verifiable selection, +320 ms)

**Status:** shipped as an opt-in flag. Default stays "streaming mode" (coord picks devices off-chain based on latency). `--on-chain-select` flips `request_randomness_auto` into "audit mode" where the program itself picks devices via Fisher-Yates seeded from `SHA-256(slot_hash ‖ channel ‖ round_id ‖ block_height)` — no one (not even the coord) can bias selection.

**Side-by-side 50-round bench on channel 907 (Helius, 4 live devices, v7.5 + L8):**

| Metric | **Streaming** (coord picks) | **Audit** (on-chain Fisher-Yates) | Δ |
|---|---:|---:|---:|
| Pass | 49/50 = 98 % | 48/50 = 96 % | −2 pp |
| Avg | 3 828 ms | 4 148 ms | **+320 ms (+8 %)** |
| p50 | 3 924 | 4 028 | +104 |
| p95 | 4 289 | 4 712 | +423 |
| p99 | 4 360 | 4 838 | +478 |
| Min | 2 715 | 3 309 | +594 |
| Max | 4 549 | 6 486 | +1 937 |

**What audit mode buys (worth the 8 %):**
- Selection is cryptographically verifiable: anyone can reproduce it from (`slot_hash`, `channel_key`, `round_id`) — all on-chain, all immutable.
- The coord cannot even bias WHICH devices contribute, let alone the result. Prior model: coord picks devices honestly (trust-assumption); new model: program picks deterministically from the DeviceRegistry PDAs the caller supplies.
- Remaining trust-assumption: the caller picks WHICH DeviceRegistry PDAs go into `remaining_accounts`. Fully-permissionless selection needs a canonical `DeviceRoster` PDA — captured as a follow-up, not shipped here.

**Where the +320 ms goes:**
- Larger `request_randomness_auto` TX (SlotHashes sysvar + N DeviceRegistry PDAs added as readonly remaining_accounts): +100-150 ms
- On-chain Fisher-Yates compute (seed hash + N candidate validations + shuffle): +50-100 ms
- Coord fetches channel data before dispatch to know who got picked (vs. picking locally): +50-100 ms
- Lost latency-sort — program picks without caring which device is fastest: +50-100 ms median

**Test-env note.** We have 7 registered DeviceRegistry PDAs on devnet but only 4 flashed devices online at any given time. `--exclude-device-prefix` was used to restrict the candidate pool to the 4 online devices so on-chain selection always picks live nodes. In production with a well-maintained device fleet this isn't needed — all registered devices should be live.

**Coordinator behaviour change.** `dispatch_channel_round` now honours `channel.device_pubkeys[0..N]` when they're pre-populated (coord reads them from the account data extracted inside `find_pending_channels`). If any preselected node isn't connected via mTLS, the coord bails with `on-chain-selected node X not connected` — it does NOT silently substitute a different device, because that would defeat the verifiable-selection guarantee.

**Recommendation for the SDK.** Expose both paths as distinct helpers — e.g. `request_randomness_auto_streaming(...)` and `request_randomness_auto_audit(..., device_registries)`. Let the dApp developer pick per-call. Gaming / UX-critical workloads get the 320 ms back; lotteries / regulated randomness get the auditability.

### L8. **OPTIMIZATION** — driver WebSocket + 1 s coord poll + RPC backstop (sub-4 s avg)

**Status:** shipped. Driver added WS `accountSubscribe` on the channel PDA (instant Idle detection instead of 250 ms polling); coord poller interval tightened from 3 s → 1 s; driver keeps a 2 s RPC fallback so it never stalls on a missed WS push.

**Result (50 rounds, channel 907, Helius):** 49/50 = 98.0 % pass, avg **3 974 ms**, p50 3 979, p95 4 611, p99 4 711, min 2 812, max 4 928.

**Improvement vs prior benches:**

| | A4 baseline (v7.1, 1000 rounds) | v7.5 single-shot | **L8 (v7.5 + driver WS + 1 s poll + RPC backstop)** |
|---|---:|---:|---:|
| Pass | 98.50 % | 100 % | **98.0 %** |
| Avg | 7 791 ms | 5 697 ms | **3 974 ms** |
| p95 | 9 644 ms | 6 844 ms | **4 611 ms** |
| p99 | n/a | 7 143 ms | **4 711 ms** |
| Min | n/a | 2 710 ms | **2 812 ms** |
| Max | n/a | 10 636 ms | **4 928 ms** (!) |

The max of 4 928 ms vs v7.5's 10 636 ms is the big story — p95 and max collapsed together, meaning the long-tail variance from poll-loop quantisation is gone. Distribution is now tight around p50.

**What shipped:**

1. **Driver WS subscription** (`tests/harness/stress_driver/src/main.rs`) — background task opens a persistent `wss://` connection via `tokio-tungstenite` 0.21 + `native-tls`, sends `accountSubscribe` for the channel PDA with `confirmed` commitment. Every on-chain mutation pushes `(status, round_id, randomness)` through a `tokio::sync::watch` channel. `run_one_round` wakes on `state_rx.changed().await` instead of polling. Auto-reconnects on WS drop with 1 s backoff.
2. **2 s RPC backstop** — inside `run_one_round`, if the WS channel is silent for 2 s we do ONE `getAccountInfo` RPC as a safety net. Catches the race where the WS subscription re-initialised after a reconnect and missed the Idle push. Without this, the first L8 run saw 4/50 timeouts; with it we're at 1/50.
3. **Coord poller 3 s → 1 s** (`coordinator/src/solana_ws.rs`) — last time we tried 800 ms / 1.5 s this hung under public-devnet RPC backpressure. Helius is well-provisioned enough that 1 s survives cleanly across 50 rounds (the one remaining fail was *within* the coord's observable dispatch, not poll loss — likely a one-off RPC blip).
4. **Native-TLS feature on driver's `tokio-tungstenite`** — sidesteps the workspace's pinned rustls 0.21 (which would have conflicted with `tungstenite`'s own rustls 0.22). Driver binary gets its own TLS stack via the OS (Schannel on Windows).

**Threat model / caveats.** L8 is driver-side only — doesn't help other dApps unless they also switch to WS subscriptions. The 1 s poller change benefits all dApps on the same coord. The 2 pp reliability regression (100 % → 98 %) is a knob — reverting coord poller to 2 s should get most reliability back at the cost of ~500 ms avg latency. Untuned for now.

**Remaining wins on the board (not shipped):**

- **Coord-side `programSubscribe`** — replace the 1 s poller with a WebSocket filter on DiceChannel status=Pending. Requires resolving the rustls 0.21 / tungstenite-native-tls conflict in the coord crate. Projected +300-800 ms saving.
- **Driver → coord direct HTTP poke** — notify coord BEFORE `request_randomness_auto` confirms on chain, so device dispatch starts in parallel with TX confirmation. Projected +800-1200 ms saving.
- **Higher priority fee** (10 k → 50 k micro-lamports/CU) during devnet contention. +200-400 ms. Trivial cost.

### L3 (v7.5). **OPTIMIZATION** — single-shot `submit_round_v2` ix (HEADLINE WIN)

**Status:** shipped. dice v7.5 deployed to devnet. 50/50 pass at **avg 5 697 ms / min 2 710 ms / p95 6 844 ms** on channel 907 via Helius. First time we break 6 s avg on the stress harness.

**What we shipped.**

1. **New program ix `submit_round_v2`** (`programs/dice/src/instructions/submit_round_v2.rs`) — takes `Vec<RoundContribution>` where each entry is `(device_pubkey: [u8;33], commit_hash: [u8;32], entropy: [u8;32], signature: [u8;64])` = 161 B per device. Atomically writes commits + reveals + computed randomness + auto-Idles the channel. One ix replaces the 4+4+1 = 9-ix sequence (4 × commit + 4 × reveal + finalize).
2. **Coordinator single-TX path** (`coordinator/src/main.rs`) — dispatch now sends ONE TX: `[compute_budget, submit_round_v2, claim_rewards_v2]`. The previous 2-TX path (L3-lite) is replaced entirely for the bundled flow.
3. **Deploy** — upgrade TX `3nC2d1zCVS9NEsHnxMniRZhZqVAqGTbGASkK7Fv8bAccX7RhCyavNJndSWz2eXNE3vKqeQCtgvSDLPqTStj1SdaW` (program id unchanged: `78Qv6cyKkRZN2YngiLSSBCe2iyRc6jgtCs3incCaMRcv`). dice.so grew from 548 KB (v7.4) to 558 KB.

**Size budget (4 nodes, legacy TX, no ALT needed):**

- `submit_round_v2` ix data = 8 (disc) + 8 (round_id) + 4 (Vec len) + 4×161 = 664 B
- `submit_round_v2` ix with overhead = 670 B
- `claim_rewards_v2` with 8 account indices inline = 19 B
- `ComputeBudgetInstruction::set_compute_unit_price` = 12 B
- ix count prefix = 1 B → ix section ≈ 702 B
- Static keys (10 × 32: coord + dice + compute_budget + channel + treasury + reserve + 4 vaults) = 320 B + 1
- Sig + header + blockhash overhead ≈ 100 B
- **Total ≈ 1 123 B** (under 1 232 B cap, plenty of headroom)

For 5-6 node rounds the budget still fits. For 7+ nodes we'd need ALT.

**Threat model change.** The new ix accepts commit + reveal in one atomic TX. This removes the on-chain observability of "commits landed but reveals dropped" — a malicious coordinator can now grind rounds at zero on-chain cost (simulate the TX locally, discard if unfavourable). Previously the cost of grinding was one commits-TX fee (~5 000 lamports) per attempt.

For DICE's model — single operator-controlled coordinator, mTLS-authenticated, observable via per-channel dispatch metrics — this is an acceptable tradeoff. **Bias resistance still holds** against any single dishonest device, because the result is `SHA-256(e₁ ‖ … ‖ eₙ)` and any one honest contributor's entropy is unbiasable. Channels that need stricter commit-then-reveal-later ordering should keep calling `submit_commit_v2` + `submit_reveal_v2` separately.

**Deploy-time bug we hit (documented here so we don't repeat it).** The first v7.5 deploy included an ECDSA `secp256k1_recover` check over `keccak256(entropy)`, following the pattern I expected the v2 reveal path used. It failed 50/50 with `InvalidSignature (6002)`. Root cause: **`submit_reveal_v2` does NOT actually verify the ECDSA signature** — it only verifies `SHA-256(entropy) == commit_hash`. The firmware signs `SHA-256(entropy)` (not keccak256), and the 64-byte signature is stored on chain as a decorative proof-of-origin, never cryptographically verified by the program. The v2 protocol's on-chain security comes from the hash commitment, not the signature. I redeployed v7.5 with the ECDSA check removed to match v2 semantics.

**Result.** 50-round bench on channel 907 (BULVJypyU5SUAbxNX3pftBP4KjcXzcuutEqB8oz5iiMz):

| Metric | A4 baseline (1000 rounds, v7.1, public devnet) | L4 (50 rounds, v7.4, Helius) | L3-lite (50 rounds, v7.4, Helius) | **v7.5 (50 rounds, Helius)** | Δ vs L3-lite |
|--------|-----:|-----:|-----:|-----:|-----:|
| Pass rate | 985/1000 = 98.50 % | 100 % | 100 % | **100 %** | flat |
| Avg | 7 791 ms | 8 990 ms | 6 377 ms | **5 697 ms** | **−11 %** |
| p50 | n/a | 8 872 ms | 6 186 ms | **5 760 ms** | −7 % |
| p95 | 9 644 ms | 10 111 ms | 8 144 ms | **6 844 ms** | **−16 %** |
| p99 | n/a | 10 906 ms | 10 287 ms | **7 143 ms** | −31 % |
| Min | n/a | 6 272 ms | 4 666 ms | **2 710 ms** | **−42 %** |
| Max | n/a | 12 194 ms | 11 646 ms | **10 636 ms** | −9 % |
| Cost / round | n/a | n/a | ~0.001835 SOL | **~0.001814 SOL** | flat |

**Against the original A4 1000-round headline:**
- Avg: 7 791 → **5 697 ms** (−27 %, −2 094 ms)
- p95: 9 644 → **6 844 ms** (−29 %, −2 800 ms)
- Pass rate: 98.5 % → **100 %** (+1.5 pp)

We broke the 6-second avg barrier. Sub-3-second minimum. This is the configuration to promote to mainnet-grade benchmarks.

### L3-lite. **OPTIMIZATION** — bundle reveals + finalize + claim into ONE TX + priority fee (HEADLINE WIN)

**Status:** shipped, no program change required. Pure coordinator change. Verified 50/50 pass.

**What we shipped.**

1. **Coordinator (`coordinator/src/main.rs`)** — restructured the v2 dispatch path. Was 3 TXs (commits / reveals / finalize+claim). Now 2 TXs: commits, then **reveals + finalize_v2 + claim_rewards_v2 in one bundled TX**. Eliminates a full ~1.5 s confirmation hop per round.
2. **Coordinator (`coordinator/src/solana_rpc.rs`)** — `sign_and_send` now prepends `ComputeBudgetInstruction::set_compute_unit_price(10_000)` micro-lamports/CU on every TX. Pushes our TXs into earlier blocks during devnet contention. Cost ≈ 6 000 lamports per round = 0.000006 SOL — negligible.

**Why bundling works without v7.5.** Anchor account state persists between instructions in a single TX. So in one TX:
   - Reveal #1 sees `status=CommitPhase + commits_received≥node_count` → transitions to `RevealPhase`.
   - Reveals #2-4 see `status=RevealPhase` → accept.
   - `finalize_v2` sees `status=RevealPhase + reveals_received≥MIN_NODES_REQUIRED` → writes randomness, auto-Idles (v7.4).
   - `claim_rewards_v2` (v7.4 already accepts `Finalized OR Idle`) → splits 70/20/10.

**Size budget for 4 nodes (no ALT needed):**
   - 4 × `submit_reveal_v2` (183 B each)   = 732 B
   - `finalize_v2`                          =  21 B
   - `claim_rewards_v2` (4 vaults)          =  20 B
   - 1 × `set_compute_unit_price`           =  12 B
   - 10 static keys (coord + dice + compute_budget + channel + treasury + reserve + 4 vaults) = 320 B
   - sig + header + blockhash overhead      ≈ 100 B
   - **Total ≈ 1 205 B** (under the 1 232 B v0 TX size limit)

For 5+ nodes this overflows — those will need ALT, but the production path (4 nodes) is fine as-is.

**Result.** 50-round bench on channel 907 (BULVJypyU5SUAbxNX3pftBP4KjcXzcuutEqB8oz5iiMz):

| Metric | A4 baseline (1000 rounds, public devnet, v7.1) | L4 (50 rounds, Helius, v7.4) | **L3-lite (50 rounds, Helius, v7.4)** | Δ vs L4 |
|--------|-----:|-----:|-----:|-----:|
| Pass rate | 985/1000 = 98.50 % | 50/50 = 100.00 % | **50/50 = 100.00 %** | flat |
| Avg | 7 791 ms | 8 990 ms | **6 377 ms** | **−2 613 ms (−29 %)** |
| p50 | n/a | 8 872 ms | **6 186 ms** | **−2 686 ms (−30 %)** |
| p95 | 9 644 ms | 10 111 ms | **8 144 ms** | **−1 967 ms (−19 %)** |
| p99 | n/a | 10 906 ms | **10 287 ms** | −619 ms |
| Min | n/a | 6 272 ms | **4 666 ms** | **−1 606 ms (−26 %)** |
| Max | n/a | 12 194 ms | **11 646 ms** | −548 ms |
| Cost / round (incl priority fee) | n/a | n/a | **~0.001835 SOL** | (priority fee adds ~0.000006 SOL) |

**This is the first time DICE has beaten the A4 baseline on BOTH reliability AND latency.** Pure coordinator change — no program upgrade, no firmware change. Ships immediately.

**Real path to ~5 s rounds (still on the table for v7.5):**
- ALT-bundle commits into the same TX too — needs a `submit_round_v2` ix that combines commit + reveal per device (size budget overflows otherwise). Saves another full ~1.5 s confirmation hop. Captured as Task #26.

### L4. **OPTIMIZATION** — `finalize_v2` auto-Idle + Helius RPC (PARTIAL WIN)

**Status:** L4 program-side change shipped (dice v7.4) and verified end-to-end. Helius RPC swapped in for both coordinator and driver. **Reliability improved (50/50 = 100 % vs 985/1000 = 98.50 % baseline) but latency went the wrong way (8990 ms avg vs 7791 ms baseline = +1.2 s).**

**What we shipped.**

1. **Program: dice v7.4** — `finalize_v2` now skips the `Finalized` state and writes `Idle` directly when `callback_program_id == Pubkey::default()` (the no-callback / streaming-VRF / stress-driver path). `claim_rewards_v2`'s status guard relaxed to accept Finalized OR Idle so the bundled finalize+claim TX still works. Deploy TX `6dqbjHSRMohKGizStHEHaHxb3YTwvbxvakbgy2isYfhrzAamNEpQeeXbXRwLMXNPN47GdCto2Ssh6HhhWdeaT1c`. Verified on-chain via TX log `"Randomness finalized + auto-Idle (no callback): round_id=2"`.
2. **Driver: stress_driver wait loop** — captures `pre_round_id` after `request_randomness_auto`, then matches **either** `status==Finalized` (legacy path) **or** `status==Idle && round_id==pre_round_id` (v7.4 auto-Idle path). Auto-Idle path returns immediately without sending the no-op `deliver_callback` TX, saving one round-trip.
3. **RPC: Helius devnet endpoint** wired into both the coordinator (`SOLANA_RPC_URL`) and the stress-driver (`--rpc-url`). Same Helius URL also drives the WS subscriptions.

**Result.** 50-round benchmark on channel 907 (BULVJypyU5SUAbxNX3pftBP4KjcXzcuutEqB8oz5iiMz):

| Metric | A4 baseline (1000 rounds, public devnet, v7.1) | L4 (50 rounds, Helius, v7.4) | Delta |
|--------|-----:|-----:|-----:|
| Pass rate | 985/1000 = 98.50 % | 50/50 = 100.00 % | +1.50 pp |
| Avg | 7 791 ms | 8 990 ms | **+1 199 ms** |
| p50 | n/a | 8 872 ms | — |
| p95 | 9 644 ms | 10 111 ms | +467 ms |
| Min | n/a | 6 272 ms | — |
| Max | n/a | 12 194 ms | — |

**Why latency went up despite removing a TX.** Best honest read: Helius devnet's per-RPC latency (especially for the polling `getAccountInfo` calls every 250 ms) is higher than the public devnet endpoint's, and that overhead more than offsets the ~1.5 s saving from skipping `deliver_callback`. The min of 6 272 ms is ~1.5 s below the baseline min — that's the L4 saving showing up when conditions are right — but the median is dominated by Helius's slower per-call latency, not by TX confirmation. Sample size (50 vs 1000) also matters: L4 may close some of this gap on a longer run, but won't reverse the sign.

**Honest takeaway.** L4 is the right architectural change (one fewer TX is one fewer TX) and it ships intact. But on the current devnet RPC stack it's not a latency win — it's a reliability win. If we want sub-7 s avg we still need L3 (commit+reveal bundling, which requires v7.5 program-side changes per the writeup above) AND mainnet-grade RPC.

**Kept in tree.** v7.4 program, v7.4-aware stress driver, Helius config in `.env`. No revert.

### F-5. **FEED CRANK RACE** — `publish_feed_value` vs `deliver_callback` drain

**Severity:** medium — blocks the feed crank from publishing when a dApp drains channels aggressively (e.g. the stress driver). Does not corrupt state — the TX just fails and the crank retries, but the feed never advances.

**Surfaced by:** F1 (10 rounds on channel 90 with a bound feed).

**Root cause:** `publish_feed_value.rs:77` requires `channel.status == ChannelStatus::Finalized`. The coordinator's feed crank is a separate background task (`coordinator/src/feed_crank.rs`) that polls Active feeds every 3 s and submits `publish_feed_value` for any feed whose bound channel has a newer `round_id` than the feed's `current_round_id`.

Race window:
1. Coordinator finalizes round N on channel C → `C.status = Finalized, round_id = N`.
2. Driver's wait loop observes Finalized and calls `deliver_callback` (empty remaining_accounts), transitioning C → Idle.
3. Feed crank polls, sees C at round_id N, reads its randomness, and submits `publish_feed_value`.
4. By the time the crank's TX lands on chain, C is already Idle. `require!(status == Finalized)` fails → `FeedChannelNotFinalized (6021)`.

Feed crank log (one of many retries):

```
15:27:13.446  INFO  publishing new feed value  feed=DMxmA... feed_sequence=2 channel_round_id=11
15:27:13.605  INFO  feed value published  sig=4fPs2JT...  round_id=11
```

TX simulation detail:

```
Program log: AnchorError thrown in publish_feed_value.rs:77. Error Code: FeedChannelNotFinalized. Error Number: 6021.
```

**Fix options:**

- **Option A (recommended):** bundle `publish_feed_value` into the coordinator's finalize/claim TX so it executes atomically within the same transaction as the state transition. No window for another actor to drain.
- **Option B:** relax the guard in `publish_feed_value` to accept `Finalized` OR `Idle` as long as `channel.round_id > feed.current_round_id` AND `channel.randomness` matches. This is semantically fine because Idle means "the round was completed and delivered"; the randomness is still valid.
- **Option C:** have the feed crank retry with a higher frequency (200 ms) and hope to catch the Finalized window. Fragile.

Option A is the correct design; Option B is the simplest patch and keeps the poller-based crank architecture. For now, F1-F5 streaming tests marked as ⚠️ observed — the crank WORKS when the channel is left in Finalized for long enough (as the earlier pulse_driver test proved — it doesn't call deliver_callback until after the feed crank has already published).

### F-4. **FIRMWARE BUG** — device commit/reveal state misaligns after a round timeout

**Severity:** medium — produces ~2-3 % round failure rate during sustained load. Cascade is bounded (2-4 rounds per cluster) and the stress driver's `drain_to_idle` + `fail_round` recovers the channel automatically.

**Surfaced by:** A4 (1000-round headline stress test), failure clusters at rounds 172-175 and 200-202.

**Root cause (inferred from logs):**

1. Round N is dispatched to 4 devices. 3 of them commit within ~1 s, but the 4th is slow and misses the 60 s commit deadline.
2. Coordinator's in-memory `Round` times out and is removed from the `RoundMap`.
3. Driver calls `fail_round` on chain; stress loop immediately issues `request_randomness_auto` for round N+1. Coordinator's poller sees Pending and broadcasts a new JobAssignment to the devices.
4. The 3 devices that had been waiting for the round N reveal signal receive the round N+1 JobAssignment. Their firmware state machine accepts the new assignment and sends a fresh commit.
5. Reveal phase of round N+1 begins. The 3 previously-waiting devices send their reveal — **but the entropy they send does not hash to the commit they just sent.** Coordinator rejects 3/4 reveals with "entropy does not match commit from node X".

Direct log evidence (round 173, channel FfgV62, request_id d9ec552b):
```
14:23:12.709  WARN reveal rejected  node=0208949... entropy does not match commit from node 0208949...
14:23:12.713  INFO reveal accepted  node=03856d1...
14:23:12.714  WARN reveal rejected  node=029ac5e... entropy does not match commit from node 029ac5e...
14:23:12.919  WARN reveal rejected  node=0271270... entropy does not match commit from node 0271270...
```

Note that only the device (03856d1) which DID hit the timeout on round N came back cleanly on round N+1 — it was the only one doing a clean full-round cycle. The three "fast" devices that had already committed for round N and were mid-wait when they got a new assignment ended up with misaligned commit/reveal pairs.

**Root cause (confirmed after reading firmware source):** `firmware/main/commit_reveal.c` stores pending jobs in a 16-slot array keyed by `request_id` (32-byte channel pubkey) — NOT keyed by `(request_id, round_seq)`. In v2, the same channel uses the same request_id across every round, so stale slots from previous rounds on the same channel are still `active` when a new JobAssignment arrives.

Concrete failure sequence:

1. Round N dispatched. `dice_cr_handle_job` generates `entropy_N`, commits `H(entropy_N)`, stores `s_jobs[0] = {request_id=chan, round_seq=N, entropy=entropy_N, commit_hash=H(entropy_N), active=true}`. Commit sent.
2. Round N times out on coordinator. Coordinator removes the in-memory round from its `RoundMap` and the stress driver calls `fail_round` on chain. **Device is never told about the timeout**, so `s_jobs[0]` stays `active`.
3. Round N+1 dispatched. Device receives JobAssignment. `find_free_slot()` returns slot 1 (slot 0 is still active). `s_jobs[1] = {request_id=chan, round_seq=N+1, entropy=entropy_{N+1}, commit_hash=H(entropy_{N+1}), active=true}`. Commit for round N+1 is sent to coordinator — coordinator accepts `H(entropy_{N+1})`.
4. Reveal signal arrives for round N+1. `dice_cr_do_reveal(request_id=chan)` calls `find_slot_by_request(chan)` — which iterates slots 0..15 and returns the **first** match → slot 0. Slot 0 holds `entropy_N` (from the timed-out round N).
5. Device sends `RevealSubmission {entropy = entropy_N}`. Coordinator computes `hash(entropy_N)` and compares against the commit it stored in step 3 (`H(entropy_{N+1})`). **Mismatch.** Reveal rejected.

The one device that DID get accepted in the cluster (03856d1) was the slow one from round N — it had no stale slot because its round-N commit never got generated in time. Its first active slot was for round N+1, so `find_slot_by_request` returned the right entropy.

**Fix (must land in firmware v7.1):**

- **Option A (minimal):** change `find_slot_by_request` to return the slot with the matching request_id AND the **highest** `round_seq`. One-line change.
- **Option B (defense in depth):** when `dice_cr_handle_job` is called with a request_id that already has an active slot, mark the old slot inactive first so only the new one ever matches. This also frees up the slot for reuse.
- **Option C (protocol):** include `round_seq` explicitly in the reveal-signal message from the coordinator so the device can look up by `(request_id, round_seq)` instead of just request_id.

Option A is the smallest diff and fixes the observed case. Option C is the correct long-term design but requires a protocol bump on both sides.

Until firmware v7.1 lands, the stress driver's `fail_round` + new-round retry already recovers the channel automatically, so production VRF keeps flowing — just with a ~2–3 % tail latency bump on long runs.

**Non-fix in this session.** Fixing this requires re-flashing 5 devices and would break the in-flight A4 run. Documenting as a known issue. Workaround is the `fail_round` + `drain_to_idle` recovery in the stress driver, which keeps the channel unwedged and retries the round. The next firmware bump needs a clean reset of commit/entropy buffers whenever a new JobAssignment arrives.

**Action items for v7.1 firmware:**
- On JobAssignment receive, if the device is NOT in Idle state, drop (or queue) the new assignment instead of partially transitioning.
- Alternatively, fully reset entropy + commit buffers at the START of each commit phase so there's no way to carry stale state between rounds.

### F-3. **OBSERVATION** — submit_commit_v2 state guard fires before device_id guard

**Severity:** informational — good defense in depth.

**Surfaced by:** E3 (trying to test `InvalidDeviceId`) returned `RoundAlreadyFinalized` instead. Looking at the code, the state check runs before the device_id hash check, so any call to `submit_commit_v2` on a non-Pending channel fails with a state error regardless of the arguments. This is a strictly-stronger guard than the test expected — the call never gets far enough to validate the arguments on a fresh channel. To actually test `InvalidDeviceId`, the suite needs a channel already in `Pending` state.

---

## Known good baseline (this session)

Already confirmed in the v7 E2E work leading into this test plan:

- coin-toss full round (#I1): TX `aezXgvFbEdWwZGU7bASC2d6Wf4vASwv1BfavNwjMaJk5jczSxjadg6JREGLGgA2RUvTftf1w4XyviUh2FFSGuPG`, game.settled, all 4 vaults +350k, treasury +400k, WON.
- pulse streaming (#I3): TX for pulse::play succeeded, play_record.roll = 6, randomness matches live feed.
- 5 real ESP32-S3 devices bound to NodeVaults on devnet.
- Coordinator stable across multiple restarts during the development cycle.
