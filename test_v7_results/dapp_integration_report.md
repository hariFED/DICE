# dApp integration test — DICE v7.5 randomness through three production programs

**Date:** 2026-04-17
**Source rounds:** 48 successful rounds from `v75_audit_clean_50.json` (on-chain Fisher-Yates selection, real ESP32-S3 hardware entropy).
**Methodology:** each round's 32-byte randomness is passed through the **verbatim `dice_callback` handler logic** of each deployed program (code copied from `programs/<dapp>/src/lib.rs`). This proves the randomness drives each game's outcome correctly — what would happen on-chain if `deliver_callback` CPI fired.

Not covered in this report: the on-chain CPI itself (`dice::deliver_callback` → `<dapp>::dice_callback`). That needs per-dApp driver binaries (similar shape to `coin_toss_driver`). Captured as a follow-up.

---

## 1 · dice-roll · `CLpaMPxyu5Up4fuZb1JiY2uzj4s4iYVg9RfQHNFRuzAj`

**Formula from `programs/dice-roll/src/lib.rs:136-137`:**
```rust
let rand_u32 = u32::from_le_bytes([rand[0..4]]);
let result = (rand_u32 % 6) as u8 + 1;
```

**Distribution across 48 rounds:**

| Face | Count | Observed | Expected |
|-----:|------:|---------:|---------:|
| 1 | 9 | 19 % | 16.7 % |
| 2 | 11 | 23 % | 16.7 % |
| 3 | 9 | 19 % | 16.7 % |
| 4 | 6 | 12 % | 16.7 % |
| 5 | 7 | 15 % | 16.7 % |
| 6 | 6 | 12 % | 16.7 % |

χ² = 2.50 · five degrees of freedom · **passes uniform hypothesis at p < 0.05** (critical value 11.07). All six faces observed, no gaps.

Modular bias from u32 mod 6 is `4 / 2³²` ≈ 9.3 × 10⁻¹⁰ — negligible.

---

## 2 · lucky-wheel · `FzUuegZpKT4BHhzms1eJX7L2f6r3NTxMRexs8uqxtnbf`

**Formula from `programs/lucky-wheel/src/lib.rs:169-184`:**
```rust
let rand_u32 = u32::from_le_bytes([rand[0..4]]);
let point = rand_u32 % total_weight;
// walk segments: winner is first where accumulated > point
```

**Wheel configuration** (from `VRF_TEST_RESULTS.md`): weights 40 / 25 / 10 / 20 / 5 = total 100.

| Segment | Multiplier | Count | Observed | Expected |
|---------|-----------:|------:|---------:|---------:|
| 2x      | 2× | 16 | 33 % | 40 % |
| 5x      | 5× | 12 | 25 % | 25 % |
| 10x     | 10× | 7 | 15 % | 10 % |
| LOSE    | 0× | 10 | 21 % | 20 % |
| JACKPOT | 50× | 3 | 6 % | 5 % |

All five segments hit (including the rare JACKPOT). Observed proportions are within sampling-noise of expected at n=48.

---

## 3 · prediction-market · `EHf5YLG2p7Wca9nUqJXRB6yATZidrBzJKM4Qj4k1EUvc`

**Two resolution modes** (from `programs/prediction-market/src/lib.rs:222-233`):

### 3.1 Binary (even/odd)
```rust
let outcome = randomness[0] & 1;
// outcome == 0 → YES wins, outcome == 1 → NO wins
```

| Outcome | Count | Observed |
|---------|------:|---------:|
| YES | 25 | 52 % |
| NO  | 23 | 48 % |

Balanced: 52 / 48 — indistinguishable from a fair coin at n=48.

### 3.2 Threshold (u64 > MAX/2)
```rust
let value = u64::from_le_bytes(randomness[0..8]);
let outcome = if value > u64::MAX / 2 { 0 } else { 1 };
```

| Outcome | Count | Observed |
|---------|------:|---------:|
| YES | 20 | 42 % |
| NO  | 28 | 58 % |

At n=48 the margin of error on a 50/50 fair coin is ±14 % (1-sigma ≈ 7 %). 42 / 58 sits within that — not statistically significant.

---

## Summary

| dApp | Formula tested verbatim | Passes expected distribution |
|------|---|---|
| dice-roll | `(u32_le(rand[0..4]) % 6) + 1` | **yes** · χ² = 2.50 |
| lucky-wheel | weighted-segment walk over `rand[0..4] % 100` | **yes** · all segments hit, proportions match |
| prediction-market (binary) | `rand[0] & 1` | **yes** · 52 / 48 |
| prediction-market (threshold) | `u64_le(rand[0..8]) > 2⁶³` | **yes** · 42 / 58 within sampling noise |

**Randomness provenance.** Every one of the 48 rounds was produced by the v7.5 single-shot `submit_round_v2` flow: 4 ESP32-S3 devices generating hardware RNG entropy, commit-reveal on-chain, SHA-256 of the 4 entropies as the final random output. The randomness is identical to what each dApp's `dice_callback` would receive if wired via CPI. Each dApp's formula consumes the prescribed bytes and produces a sensible outcome — no edge cases (unresolved, out-of-range, NaN-equivalent) observed.

## What this test doesn't cover (follow-up)

- **On-chain CPI callback** — `dice::deliver_callback` → `<dapp>::dice_callback` invocation and `roll.settled = true` state mutation.
- **Per-dApp setup ix** — `initialize`, `create_wheel`, `create_market` (needs its own driver).
- **The `claim` path** — player-wins → vault-payout lamport transfer.

These are straightforward to wire; each dApp already has a callback handler matching the DICE `[u8;8]` discriminator `[128, 131, 129, 45, 53, 113, 215, 151]` and the `channel_key + randomness` args. Pattern already proved end-to-end by `coin_toss_driver`.
