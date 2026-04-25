# Prediction Market — DICE VRF Integration Guide

## Overview

A Polymarket-style binary prediction market where users bet YES or NO on outcomes.
When the market deadline passes, DICE VRF provides verifiable randomness to
resolve the market — ensuring no single party can manipulate the result.

## How It Works

```
┌──────────┐         ┌───────────────────┐         ┌──────────────────┐
│  Creator │         │  Prediction Market│         │  DICE VRF Oracle │
│          │         │  (Solana program)  │         │  (ESP32-S3 HW)   │
└────┬─────┘         └────────┬──────────┘         └────────┬─────────┘
     │                        │                             │
     │  create_market(        │                             │
     │    "Will X happen?",   │                             │
     │    deadline=Friday)    │                             │
     │───────────────────────>│                             │
     │                        │                             │
┌────┴─────┐                  │                             │
│  Users   │                  │                             │
│          │                  │                             │
│  place_position(YES, 10SOL) │                             │
│  place_position(NO, 5SOL)   │                             │
│  place_position(YES, 3SOL)  │                             │
│────────────────────────────>│                             │
│                             │  Pool: YES=13 SOL           │
│                             │        NO=5 SOL             │
│                             │        Total=18 SOL         │
│                             │                             │
│  [Friday arrives]           │                             │
│                             │                             │
│  resolve_market() ────────>│                             │
│                             │  request_randomness ──────>│
│                             │                             │
│                             │  [ESP32 commit-reveal]      │
│                             │                             │
│                             │  dice_callback(randomness)  │
│                             │<────────────────────────────│
│                             │                             │
│                             │  outcome = rand[0] & 1      │
│                             │  0 = YES wins!              │
│                             │                             │
│  claim_winnings() ─────────>│                             │
│  YES bettors split 18 SOL   │                             │
│  proportionally             │                             │
│<────────────────────────────│                             │
```

## VRF Call: Market Resolution

### Resolution Modes

#### Binary Mode (mode=0) — Simple 50/50
```rust
let outcome = randomness[0] & 1;
// 0 = YES wins (128/256 byte values = exactly 50%)
// 1 = NO wins  (128/256 byte values = exactly 50%)
```

#### Threshold Mode (mode=1) — Precision 50/50
```rust
let value = u64::from_le_bytes(randomness[0..8]);
let threshold = u64::MAX / 2;
// value > threshold = YES wins
// value <= threshold = NO wins
// Uses 64 bits of entropy for higher precision
```

### Payout Calculation

```
Market: "Will the next DICE output start with 0x?"
  YES pool: 13 SOL (from 3 bettors)
  NO pool:   5 SOL (from 2 bettors)
  Total:    18 SOL

VRF resolves: YES wins (randomness[0] = 0x4A → even → YES)

Payouts (proportional):
  Player A bet 10 SOL on YES → 18 * (10/13) = 13.85 SOL (+38.5% profit)
  Player B bet  3 SOL on YES → 18 * (3/13)  =  4.15 SOL (+38.5% profit)
  Player C bet  5 SOL on NO  → 0 SOL (lost)
```

### The VRF Callback

```rust
pub fn dice_callback(ctx, _channel_key: Pubkey, randomness: [u8; 32]) -> Result<()> {
    let market = &mut ctx.accounts.market;
    
    // Determine outcome from hardware entropy
    let outcome = match market.resolution_mode {
        0 => randomness[0] & 1,                    // Binary: even/odd
        1 => {                                      // Threshold: u64 comparison
            let value = u64::from_le_bytes(randomness[0..8].try_into().unwrap());
            if value > u64::MAX / 2 { 0 } else { 1 }
        }
        _ => unreachable!(),
    };
    
    market.outcome = outcome;  // 0 = YES, 1 = NO
    market.resolved = true;
    market.randomness = randomness;  // Store proof on-chain
}
```

### Claiming Winnings

```rust
pub fn claim_winnings(ctx) -> Result<()> {
    // Only winning side can claim
    require!(position.side == market.outcome, MarketError::PositionLost);
    
    // Proportional payout from total pool
    let total_pool = market.yes_pool + market.no_pool;
    let winning_pool = if market.outcome == 0 { market.yes_pool } else { market.no_pool };
    let payout = total_pool * position.amount / winning_pool;
    
    // Transfer to winner
}
```

## Why VRF for Prediction Markets?

### Traditional Oracle vs VRF Resolution

| Property | Traditional Oracle | DICE VRF |
|----------|--------------------|----------|
| Trust | Must trust the oracle provider | Trustless — hardware entropy |
| Manipulation | Oracle can lie about outcome | Commit-reveal prevents manipulation |
| Proof | Off-chain attestation | On-chain: randomness + commit hash |
| Speed | Variable (oracle response time) | ~2 seconds (commit-reveal) |
| Cost | Variable ($0.01-$10 per query) | Fixed: 0.002 SOL (~$0.30) |

### When to Use VRF Resolution

VRF-resolved markets are ideal for:
- **Lottery/raffle markets** — "Will ticket #42 win?"
- **Randomness-based events** — "Will the next block hash end in 0?"
- **Sports props with random elements** — "First coin toss result?"
- **Decentralized governance** — Random selection of validators/jurors
- **Any binary event** where a fair, verifiable random outcome is needed

### When NOT to Use VRF

For markets about real-world events ("Will SOL hit $300?"), you need
a price oracle (like Pyth/Switchboard), not VRF. VRF resolves markets
where the outcome should BE random, not where it depends on external data.

## Accounts

| Account | Type | Description |
|---------|------|-------------|
| `Market` | PDA `[b"mkt_acct", market_id]` | Market config: question, deadline, pools |
| `Position` | PDA `[b"position", market, player]` | Individual bet (side + amount) |
| `Vault` | PDA `[b"mkt_vlts", market_id]` | Holds all bets for this market |

## Cost

- Market creation: ~0.003 SOL (account rent)
- VRF request: **0.002 SOL** (paid once at resolution)
- Position placement: ~0.002 SOL (account rent)
- Claim: ~0.000005 SOL (transaction fee)
