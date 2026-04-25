# Dice Roll — VRF Test Results

**Program ID:** `CLpaMPxyu5Up4fuZb1JiY2uzj4s4iYVg9RfQHNFRuzAj`
**Network:** Solana Devnet
**VRF Source:** ESP32-S3-N16R8 (MAC: 1c:db:d4:46:c8:b4)
**Date:** April 5, 2026

## Test: 10 Consecutive Dice Rolls

Each roll uses hardware-generated entropy from ESP32-S3's TRNG.

| Roll | Randomness (first 16 hex) | u32 (LE) | % 6 + 1 | Result |
|------|--------------------------|----------|---------|--------|
| 1 | `d3a49551163d02c9` | 1,368,761,555 | 6 | **6** |
| 2 | `2cb09c0ee6b37562` | 245,149,740 | 1 | **1** |
| 3 | `8ec6dda884827fcf` | 2,833,106,574 | 1 | **1** |
| 4 | `2192ec89d4cecb0f` | 2,313,982,497 | 4 | **4** |
| 5 | `d3635a3fa4af4909` | 1,062,888,403 | 2 | **2** |
| 6 | `a448a7d71185c7fe` | 3,618,064,548 | 1 | **1** |
| 7 | `b74cd5ffac48a74d` | 4,292,168,887 | 2 | **2** |
| 8 | `64a996bbecaf771f` | 3,147,213,156 | 1 | **1** |
| 9 | `96b9d92337c17171` | 601,471,382 | 3 | **3** |
| 10 | `9519c1db72c16adc` | 3,686,865,301 | 2 | **2** |

## Distribution

| Number | Count | Expected | Actual |
|--------|-------|----------|--------|
| 1 | 4 | 1.67 | 40% |
| 2 | 3 | 1.67 | 30% |
| 3 | 1 | 1.67 | 10% |
| 4 | 1 | 1.67 | 10% |
| 5 | 0 | 1.67 | 0% |
| 6 | 1 | 1.67 | 10% |

Small sample size — distribution normalizes with more rolls (confirmed in 446-round battle test: near-perfect uniformity).

## How This VRF Round Worked

```
1. Coordinator dispatched JobAssignment to ESP32-S3 (MAC: 1c:db:d4:46:c8:b4)
2. ESP32-S3 generated entropy:
   - Hardware TRNG (ring oscillator quantum noise)
   - Floating ADC pin (GPIO1 thermal/EMI noise)
   - FreeRTOS timing jitter
   - Mixed via XOR → SHA-256 finalization
3. Device sent SHA-256(entropy) as commit (ECDSA signed)
4. Coordinator broadcast reveal signal
5. Device revealed raw entropy (ECDSA signed)
6. Coordinator verified: SHA-256(entropy) == commit ✓
7. Combined entropy → final randomness
8. dice_callback(randomness) called on Dice Roll program
9. result = (u32(randomness[0..4]) % 6) + 1

Total time: ~1.5 seconds per roll
```

## Verification

Anyone can verify the result:
1. Randomness `d3a49551163d02c9...` is on-chain
2. First 4 bytes as LE u32: `0x519504d3` = 1,368,761,555
3. 1,368,761,555 % 6 = 5
4. 5 + 1 = **6** ✓
