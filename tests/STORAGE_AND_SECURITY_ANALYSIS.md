# DICE VRF — Storage Architecture & Bundled TX Security Analysis

**Date:** April 8, 2026
**Context:** After implementing bundled TX (commit+reveal+finalize in 1 TX), reducing latency from 8s to 3.5s

---

## Part 1: Where Data is Stored

### Dual Storage: PostgreSQL + Solana

Every VRF round stores data in BOTH locations. They are redundant — if PostgreSQL dies, all data is still on-chain and verifiable by anyone.

| Data | PostgreSQL (Coordinator DB) | Solana (On-Chain PDAs) |
|------|---------------------------|----------------------|
| **Nodes** | `nodes` table: node_id, latency, uptime, last_seen, is_active | `DeviceRegistry` PDA: device_pubkey, registered_at, jobs_completed |
| **Round metadata** | `rounds` table: request_id, status, selected_nodes, created_at, finalized_at | `RandomnessRequest` PDA: requester, sequence, status, selected_nodes, deadlines |
| **Commits** | `commits` table: round_id, node_id, commit_hash, submitted_at | `CommitRecord` PDA: request, device_pubkey, commit_hash, submitted_slot |
| **Reveals** | `reveals` table: round_id, node_id, entropy, submitted_at | `RevealRecord` PDA: request, device_pubkey, entropy, signature, submitted_slot |
| **Randomness output** | `rounds.randomness` column (32 bytes) | `RandomnessResult` PDA: randomness, contributing_nodes, finalized_slot |
| **Audit log** | `audit_log` table: event_type, payload, created_at | Not stored on-chain |

### PostgreSQL Schema

```sql
-- Nodes
CREATE TABLE nodes (
    node_id        BYTEA PRIMARY KEY,
    registered_at  TIMESTAMPTZ DEFAULT NOW(),
    last_seen      TIMESTAMPTZ,
    latency_ms     INTEGER,
    uptime_secs    BIGINT,
    jobs_completed BIGINT DEFAULT 0,
    is_active      BOOLEAN DEFAULT TRUE
);

-- Rounds
CREATE TABLE rounds (
    id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    request_id     BYTEA NOT NULL,
    status         TEXT NOT NULL,        -- collecting_commits, collecting_reveals, finalized, failed
    selected_nodes BYTEA[],
    randomness     BYTEA,               -- 32 bytes, NULL until finalized
    created_at     TIMESTAMPTZ DEFAULT NOW(),
    finalized_at   TIMESTAMPTZ
);

-- Commits
CREATE TABLE commits (
    round_id     UUID REFERENCES rounds(id),
    node_id      BYTEA NOT NULL,
    commit_hash  BYTEA NOT NULL,        -- SHA-256(entropy)
    submitted_at TIMESTAMPTZ DEFAULT NOW(),
    PRIMARY KEY (round_id, node_id)
);

-- Reveals
CREATE TABLE reveals (
    round_id     UUID REFERENCES rounds(id),
    node_id      BYTEA NOT NULL,
    entropy      BYTEA NOT NULL,        -- raw 32-byte entropy
    submitted_at TIMESTAMPTZ DEFAULT NOW(),
    PRIMARY KEY (round_id, node_id)
);

-- Audit log
CREATE TABLE audit_log (
    id          BIGSERIAL PRIMARY KEY,
    event_type  TEXT NOT NULL,
    payload     JSONB,
    created_at  TIMESTAMPTZ DEFAULT NOW()
);
```

### Solana PDA Accounts

```
DeviceRegistry PDA (58 bytes)
  seeds: ["device", SHA-256(device_pubkey)]
  Fields: device_pubkey [u8; 33], registered_at, jobs_completed, is_active

RandomnessRequest PDA (339 bytes)
  seeds: ["request", requester, sequence_le]
  Fields: requester, sequence, status, selected_nodes, callback_program_id, deadlines

CommitRecord PDA (113 bytes)
  seeds: ["commit", requester, sequence_le, device_id]
  Fields: request, device_pubkey, commit_hash, submitted_slot

RevealRecord PDA (185 bytes)
  seeds: ["reveal", requester, sequence_le, device_id]
  Fields: request, device_pubkey, entropy, signature, submitted_slot

RandomnessResult PDA (312 bytes)
  seeds: ["result", requester, sequence_le]
  Fields: request, randomness [u8; 32], contributing_nodes, contributing_count, finalized_slot

EscrowAccount PDA (57 bytes)
  seeds: ["escrow", requester, sequence_le]
  Fields: requester, sequence, amount, is_claimed
```

### Purpose of Each Storage Layer

| Layer | Purpose | Who reads it | Mutable? |
|-------|---------|-------------|----------|
| **PostgreSQL** | Fast queries, monitoring, dashboard, operational analytics | Coordinator, dashboard, monitoring | Yes (coordinator writes) |
| **Solana** | Source of truth, public verification, payment escrow | Anyone (permissionless read), smart contracts | Only via program instructions |

**Key point:** PostgreSQL is a convenience layer. If the coordinator's database is wiped, all VRF results are still verifiable on Solana. The on-chain data is the canonical record.

---

## Part 2: Bundled TX Security Analysis

### What Changed

**Before (3 separate TXs, ~8 seconds):**
```
TX 1: submit_commit(commit_hash)         → CommitRecord PDA on-chain
        [~1.5 second gap]
TX 2: submit_reveal(entropy, signature)  → RevealRecord PDA on-chain
        [~1.5 second gap]
TX 3: finalize_randomness()              → RandomnessResult PDA on-chain
```

**After (1 bundled TX, ~3.5 seconds):**
```
TX 1: [submit_commit + submit_reveal + finalize_randomness]
      → All 3 PDAs created atomically in one Solana transaction
```

### Attack Vector Analysis

#### Attack 1: Coordinator forges the commit hash

**Scenario:** Coordinator submits a different commit_hash than what the device sent.

**Defense:** The commit_hash in the bundled TX is `SHA-256(entropy)`. The `submit_reveal` instruction (in the SAME TX) verifies:
```rust
// On-chain verification in submit_reveal.rs:
require!(SHA-256(entropy) == commit_record.commit_hash, DiceError::RevealMismatch);
```
If the coordinator fakes the commit_hash, the reveal verification fails and the **entire bundled TX reverts**.

**Status: DEFENDED** — same as before bundling.

#### Attack 2: Coordinator substitutes different entropy

**Scenario:** Coordinator receives real entropy from device but submits different entropy.

**Defense:** The device signs its entropy with ECDSA secp256k1. The `submit_reveal` instruction verifies:
```rust
// On-chain verification in submit_reveal.rs:
// Recovers the public key from the signature and verifies it matches device_pubkey
```
If the coordinator changes the entropy, the ECDSA signature won't match → TX reverts.

**Status: DEFENDED** — same as before bundling.

#### Attack 3: Coordinator sees result before committing (front-running)

**Scenario:** With separate TXs, the commit goes on-chain BEFORE the reveal, creating a time-lock. With bundling, both hit simultaneously. Could the coordinator see the entropy, decide the result is unfavorable, and not submit?

**Analysis:**
- **Before bundling:** The coordinator ALREADY knew the entropy before submitting `finalize_randomness` (the 3rd TX). It could have withheld TX 3.
- **After bundling:** The coordinator knows the entropy before submitting the bundled TX. It could withhold the entire bundle.
- **Conclusion:** This is the SAME risk. Bundling does NOT introduce a new attack vector.

**Mitigation:**
- Protocol timeout (60s): if coordinator doesn't submit, the round fails and user's escrow is refundable
- Multi-node rounds (4-7 nodes): coordinator can't predict combined randomness until ALL nodes reveal
- The coordinator is a known, auditable entity — not anonymous

**Status: SAME RISK AS BEFORE** — not worsened by bundling.

#### Attack 4: Coordinator replays old entropy

**Scenario:** Coordinator reuses entropy from a previous round.

**Defense:** Each round has a unique `request_id` (PDA address). The CommitRecord and RevealRecord PDAs include the request_id in their seeds. Replaying old data for a new round would derive different PDAs → the commit_hash verification fails because the old CommitRecord PDA doesn't exist for the new round.

**Status: DEFENDED** — same as before bundling.

#### Attack 5: Atomic failure manipulation

**Scenario:** With bundling, if `finalize_randomness` fails, the commit and reveal also revert (atomic). Could an attacker exploit this?

**Analysis:** If the bundled TX fails:
- No commit, reveal, or result is recorded on-chain
- The round times out (60s) and fails
- User's escrow remains locked (refundable)
- The coordinator can retry or the round is abandoned

This is actually BETTER than the old model where `submit_commit` could succeed but `finalize_randomness` could fail — leaving orphaned commit records on-chain with no result.

**Status: IMPROVED** — atomic all-or-nothing is cleaner.

### What We Lose

| Property | Before (3 TXs) | After (Bundled) | Impact |
|----------|----------------|-----------------|--------|
| On-chain time gap between commit and reveal | ~1.5 seconds | 0 (atomic) | LOW — the time gap was never a security property; coordinator knew both values |
| Separate timestamps per step | 3 distinct on-chain timestamps | 1 timestamp for all 3 | LOW — operational, not security |
| Partial success | Commit can succeed without finalize | All-or-nothing | IMPROVED — no orphaned records |
| Proof of temporal ordering | Commit provably before reveal | Both in same slot | LOW — coordinator sees both before submitting either way |

### What We Keep

| Property | Status | On-chain enforcement |
|----------|--------|---------------------|
| SHA-256 hash binding | **KEPT** | `submit_reveal` checks `SHA-256(entropy) == commit_hash` |
| ECDSA device signature | **KEPT** | `submit_reveal` verifies secp256k1 signature |
| Multi-node entropy mixing | **KEPT** | `finalize_randomness` combines all entropies via SHA-256 |
| On-chain audit trail | **KEPT** | CommitRecord + RevealRecord + RandomnessResult PDAs all created |
| Public verifiability | **KEPT** | Anyone can read the PDAs and verify the math |
| Escrow payment | **KEPT** | 0.002 SOL locked until round completes |

### Comparison with Other Oracles

| Oracle | Commits on-chain separately? | Bundled fulfillment? |
|--------|------------------------------|---------------------|
| **Chainlink VRF v2** | No — single fulfillment TX | Yes — oracle submits result in 1 TX |
| **Switchboard VRF** | No — SGX produces result in 1 step | Yes — result posted in 1 TX |
| **DICE VRF (old)** | Yes — 3 separate TXs | No |
| **DICE VRF (new)** | No — bundled | Yes — 1 TX like Chainlink/Switchboard |

**Bundling makes DICE match the industry standard.** Chainlink and Switchboard never put commits on-chain separately — they always submit the final result in a single TX after off-chain computation.

---

## Verdict

**Bundling is safe and matches industry practice.**

The smart contract performs ALL cryptographic verifications (SHA-256 hash binding, ECDSA signature verification, multi-node entropy combination) regardless of whether the instructions arrive in 3 TXs or 1. The only change is latency improvement (8s → 3.5s) and cost reduction (3 TX fees → 1).

No new attack vectors are introduced. The theoretical coordinator-withholding risk exists equally in both models and is mitigated by protocol timeouts + multi-node BFT.
