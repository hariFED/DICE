# DICE On-Chain VRF Test Results

**Date:** April 8, 2026
**Network:** Solana Devnet
**Mode:** Full Production (mTLS + PostgreSQL + Solana devnet, NO simulation)
**Device:** ESP32-S3-N16R8 (MAC: 1c:db:d4:46:c8:b4)
**Program:** `78Qv6cyKkRZN2YngiLSSBCe2iyRc6jgtCs3incCaMRcv`
**Coordinator:** `3df8FZoosdv3mrYwWS82TEqQps97qAdmnnijUNhz6tp9`

---

## Summary

| Metric | Value |
|--------|-------|
| On-chain VRF rounds completed | **18** |
| Bundled TXs sent (commit+reveal+finalize) | **20** |
| Bundled TX failures | **0** |
| Average round latency | **3.9 seconds** |
| On-chain TXs per round | **2** (request + bundled) |
| Device uptime | 1,354 seconds continuous |

---

## How Each On-Chain TX Works

### Transaction 1: `request_randomness` (User/Coordinator → Solana)

Creates the on-chain request and escrow:

```
Instruction: request_randomness
Accounts:
  [0] Requester (signer, writable) — pays 0.002 SOL
  [1] RandomnessRequest PDA (writable, init)
      seeds = ["request", requester, sequence_le_bytes]
  [2] EscrowAccount PDA (writable, init)
      seeds = ["escrow", requester, sequence_le_bytes]
  [3] SystemProgram

Data: [discriminator (8)] [sequence (u64 LE)] [callback_program_id (32)]

Result: RandomnessRequest PDA created (339 bytes)
        EscrowAccount PDA created (57 bytes, holds 0.002 SOL)
```

### Transaction 2: Bundled `submit_commit + submit_reveal + finalize_randomness`

All 3 instructions in ONE transaction after ESP32 VRF completes:

```
Instruction 1: submit_commit
  Data: [disc (8)] [device_id (32)] [device_pubkey (33)] [commit_hash (32)]
  Creates: CommitRecord PDA

Instruction 2: submit_reveal
  Data: [disc (8)] [device_id (32)] [device_pubkey (33)] [entropy (32)] [signature (64)]
  Creates: RevealRecord PDA
  Verifies: SHA-256(entropy) == commit_hash (on-chain!)

Instruction 3: finalize_randomness
  Combines: randomness = SHA-256(entropy_1 || ... || entropy_n)
  Creates: RandomnessResult PDA (32-byte randomness output)
  Invokes: CPI callback to developer's program (if configured)
```

---

## dApp Test Results (Real On-Chain Transactions)

### 1. Dice Roll Game

**Program:** `CLpaMPxyu5Up4fuZb1JiY2uzj4s4iYVg9RfQHNFRuzAj`

| Step | Detail |
|------|--------|
| request_randomness TX | `o4HmoszUckJCCyTWFvUFmYzBYidFMwm7cA1MWkvgfwQekQ1qQ9rdihBbZ3WFTJaqPB5qnpcnMLBdTQ92zNLb4rY` |
| Bundled TX | `3tWoZ2ZyScMmfqLkDt7YMkLZbs2W4AMeY7GCVkiYhBVsAv1p2UBuwdRyFZeAx4WBNBy3k6mgJNRhdAeM3Tr9Pqhj` |
| Round latency | 4,006ms |
| Randomness | `e4fa85a00730fc21e6b105940f3a752bb009a48463357672ffd2b43ee6046767` |

**Dice roll computation:**
```
randomness[0..4] as u32 (LE) = 2,693,135,076
result = (2693135076 % 6) + 1 = 1

The dice shows: 1
```

**Verification:** Anyone can verify on Solana Explorer:
- [request_randomness TX](https://explorer.solana.com/tx/o4HmoszUckJCCyTWFvUFmYzBYidFMwm7cA1MWkvgfwQekQ1qQ9rdihBbZ3WFTJaqPB5qnpcnMLBdTQ92zNLb4rY?cluster=devnet)
- [Bundled TX](https://explorer.solana.com/tx/3tWoZ2ZyScMmfqLkDt7YMkLZbs2W4AMeY7GCVkiYhBVsAv1p2UBuwdRyFZeAx4WBNBy3k6mgJNRhdAeM3Tr9Pqhj?cluster=devnet)

---

### 2. Lucky Wheel

**Program:** `FzUuegZpKT4BHhzms1eJX7L2f6r3NTxMRexs8uqxtnbf`

| Step | Detail |
|------|--------|
| request_randomness TX | `419zMSiaRJEL6ujHrMrs6TGcH1HyukTG6veTGUtRCBfXPMv9isLTpkFDPHLw7RdwLUj9Bbk8YRtATb3s36MBh9FV` |
| Bundled TX | `35cpneE7rQ8RBmQpDe2by3u6gEhUjCeC7E4vW9EYXadCxYxZ4UndiiEfJayXZKyRjHWLxq3aHczEp6p4kWExx916` |
| Round latency | 3,486ms |
| Randomness | `6f96bbea76aca3252266c8515eb36e8dd837cc7c46608562a0d4a2e5a3a08264` |

**Wheel segment selection:**
```
Wheel config:
  Segment 0: 2x   (weight 40, range 0-39)
  Segment 1: 5x   (weight 25, range 40-64)
  Segment 2: 10x  (weight 10, range 65-74)
  Segment 3: LOSE (weight 20, range 75-94)
  Segment 4: 50x  (weight 5,  range 95-99)

randomness[0..4] as u32 (LE) = 3,938,162,287
point = 3938162287 % 100 = 87

87 is in range [75-94] → Segment 3: LOSE (0x payout)
```

**Verification:**
- [request_randomness TX](https://explorer.solana.com/tx/419zMSiaRJEL6ujHrMrs6TGcH1HyukTG6veTGUtRCBfXPMv9isLTpkFDPHLw7RdwLUj9Bbk8YRtATb3s36MBh9FV?cluster=devnet)
- [Bundled TX](https://explorer.solana.com/tx/35cpneE7rQ8RBmQpDe2by3u6gEhUjCeC7E4vW9EYXadCxYxZ4UndiiEfJayXZKyRjHWLxq3aHczEp6p4kWExx916?cluster=devnet)

---

### 3. Prediction Market

**Program:** `EHf5YLG2p7Wca9nUqJXRB6yATZidrBzJKM4Qj4k1EUvc`

| Step | Detail |
|------|--------|
| request_randomness TX | `aSh2KNLjMq7ThfoVKmFaZohG4mrqJzGPKWSUfNkyGJY85wnPXW4SRJtHbxMZZfT6KfX4SpNohDw1KQSK1PqdU95` |
| Bundled TX | `yXDyKmjzCg4vWHRBQq8TZ421qswUBDkej8HJB4Sky2eW8j56Ft2jyNHDufaN4BEaCYJvHd7FyQ9MEo5ufRnupzN` |
| Round latency | 4,513ms |
| Randomness | `c32f7a92d44e9843e580b27c207a983fb1eff4cba12415d48feb8a7ad65b8d1e` |

**Market resolution (binary mode):**
```
Question: "Will the random output be even?"
YES pool: 10 SOL, NO pool: 5 SOL

randomness[0] = 0xc3 = 195
outcome = 195 & 1 = 1 → NO wins!

Payout: NO bettors split 15 SOL proportionally
```

**Verification:**
- [request_randomness TX](https://explorer.solana.com/tx/aSh2KNLjMq7ThfoVKmFaZohG4mrqJzGPKWSUfNkyGJY85wnPXW4SRJtHbxMZZfT6KfX4SpNohDw1KQSK1PqdU95?cluster=devnet)
- [Bundled TX](https://explorer.solana.com/tx/yXDyKmjzCg4vWHRBQq8TZ421qswUBDkej8HJB4Sky2eW8j56Ft2jyNHDufaN4BEaCYJvHd7FyQ9MEo5ufRnupzN?cluster=devnet)

---

## On-Chain Stress Test Results

### Sequential 10 rounds (bundled TX)

| Round | Dispatch (ms) | Total (ms) | Status |
|-------|--------------|------------|--------|
| 1 | 1,369 | 3,994 | Finalized |
| 2 | 1,045 | 3,322 | Finalized |
| 3 | 1,378 | 4,353 | Finalized |
| 4 | 1,461 | 3,668 | Finalized |
| 5 | 1,378 | 4,248 | Finalized |
| 6 | 1,101 | 3,299 | Finalized |
| 7 | 1,107 | 4,961 | Finalized |
| 8 | 1,101 | 4,371 | Finalized |
| 9 | 1,059 | 3,538 | Finalized |
| 10 | 1,104 | 3,771 | Finalized |

**Average: 3,953ms | Median: 3,882ms | Min: 3,299ms | Max: 4,961ms**

### Latency breakdown

```
request_randomness TX → Solana:     ~1.2s (dispatch)
ESP32 VRF (entropy + commit-reveal): ~1.5s (hardware)
Bundled TX → Solana:                  ~1.2s (single TX)
Total:                                ~3.9s
```

### Cost per round

```
User pays:
  request_randomness TX fee:   0.000005 SOL
  Protocol fee (escrow):       0.002000 SOL
  Total user cost:             0.002005 SOL

Coordinator pays:
  Bundled TX fee:              0.000005 SOL
  CommitRecord PDA rent:       ~0.001 SOL (reclaimable)
  RevealRecord PDA rent:       ~0.001 SOL (reclaimable)
  RandomnessResult PDA rent:   ~0.002 SOL (reclaimable)
  Total coordinator cost:      ~0.004 SOL (mostly reclaimable)
```

---

## Complete On-Chain Flow (Verified)

```
User                          Solana                    Coordinator              ESP32-S3
  |                             |                           |                       |
  |-- request_randomness TX --->|                           |                       |
  |   (0.002 SOL + TX fee)      |                           |                       |
  |                             |-- event detected -------->|                       |
  |                             |                           |                       |
  |                             |                           |-- JobAssignment ----->|
  |                             |                           |   (via wss:// mTLS)   |
  |                             |                           |                       |
  |                             |                           |   [Hardware TRNG      |
  |                             |                           |    generates entropy] |
  |                             |                           |                       |
  |                             |                           |<-- CommitSubmission --|
  |                             |                           |   (SHA-256 + ECDSA)   |
  |                             |                           |                       |
  |                             |                           |-- reveal signal ----->|
  |                             |                           |                       |
  |                             |                           |<-- RevealSubmission --|
  |                             |                           |   (entropy + ECDSA)   |
  |                             |                           |                       |
  |                             |<-- BUNDLED TX ------------|                       |
  |                             |   [submit_commit          |                       |
  |                             |    submit_reveal          |                       |
  |                             |    finalize_randomness]   |                       |
  |                             |                           |                       |
  |<-- RandomnessResult PDA ---|                           |                       |
  |   (32 bytes of randomness) |                           |                       |
  |                             |                           |                       |
  |   Total: ~3.9 seconds      |                           |                       |
```

---

## Solana Explorer Verification

All transactions are verifiable on Solana devnet:

**Device Registration:**
- TX: [`4i9EhU64t7pwLssufA95VoUMBzuo4171Gysk29FUYiAkqukSUZnjQ9Zck3DJAegtnof3DcYtoNhjH25LTjaGL3Wj`](https://explorer.solana.com/tx/4i9EhU64t7pwLssufA95VoUMBzuo4171Gysk29FUYiAkqukSUZnjQ9Zck3DJAegtnof3DcYtoNhjH25LTjaGL3Wj?cluster=devnet)

**First Ever On-Chain VRF Round:**
- request: [`eqhuzUgU8iF9M8thj6Hu5nyVyZJK6bRwcb8NNbbRwHLayqs3dDV9Yymxvwxtnv9jUyGTK8JqsqRxci8q5nvBb6Q`](https://explorer.solana.com/tx/eqhuzUgU8iF9M8thj6Hu5nyVyZJK6bRwcb8NNbbRwHLayqs3dDV9Yymxvwxtnv9jUyGTK8JqsqRxci8q5nvBb6Q?cluster=devnet)
- bundled: [`VdB5UitUfMKMqzDncwiDMzPFVrNbZeywAZMN2yhzKubhL7cCrhnf2JWfFMJ6g7qAJYfSfJFbgiaX1Evvdi37q1Q`](https://explorer.solana.com/tx/VdB5UitUfMKMqzDncwiDMzPFVrNbZeywAZMN2yhzKubhL7cCrhnf2JWfFMJ6g7qAJYfSfJFbgiaX1Evvdi37q1Q?cluster=devnet)

**Total on-chain VRF rounds: 18 | Bundled TX failures: 0**
