# DiceChannel — Reusable PDA Design

> **Status:** Design review (not yet implemented)
> **Author:** DICE team
> **Date:** 2026-03-27

---

## Problem

The current design creates **N new PDA accounts per round** (CommitRecord, RevealRecord per node + RandomnessResult). This is expensive:

| Nodes per round | PDAs created | Rent cost | Coordinator pays |
|----------------|-------------|-----------|-----------------|
| 7 | 16 | ~0.036 SOL | ~0.031 SOL |
| 20 | 42 | ~0.090 SOL | ~0.085 SOL |
| 50 | 102 | ~0.210 SOL | ~0.205 SOL |

At 1000 rounds/month with 7 nodes: coordinator bleeds **31 SOL/month** ($4,650).

The developer pays 0.002 SOL per request but the coordinator subsidizes ~15x that. This is unsustainable.

---

## Proposed Solution: DiceChannel

A persistent, reusable PDA that stores everything inline. The developer creates it once and uses it for all future requests.

### How it works

```
ONE TIME (developer pays rent once):
  init_channel(max_nodes=7)
  → Creates DiceChannel PDA (~0.008 SOL rent)
  → Stores: commits[], reveals[], result, status, balance — all inline
  → Developer also funds the channel with SOL for protocol fees

EVERY REQUEST (just TX fees + protocol fee):
  request_randomness(node_count=7)
  → Resets channel to Pending
  → Deducts 0.002 SOL from channel balance
  → Zeroes commit/reveal arrays
  → Increments round_id counter

  submit_commit(round_id, device_id, commit_hash)
  → Writes to channel.commits[i] inline (no new PDA)

  submit_reveal(round_id, device_id, entropy, signature)
  → Writes to channel.reveals[i] inline (no new PDA)

  finalize_randomness(round_id)
  → Computes randomness from inline reveals
  → Writes result to channel.randomness
  → Sets status to Finalized

  deliver_callback()   ← NEW: separate from finalize
  → CPI calls developer's dice_callback
  → Sets status to Idle (ready for next request)
```

### Cost comparison

| | Current (new PDAs) | DiceChannel (reusable) |
|--|--------------------|-----------------------|
| **Developer setup** | 0 SOL | ~0.008 SOL (one time) |
| **Developer per request** | 0.005 SOL | 0.002 SOL (fee only) |
| **Coordinator per request** | 0.031 SOL | 0.00003 SOL (TX fees only) |
| **1000 requests** | 36 SOL total | 2.01 SOL total |

**18x cheaper. Coordinator is sustainable.**

---

## Developer-Chosen Node Count

The developer picks `max_nodes` at channel creation and `node_count` per request:

```rust
// Init: allocate for up to 20 nodes (one-time rent ~0.015 SOL)
init_channel(max_nodes=20)

// Request with default security
request_randomness(node_count=7)   // 0.002 SOL fee

// Request with higher security (more nodes = more entropy sources)
request_randomness(node_count=20)  // 0.004 SOL fee (scales with node_count)
```

| Node count | Channel rent (one time) | Fee per request | Security level |
|-----------|------------------------|----------------|---------------|
| 4 | ~0.005 SOL | 0.001 SOL | Minimum |
| 7 | ~0.008 SOL | 0.002 SOL | Default |
| 15 | ~0.012 SOL | 0.003 SOL | High |
| 30 | ~0.020 SOL | 0.005 SOL | Very high |
| 50 | ~0.066 SOL | 0.008 SOL | Maximum |

Fee formula: `base_fee + (node_count - MIN_NODES) * per_node_fee`

---

## Parallel Requests

One channel = one request at a time. For parallel requests:

```rust
// Developer inits 3 channels for 3 concurrent requests
init_channel(channel_index=0, max_nodes=7)  // 0.008 SOL
init_channel(channel_index=1, max_nodes=7)  // 0.008 SOL
init_channel(channel_index=2, max_nodes=7)  // 0.008 SOL

// Now 3 requests can run simultaneously
request_randomness(channel_0)  // running...
request_randomness(channel_1)  // running in parallel
request_randomness(channel_2)  // running in parallel
```

Most use cases (lottery, game, NFT) need 1 channel. High-frequency protocols init 3-5 and rotate.

---

## Account Layout

```
DiceChannel (PDA seeds: ["channel", authority, channel_index_le])
├── authority: Pubkey              (32 bytes)  — channel owner, required signer
├── channel_index: u16             (2 bytes)   — for multiple channels per developer
├── max_nodes: u8                  (1 byte)    — allocated capacity
├── status: ChannelStatus          (1 byte)    — Idle/Pending/CommitPhase/RevealPhase/Finalized
├── round_id: u64                  (8 bytes)   — increments every request, prevents replay
├── node_count: u8                 (1 byte)    — requested nodes for current round
├── commits_received: u8           (1 byte)
├── reveals_received: u8           (1 byte)
├── balance: u64                   (8 bytes)   — prepaid protocol fees
├── callback_program_id: Pubkey    (32 bytes)
├── created_slot: u64              (8 bytes)
├── commit_deadline_slot: u64      (8 bytes)
├── reveal_deadline_slot: u64      (8 bytes)
├── randomness: [u8; 32]           (32 bytes)  — result of last finalized round
├── device_ids: [[u8; 32]; N]      (N×32 bytes) — selected node IDs
├── commit_hashes: [[u8; 32]; N]   (N×32 bytes) — commit phase data
├── entropies: [[u8; 32]; N]       (N×32 bytes) — reveal phase data
├── signatures: [[u8; 64]; N]      (N×64 bytes) — ECDSA sigs for verification
└── device_pubkeys: [[u8; 33]; N]  (N×33 bytes) — compressed secp256k1 keys
```

**Size by max_nodes:**

| max_nodes | Account size | Rent (one time) |
|-----------|-------------|----------------|
| 7 | ~1,500 bytes | ~0.012 SOL |
| 15 | ~2,900 bytes | ~0.022 SOL |
| 30 | ~5,600 bytes | ~0.042 SOL |
| 50 | ~9,100 bytes | ~0.067 SOL |

All within Solana's 10 KB init limit.

---

## Security Analysis

### Risk 1: Stale Data Replay (HIGH — mitigated)

**Problem:** After a round, old commits/reveals remain in the account. An attacker could replay them.

**Mitigation:**
- `round_id` counter increments on every `request_randomness()` call
- Every `submit_commit` and `submit_reveal` must include the current `round_id` — mismatch = rejected
- All commit/reveal arrays are zeroed on `request_randomness()`

### Risk 2: Channel Front-Running (HIGH — mitigated)

**Problem:** Anyone could call `request_randomness()` on someone else's channel and drain their balance.

**Mitigation:**
- `authority` stored in channel, required as `Signer` on every `request_randomness()`
- PDA seeds include authority pubkey — can't derive another developer's channel

### Risk 3: Channel Closing During Active Round (HIGH — mitigated)

**Problem:** Closing a channel mid-round loses commit data and wastes coordinator effort.

**Mitigation:**
- `close_channel` only allowed when `status == Idle` AND `balance == 0`
- Authority must explicitly withdraw balance first (`withdraw_balance()`)
- Uses Anchor's `close` constraint (zeros data, sets closed discriminator)

### Risk 4: CPI Callback Failure (MEDIUM — mitigated)

**Problem:** If the developer's callback fails, the entire `finalize_randomness` TX reverts — randomness is never written.

**Mitigation:**
- **Split into two instructions:** `finalize_randomness()` writes the result. `deliver_callback()` does the CPI separately.
- If callback fails, randomness is still finalized. Developer can retry callback or poll.

### Risk 5: Fund Drainage (MEDIUM — mitigated)

**Problem:** Failed rounds still deduct fees. Coordinator could grief by starting rounds that timeout.

**Mitigation:**
- Fee only deducted on `request_randomness()` (developer-initiated, requires authority signer)
- `refund_failed_round()` instruction credits fee back if round status == Failed
- `checked_sub` for all balance operations

### Risk 6: Transaction Size for Batching (MEDIUM — acceptable)

**Problem:** Can the coordinator batch multiple commits into one TX?

**Analysis:**
- Single `submit_commit` instruction: ~105 bytes data
- Solana TX limit: 1,232 bytes
- Can batch **7-8 commits per TX** (all target the same channel PDA)
- Can batch **4-5 reveals per TX**
- With Address Lookup Tables: even more batching

**For 7 nodes:** 1 TX for all commits, 1 TX for all reveals = 2 TXs total (instead of 14).

### Risk 7: Concurrent Channel Confusion (MEDIUM — mitigated)

**Problem:** Coordinator submits commit to wrong channel.

**Mitigation:**
- `round_id` in every instruction prevents cross-round confusion
- Coordinator keys rounds by `(channel_pubkey, round_id)` not just request_id
- On-chain validation: `channel.round_id == instruction.round_id`

### Risk 8: Account Revival Attack (LOW — mitigated)

**Problem:** After closing, a channel could be "revived" in the same transaction.

**Mitigation:**
- Anchor's `close` sets `CLOSED_ACCOUNT_DISCRIMINATOR`
- All instructions check discriminator (Anchor does this automatically)
- Cannot `init` a closed account in the same transaction

---

## Instructions (new design)

| Instruction | Signer | Creates account? | Mutates channel? |
|-------------|--------|-----------------|-----------------|
| `init_channel(max_nodes, callback_program_id)` | authority | Yes (one time) | — |
| `fund_channel(amount)` | authority | No | balance += amount |
| `withdraw_balance(amount)` | authority | No | balance -= amount (Idle only) |
| `resize_channel(new_max_nodes)` | authority | No (realloc) | max_nodes changed (Idle only) |
| `request_randomness(node_count)` | authority | No | resets to Pending, deducts fee |
| `submit_commit(round_id, device_id, device_pubkey, commit_hash)` | coordinator | No | writes commit inline |
| `submit_reveal(round_id, device_id, device_pubkey, entropy, signature)` | coordinator | No | writes reveal inline |
| `finalize_randomness(round_id)` | coordinator | No | computes result, sets Finalized |
| `deliver_callback(round_id)` | coordinator | No | CPI to developer, sets Idle |
| `close_channel()` | authority | No (closes) | returns rent to authority |

**Removed:** CommitRecord PDA, RevealRecord PDA, EscrowAccount PDA, RandomnessResult PDA
**Kept:** DeviceRegistry PDA (one per registered hardware node, unchanged)

---

## Migration from Current Design

| Current | New | Change |
|---------|-----|--------|
| `RandomnessRequest` PDA | `DiceChannel` PDA | Merged, reusable |
| `EscrowAccount` PDA | `DiceChannel.balance` field | Merged inline |
| `CommitRecord` PDA × N | `DiceChannel.commit_hashes[]` | Inline array |
| `RevealRecord` PDA × N | `DiceChannel.entropies[]` + `.signatures[]` | Inline arrays |
| `RandomnessResult` PDA | `DiceChannel.randomness` field | Inline |
| `request_randomness(seq)` | `request_randomness(node_count)` | No sequence needed |
| `finalize_randomness` + callback | `finalize_randomness` + `deliver_callback` | Split into 2 |

---

## Developer Experience

### Before (current)

```rust
// Every request:
let ix = dice_vrf::cpi::request_randomness_ix(&accounts, sequence, &callback_id);
// Developer must track sequence numbers
// Developer pays ~0.005 SOL per request
// Each request creates 2 new PDAs
```

### After (channel)

```rust
// One-time setup:
let ix = dice_vrf::cpi::init_channel_ix(&accounts, max_nodes, &callback_id);
let ix = dice_vrf::cpi::fund_channel_ix(&accounts, amount);

// Every request (simpler, cheaper):
let ix = dice_vrf::cpi::request_randomness_ix(&channel, node_count);
// No sequence tracking needed (round_id auto-increments)
// Developer pays 0.002 SOL from prepaid balance
// No new PDAs created
```

---

## Open Questions

1. **Should `deliver_callback` be optional?** If the developer doesn't set a callback_program_id, `finalize_randomness` directly sets status to Idle (skip callback step).

2. **Should we keep the old `request_randomness` + separate PDAs as a "simple mode"?** Some developers might prefer one-shot requests without managing channels. Could support both modes.

3. **Timeout handling:** If a round times out, who calls the timeout instruction? The coordinator? Anyone? Should there be a permissionless `timeout_round()` that anyone can call after the deadline passes?

4. **Node selection authority:** Currently the coordinator picks nodes. The planned on-chain `select_nodes` instruction (using SlotHashes) would write to `channel.device_ids[]`. This is compatible with the channel design — just adds one more instruction in the flow.

5. **What happens if the developer's balance runs out mid-request?** The fee is deducted at `request_randomness()` time, before any work starts. So if balance is insufficient, the request fails immediately — no wasted coordinator effort.

---

## Recommendation

**Implement the channel design.** The security risks are all mitigable (round_id, authority checks, split finalize/callback). The cost savings are 18x. The developer experience is simpler (no sequence tracking, prepaid balance).

**Implementation order:**
1. Define `DiceChannel` account struct
2. Implement `init_channel` + `fund_channel`
3. Port `submit_commit` and `submit_reveal` to write inline
4. Port `finalize_randomness` to read inline
5. Add `deliver_callback` as separate instruction
6. Add `close_channel` + `withdraw_balance` + `resize_channel`
7. Update coordinator to use channel-based flow
8. Update SDK and TypeScript types
9. Deploy to devnet and test
10. Keep old instructions for backwards compatibility during migration
