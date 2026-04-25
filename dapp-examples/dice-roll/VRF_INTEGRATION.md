# Dice Roll — DICE VRF Integration Guide

## Overview

A provably fair dice game (1-6) where the outcome is determined by hardware-backed
VRF from ESP32-S3 nodes. Players bet on a number, and the result cannot be
manipulated by anyone — not the house, not the player, not the coordinator.

## How It Works

```
┌──────────┐         ┌───────────────────┐         ┌──────────────────┐
│  Player  │         │  Dice Roll        │         │  DICE VRF Oracle │
│          │         │  (Solana program)  │         │  (ESP32-S3 HW)   │
└────┬─────┘         └────────┬──────────┘         └────────┬─────────┘
     │                        │                             │
     │  roll_dice(bet=3,      │                             │
     │    wager=1 SOL)        │                             │
     │───────────────────────>│                             │
     │                        │                             │
     │                        │  request_randomness(seq)    │
     │                        │────────────────────────────>│
     │                        │                             │
     │                        │    ESP32-S3 Hardware:       │
     │                        │    1. TRNG ring oscillator  │
     │                        │    2. ADC floating pin noise│
     │                        │    3. FreeRTOS timing jitter│
     │                        │    4. SHA-256 mix           │
     │                        │    5. ECDSA sign            │
     │                        │    6. Commit → Reveal       │
     │                        │                             │
     │                        │  dice_callback(randomness)  │
     │                        │<────────────────────────────│
     │                        │                             │
     │                        │  result = (u32(rand) % 6) + 1
     │                        │  if result == bet: WIN 5x   │
     │                        │                             │
     │  claim() → 5 SOL       │                             │
     │<───────────────────────│                             │
```

## VRF Call: Step by Step

### Step 1: Player Places Bet
```rust
pub fn roll_dice(ctx: Context<RollDice>, bet: u8, wager: u64, roll_id: u64) -> Result<()> {
    // Validate bet is 1-6
    require!(bet >= 1 && bet <= 6, DiceRollError::InvalidBet);
    
    // Store bet and transfer wager to vault PDA
    roll.bet = bet;
    roll.wager = wager;
    anchor_lang::system_program::transfer(cpi_ctx, wager)?;
}
```

### Step 2: DICE VRF Generates Randomness
The coordinator detects the on-chain request and dispatches to ESP32 nodes:

1. **ESP32 generates entropy**: Hardware TRNG (ring oscillator) + floating ADC pin + timing jitter
2. **Commit phase**: Node sends `SHA-256(entropy)` to coordinator
3. **Reveal phase**: Node reveals raw entropy + ECDSA signature
4. **Finalize**: Coordinator combines entropy from all nodes → `SHA-256(e1 || e2 || ... || eN)`

### Step 3: Callback Resolves the Game
```rust
pub fn dice_callback(ctx: Context<DiceRollCallback>, _channel_key: Pubkey, randomness: [u8; 32]) -> Result<()> {
    // Convert 32 bytes of hardware randomness to a dice roll (1-6)
    let rand_u32 = u32::from_le_bytes([randomness[0], randomness[1], randomness[2], randomness[3]]);
    let result = (rand_u32 % 6) as u8 + 1;
    
    // Record result
    roll.result = result;
    roll.settled = true;
}
```

### Step 4: Player Claims Winnings
```rust
pub fn claim(ctx: Context<ClaimRoll>) -> Result<()> {
    require!(roll.result == roll.bet, DiceRollError::PlayerLost);
    let payout = roll.wager * 5; // 5x for 1-in-6 odds
    // Transfer payout from vault to player
}
```

## Randomness → Dice Roll Conversion

```
32 bytes from ESP32 TRNG:
  a9 a3 29 14 90 93 a7 1f 5b ab 77 94 95 69 92 74
  f9 00 ab 6e 71 eb 63 bf 51 0e 54 ae 68 85 7f f8

Take first 4 bytes as little-endian u32:
  rand_u32 = 0x1429a3a9 = 338,469,801

Apply modular arithmetic:
  result = (338469801 % 6) + 1 = (3) + 1 = 4

The dice shows: 4
```

## Fairness Guarantee

- **Hardware entropy**: ESP32-S3 TRNG uses ring oscillator quantum noise
- **Multi-source mixing**: 3 independent entropy sources XOR-mixed + SHA-256
- **Commit-reveal**: Nobody can see the entropy until all nodes have committed
- **On-chain verification**: SHA-256(revealed_entropy) must match the commit hash
- **ECDSA signed**: Every commit and reveal is cryptographically signed by the device
- **No manipulation**: The coordinator cannot change the output — it's bound by the commit

## Accounts

| Account | Type | Description |
|---------|------|-------------|
| `House` | PDA `[b"house"]` | Game configuration and statistics |
| `Roll` | PDA `[b"roll", player, roll_id]` | Individual roll state |
| `Vault` | PDA `[b"vault"]` | Holds all wagers |

## Cost

- Request fee: **0.002 SOL** (paid to DICE oracle nodes)
- Transaction fees: ~0.000005 SOL (standard Solana fees)
- Total per roll: **~0.002 SOL**
