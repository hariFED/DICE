# DICE v2.0 — Channel Design

> **Branch:** `v2.0-channel-design`
> **Date:** 2026-03-30
> **Status:** Implementation complete, pending devnet deployment

---

## TL;DR

v2.0 replaces the per-round PDA model with a single reusable **DiceChannel** account. This makes randomness requests **18x cheaper**, decouples callback delivery from finalization so randomness is never lost, and adds **on-chain verifiable node selection** so even a compromised coordinator cannot manipulate which devices participate.

---

## Why v2.0 Exists

### The v1.0 Problem

Every randomness request in v1.0 creates **17 new on-chain accounts**:

```
1  RandomnessRequest PDA
1  EscrowAccount PDA
7  CommitRecord PDAs      (one per selected node)
7  RevealRecord PDAs      (one per selected node)
1  RandomnessResult PDA
──────────────────────
17 accounts per request
```

Each account costs ~0.002 SOL in rent. These accounts are **single-use** — created for one round, never touched again. This creates three problems:

1. **Expensive:** ~0.036 SOL per request (rent + fees). A game making 1,000 requests/day burns 36 SOL/day.
2. **Fragile callbacks:** `finalize_randomness` computes the result AND calls the developer's callback in one transaction. If the callback reverts (bug, out of compute, etc.), the **entire finalization reverts** — randomness is lost, the round fails.
3. **Centralized node selection:** The coordinator picks which nodes participate. A compromised coordinator could select colluding nodes to manipulate the output.

### What v2.0 Fixes

| Problem | v1.0 | v2.0 |
|---------|------|------|
| Cost per request | ~0.036 SOL (17 PDAs) | ~0.002 SOL (fee only, no new accounts) |
| Callback failure | Reverts finalization, randomness lost | Randomness persists, callback retried separately |
| Node selection | Coordinator picks (centralized) | On-chain via SlotHashes (trustless) |
| Accounts per request | 17 new PDAs | 0 new PDAs (reuse channel) |
| Developer complexity | Track sequence numbers, manage escrows | Fund channel once, call `request_randomness_v2` |

---

## Architecture

### The DiceChannel Account

A `DiceChannel` is a persistent PDA that holds **everything** for one randomness round at a time:

```
Seeds: ["channel", authority, channel_index (u16 LE)]
```

**Fixed fields (143 bytes):**
- `authority` — channel owner (signer on request/fund/withdraw/close)
- `channel_index` — supports multiple channels per developer (0, 1, 2, ...)
- `max_nodes` — capacity set at init (4-50), changeable via `resize_channel`
- `status` — lifecycle enum: Idle / Pending / CommitPhase / RevealPhase / Finalized / Failed
- `round_id` — auto-incrementing counter (prevents commit/reveal replay)
- `node_count`, `commits_received`, `reveals_received` — round progress
- `created_slot`, `commit_deadline_slot`, `reveal_deadline_slot` — timing
- `balance` — prepaid protocol fees (lamports)
- `callback_program_id` — CPI target (Pubkey::default() = no callback)
- `randomness` — 32-byte result from last finalized round

**Inline arrays (variable, based on max_nodes):**
- `device_ids: Vec<[u8; 32]>` — SHA-256 of device pubkeys
- `device_pubkeys: Vec<[u8; 33]>` — compressed secp256k1 keys
- `commit_hashes: Vec<[u8; 32]>` — SHA-256(entropy) per node
- `entropies: Vec<[u8; 32]>` — revealed entropy values
- `signatures: Vec<[u8; 64]>` — ECDSA signatures over entropy

**Account size:** `143 + 20 + (max_nodes * 193)` bytes

For a 7-node channel: 143 + 20 + 1351 = **1,514 bytes** (~0.011 SOL rent, paid once).

### Round Lifecycle

```
                 init_channel (once)
                       |
                 fund_channel (prepay SOL)
                       |
        +--- request_randomness_v2 (deducts 0.002 SOL fee)
        |              |
        |       select_nodes (on-chain, uses SlotHashes)
        |              |
        |       submit_commit_v2 x N  (inline in channel)
        |              |
        |       submit_reveal_v2 x N  (inline in channel)
        |              |
        |        finalize_v2 (SHA-256 combination -> randomness)
        |              |
        |       deliver_callback (CPI to developer, separate TX)
        |              |
        |        channel.status = Idle
        |              |
        +--- (loop) next request_randomness_v2
```

---

## Instruction Reference

### Channel Management

| Instruction | Signer | Description |
|-------------|--------|-------------|
| `init_channel(channel_index, max_nodes, callback_program_id)` | authority | Create channel PDA. Pays rent once. max_nodes: 4-50. |
| `fund_channel(amount)` | authority | Add SOL to channel's prepaid balance. |
| `withdraw_balance(amount)` | authority | Pull funds out (Idle state only). |
| `resize_channel(new_max_nodes)` | authority | Change capacity without closing (Idle only). Anchor realloc handles rent delta. |
| `close_channel()` | authority | Close channel, reclaim all rent. Requires Idle + zero balance. |

### Round Execution

| Instruction | Signer | Description |
|-------------|--------|-------------|
| `request_randomness_v2(node_count)` | authority | Start a new round. Deducts 0.002 SOL. Resets channel. Sets commit deadline. |
| `select_nodes(round_id)` | coordinator | On-chain node selection via SlotHashes. DeviceRegistry PDAs as remaining_accounts. |
| `submit_commit_v2(round_id, device_id, device_pubkey, commit_hash)` | coordinator | Store commit inline. Validates device_id = SHA-256(device_pubkey). Duplicate/replay protection. |
| `submit_reveal_v2(round_id, device_id, device_pubkey, entropy, signature)` | coordinator | Verify SHA-256(entropy) == commit_hash. Store entropy + sig inline. |
| `finalize_v2(round_id)` | coordinator | Combine entropies via SHA-256. Requires >= 4 reveals. Writes randomness to channel. |
| `deliver_callback(round_id)` | coordinator | CPI to callback_program_id with `[discriminator, channel_key, randomness]`. Transitions to Idle. Failure doesn't affect stored randomness. |

### Account Arguments

**Channel PDA derivation:**
```
seeds = ["channel", authority.key(), &channel_index.to_le_bytes()]
```

**All v2.0 instructions use only 2-3 accounts** (coordinator/authority + channel + optional system_program). Compare to v1.0 which needed 4-8 accounts per instruction.

---

## On-Chain Node Selection (`select_nodes`)

This is the key trustlessness upgrade in v2.0.

### Problem
In v1.0, the coordinator's `SelectionEngine` picks nodes off-chain. If the coordinator is compromised, an attacker could select colluding nodes to control the randomness output.

### Solution
`select_nodes` runs **entirely on-chain** using the `SlotHashes` sysvar:

```rust
// 1. Read most recent slot hash (unpredictable, 512 entries)
let recent_slot_hash = &slot_hashes_data[16..48];

// 2. Compute deterministic seed
let seed = SHA-256(
    slot_hash ||       // unpredictable on-chain entropy
    channel_key ||     // unique to this channel
    round_id ||        // unique to this round
    block_height       // current slot
);

// 3. Fisher-Yates shuffle over active DeviceRegistry accounts
for i in (1..candidates.len()).rev() {
    let j = SHA-256(seed || i) % (i + 1);
    swap(indices[i], indices[j]);
}

// 4. Select first node_count from shuffled list
```

### Why This Works
- **SlotHashes** contains hashes of recent slots — generated by the validator leader, not predictable before the slot is produced
- The seed combines slot hash + channel-specific data + current block height — unique per round
- Fisher-Yates shuffle is uniform and deterministic given the seed
- All DeviceRegistry PDAs are passed as remaining_accounts — anyone can verify the selection was fair
- Even the coordinator cannot predict which nodes will be selected before calling the instruction

---

## Cost Breakdown

### v1.0 Cost Per Request
```
RandomnessRequest PDA rent:    ~0.002 SOL
EscrowAccount PDA rent:        ~0.002 SOL
7x CommitRecord PDA rent:      ~0.014 SOL
7x RevealRecord PDA rent:      ~0.014 SOL
RandomnessResult PDA rent:     ~0.002 SOL
Protocol fee:                   0.002 SOL
                               ──────────
Total:                         ~0.036 SOL
```

### v2.0 Cost Per Request
```
Channel creation (one-time):    0.011 SOL (7-node channel)
Per-request fee:                0.002 SOL
New accounts created:           0
                               ──────────
Total per request:              0.002 SOL
```

### At Scale
| Requests/day | v1.0 daily cost | v2.0 daily cost | Savings |
|-------------|----------------|----------------|---------|
| 100 | 3.6 SOL | 0.2 SOL | 94% |
| 1,000 | 36 SOL | 2 SOL | 94% |
| 10,000 | 360 SOL | 20 SOL | 94% |

---

## Migration Guide (v1.0 to v2.0)

### For Developers Using the SDK

**Before (v1.0):**
```rust
// Every request: track sequence, create escrow, create request
let accounts = DiceVrfAccounts::resolve(&requester, sequence, &program_id);
let ix = dice_vrf::cpi::request_randomness_ix(&accounts, sequence, &callback_pid);
```

**After (v2.0):**
```rust
// One-time setup
let ix = dice_vrf::cpi::init_channel_ix(&program_id, &authority, 0, 7, &callback_pid);
let ix = dice_vrf::cpi::fund_channel_ix(&program_id, &authority, 0, 10_000_000);

// Per request (no sequence tracking needed)
let ix = dice_vrf::cpi::request_randomness_v2_ix(&program_id, &authority, 0, 5);

// Read result
let randomness = dice_vrf::cpi::decode_channel_randomness(&account_data);
```

### For TypeScript Integrations

```typescript
// One-time
await program.methods.initChannel(0, 7, NO_CALLBACK).accounts({...}).rpc();
await program.methods.fundChannel(new BN(10_000_000)).accounts({...}).rpc();

// Per request
await program.methods.requestRandomnessV2(5).accounts({...}).rpc();

// Read result
const ch = await program.account.diceChannel.fetch(channelPda);
if ("idle" in ch.status || "finalized" in ch.status) {
    const randomness = ch.randomness; // [u8; 32]
}
```

### Backwards Compatibility

All v1.0 instructions remain in the program. Existing integrations continue to work unchanged. v2.0 is additive — developers can migrate at their own pace.

---

## Files Changed (v2.0 branch)

### Smart Contract (`programs/dice/`)
| File | Change |
|------|--------|
| `src/state/dice_channel.rs` | **New.** DiceChannel account, ChannelStatus enum, space calculation, reset logic |
| `src/state/mod.rs` | Added dice_channel export |
| `src/instructions/init_channel.rs` | **New.** Channel creation with max_nodes validation |
| `src/instructions/fund_channel.rs` | **New.** CPI transfer + balance tracking |
| `src/instructions/request_randomness_v2.rs` | **New.** Fee deduction, round reset, deadline setting |
| `src/instructions/submit_commit_v2.rs` | **New.** Inline commit storage, duplicate/replay protection |
| `src/instructions/submit_reveal_v2.rs` | **New.** Hash verification, phase transition, inline storage |
| `src/instructions/finalize_v2.rs` | **New.** SHA-256 entropy combination |
| `src/instructions/deliver_callback.rs` | **New.** Decoupled CPI callback delivery |
| `src/instructions/withdraw_balance.rs` | **New.** Checked balance withdrawal |
| `src/instructions/close_channel.rs` | **New.** Channel closure with safety checks |
| `src/instructions/resize_channel.rs` | **New.** Anchor realloc for capacity changes |
| `src/instructions/select_nodes.rs` | **New.** On-chain node selection via SlotHashes |
| `src/instructions/mod.rs` | Added all v2.0 module declarations and exports |
| `src/lib.rs` | Added all 12 v2.0 instruction endpoints |
| `src/constants.rs` | Added SEED_CHANNEL |
| `src/error.rs` | Fixed InvalidNodeCount message, added InvalidDeviceId |

### Coordinator (`coordinator/`)
| File | Change |
|------|--------|
| `src/solana_tx.rs` | Added `channel_pda()` + 7 v2.0 instruction builders + 5 discriminants |

### SDK (`sdk/dice-vrf/`)
| File | Change |
|------|--------|
| `src/pda.rs` | Added SEED_CHANNEL, `channel_pda()` |
| `src/cpi.rs` | Added `init_channel_ix`, `fund_channel_ix`, `request_randomness_v2_ix`, `decode_channel_randomness` |
| `src/types.rs` | Added `ChannelStatus` enum, `DiceChannelInfo` struct |
| `src/lib.rs` | Added v2.0 type re-exports |

### IDL & Types
| File | Change |
|------|--------|
| `target/idl/dice.json` | Added 12 v2.0 instructions, DiceChannel account, ChannelStatus type, InvalidDeviceId error |
| `target/types/dice.ts` | Regenerated from updated IDL |

### Tests
| File | Change |
|------|--------|
| `tests/dice_v2.ts` | **New.** 12 integration tests covering full channel lifecycle |

### Documentation
| File | Change |
|------|--------|
| `docs/V2_CHANGELOG.md` | **New.** This document |
| `docs/PROGRESS.md` | Updated with v2.0 implementation status table |
| `docs/TODO.md` | Marked select_nodes items as complete |

---

## Build Health

```
cargo check --workspace    ->  0 errors
cargo test  --workspace    ->  13 pass, 0 fail
TypeScript tests (v2.0)    ->  12 tests (needs solana-test-validator)
```

---

## What's Next (v3.0 Roadmap)

- [ ] Wire coordinator to use v2.0 channel flow (replace v1.0 round dispatch)
- [ ] Backup node selection on round timeout
- [ ] Node blacklist for non-revealers
- [ ] TypeScript SDK (`@dice-network/sdk` npm package)
- [ ] Lottery example program (end-to-end)
- [ ] Coin-flip example program
- [ ] Devnet deployment of v2.0 contract
- [ ] Trident fuzz testing
