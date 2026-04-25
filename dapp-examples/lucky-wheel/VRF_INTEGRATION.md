# Lucky Wheel — DICE VRF Integration Guide

## Overview

A provably fair spin-to-win game with configurable weighted segments.
The house owner defines segments with different multipliers and probabilities.
DICE VRF's hardware randomness determines which segment the wheel lands on.

## How It Works

```
┌──────────┐         ┌───────────────────┐         ┌──────────────────┐
│  Player  │         │  Lucky Wheel      │         │  DICE VRF Oracle │
│          │         │  (Solana program)  │         │  (ESP32-S3 HW)   │
└────┬─────┘         └────────┬──────────┘         └────────┬─────────┘
     │                        │                             │
     │  spin(wager=1 SOL)     │                             │
     │───────────────────────>│                             │
     │                        │                             │
     │                        │  request_randomness(seq)    │
     │                        │────────────────────────────>│
     │                        │                             │
     │                        │   [4-7 ESP32 nodes run      │
     │                        │    commit-reveal protocol]  │
     │                        │                             │
     │                        │  dice_callback(randomness)  │
     │                        │<────────────────────────────│
     │                        │                             │
     │                        │  Weighted selection:        │
     │                        │  ┌──────────────────────┐   │
     │                        │  │ 2x   │ 5x  │10x│ 0 │50│ │
     │                        │  │ 40%  │ 25% │10%│20%│5%│ │
     │                        │  └──────────────────────┘   │
     │                        │  rand → point=72 → 10x!    │
     │                        │                             │
     │  claim() → 10 SOL      │                             │
     │<───────────────────────│                             │
```

## Wheel Configuration Example

```
Segment 0: "2x"      weight=40  multiplier=200   (40% chance, 2x payout)
Segment 1: "5x"      weight=25  multiplier=500   (25% chance, 5x payout)
Segment 2: "10x"     weight=10  multiplier=1000  (10% chance, 10x payout)
Segment 3: "LOSE"    weight=20  multiplier=0     (20% chance, 0x payout)
Segment 4: "JACKPOT" weight=5   multiplier=5000  (5% chance, 50x payout)
                     ─────
                     total=100

Expected value per spin: 0.4*2 + 0.25*5 + 0.1*10 + 0.2*0 + 0.05*50
                       = 0.8 + 1.25 + 1.0 + 0 + 2.5 = 5.55x
House edge: -455% (house loses in this config — adjust weights for house edge)
```

## VRF Call: Weighted Random Selection

### The Algorithm

```rust
pub fn dice_callback(ctx, _channel_key: Pubkey, randomness: [u8; 32]) -> Result<()> {
    // Step 1: Convert first 4 bytes of hardware entropy to u32
    let rand_u32 = u32::from_le_bytes([randomness[0], randomness[1], randomness[2], randomness[3]]);
    
    // Step 2: Map to range [0, total_weight)
    let point = rand_u32 % (total_weight as u32);
    
    // Step 3: Walk segments, accumulate weights
    let mut accumulated: u32 = 0;
    for i in 0..segment_count {
        accumulated += segment_weights[i];
        if point < accumulated {
            // This segment wins!
            winner = i;
            break;
        }
    }
    
    // Step 4: Calculate payout
    let payout = wager * multiplier / 100;
}
```

### Visual Example

```
Randomness from ESP32: a9 a3 29 14 ...
rand_u32 = 0x1429a3a9 = 338,469,801
point = 338469801 % 100 = 1

Segment walk:
  Segment 0 (2x):  accumulated = 40.  point=1 < 40? YES → Winner!
  
Result: 2x multiplier
Payout: 1 SOL * 200 / 100 = 2 SOL
```

Another example:
```
rand_u32 = 0x8F3A2B1C = 2,403,896,092
point = 2403896092 % 100 = 92

Segment walk:
  Segment 0: accumulated = 40.  92 < 40? NO
  Segment 1: accumulated = 65.  92 < 65? NO
  Segment 2: accumulated = 75.  92 < 75? NO
  Segment 3: accumulated = 95.  92 < 95? YES → Winner!

Result: Segment 3 (LOSE, 0x multiplier)
Payout: 0 SOL
```

## Why Hardware VRF Matters for Wheels

A software-based RNG could be:
- **Predicted** by miners who see the block hash before others
- **Manipulated** by the house operating the RNG server
- **Replayed** to find favorable outcomes

DICE VRF prevents all of this:
- **Hardware TRNG** produces physically random bits from quantum noise
- **Commit-reveal** ensures nobody can see the result before committing
- **Multiple nodes** must agree — no single point of manipulation
- **On-chain proof** — anyone can verify the randomness was fairly generated

## Accounts

| Account | Type | Description |
|---------|------|-------------|
| `Wheel` | PDA `[b"wheel"]` | Wheel config: segments, weights, multipliers |
| `SpinAccount` | PDA `[b"spin", player, spin_id]` | Individual spin state + result |
| `Vault` | PDA `[b"vault"]` | Holds all wagers |

## Cost

- VRF request: **0.002 SOL** per spin
- Transaction fees: ~0.000005 SOL
- Account rent: ~0.002 SOL (refundable)
