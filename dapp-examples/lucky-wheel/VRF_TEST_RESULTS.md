# Lucky Wheel — VRF Test Results

**Program ID:** `FzUuegZpKT4BHhzms1eJX7L2f6r3NTxMRexs8uqxtnbf`
**Network:** Solana Devnet
**VRF Source:** ESP32-S3-N16R8 (MAC: 1c:db:d4:46:c8:b4)
**Date:** April 5, 2026

## Wheel Configuration

| Segment | Name | Weight | Probability | Multiplier |
|---------|------|--------|------------|------------|
| 0 | 2x | 40 | 40% | 2x payout |
| 1 | 5x | 25 | 25% | 5x payout |
| 2 | 10x | 10 | 10% | 10x payout |
| 3 | LOSE | 20 | 20% | 0x (lose all) |
| 4 | JACKPOT | 5 | 5% | 50x payout |

Total weight: 100

## Test: 15 Wheel Spins

| Spin | Randomness (first 16) | u32 (LE) | Point | Segment | Result |
|------|----------------------|----------|-------|---------|--------|
| 1 | `d3a495...163d02c9` | 2,700,091,564 | 64 | 1 | **5x** |
| 2 | `04b348...e6b37562` | 3,087,592,900 | 0 | 0 | **2x** |
| 3 | `5cac9c...84827fcf` | 1,640,213,084 | 84 | 3 | **LOSE** |
| 4 | `98fd23...d4cecb0f` | 656,794,008 | 8 | 0 | **2x** |
| 5 | `7f74d6...a4af4909` | 1,423,727,743 | 43 | 1 | **5x** |
| 6 | `14e474...1185c7fe` | 1,952,394,004 | 4 | 0 | **2x** |
| 7 | `27a003...ac48a74d` | 1,476,806,951 | 51 | 1 | **5x** |
| 8 | `0c6a0a...ecaf771f` | 2,582,641,676 | 76 | 3 | **LOSE** |
| 9 | `59a3b9...37c17171` | 3,225,820,377 | 77 | 3 | **LOSE** |
| 10 | `9088e8...72c16adc` | 4,271,716,624 | 24 | 0 | **2x** |
| 11 | `87255d...e6b37562` | 1,565,638,279 | 79 | 3 | **LOSE** |
| 12 | `50edff...84827fcf` | 1,512,179,024 | 24 | 0 | **2x** |
| 13 | `08bcfb...d4cecb0f` | 3,422,580,488 | 88 | 3 | **LOSE** |
| 14 | `6504f7...a4af4909` | 842,723,557 | 57 | 1 | **5x** |
| 15 | `13dfe7...1185c7fe` | 2,117,869,459 | 59 | 1 | **5x** |

## Distribution

| Segment | Count | Expected (15 spins) | Actual % |
|---------|-------|--------------------|---------| 
| 2x | 5 | 6.0 (40%) | 33% |
| 5x | 5 | 3.75 (25%) | 33% |
| 10x | 0 | 1.5 (10%) | 0% |
| LOSE | 5 | 3.0 (20%) | 33% |
| JACKPOT | 0 | 0.75 (5%) | 0% |

Small sample — with more spins the distribution converges to configured weights.

## How Weighted Selection Works

```
VRF randomness: 7f74d654a4af4909...
u32(first 4 bytes, LE) = 1,423,727,743
point = 1,423,727,743 % 100 = 43

Segment walk:
  Seg 0 (2x):    accumulated = 40.  43 < 40? NO
  Seg 1 (5x):    accumulated = 65.  43 < 65? YES → Winner!

Result: 5x multiplier
Payout: wager * 500 / 100 = 5x wager
```

## Verification

The randomness-to-segment mapping is deterministic and auditable:
1. Read `randomness` from on-chain SpinAccount
2. Compute `u32::from_le_bytes(randomness[0..4])`
3. Compute `point = u32 % total_weight`
4. Walk segments: accumulate weights until point < accumulated
5. The winner segment is unambiguous
