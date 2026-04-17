# DICE VRF — 60-second integration

Copy `src/lib.rs` into your Anchor project. Change 3 things. Deploy. Done.

## The 3 things to change

1. **`declare_id!(...)`** — your program ID.
2. **`OUTCOME_FORMULA`** (inside `dice_callback`, ~5 lines) — how random bytes → your game's outcome.
   ```rust
   // dice (1-6):
   let outcome = (u32::from_le_bytes([r[0],r[1],r[2],r[3]]) % 6) as u8 + 1;
   // coin flip:
   let outcome = r[0] & 1;
   // weighted wheel:
   let point = u32::from_le_bytes([r[0],r[1],r[2],r[3]]) % total_weight;
   // lottery pick:
   let winner_idx = u64::from_le_bytes([r[0],r[1],r[2],r[3],r[4],r[5],r[6],r[7]]) as usize % players.len();
   ```
3. **`GameState` struct** — whatever per-round data you want to persist.

That's it. Wiring to DICE is one CPI call in `play()` and one handler for `dice_callback` — both already written for you.

## One-time setup (client-side, once per dApp)

```ts
// Create a DiceChannel for your program. Do this once at deploy time.
await dice.methods
  .initChannel(channelIndex, maxNodes, yourProgram.programId, coordinatorPubkey)
  .rpc();

// Pre-fund the channel so request_randomness_auto doesn't need top-ups per round.
await dice.methods.fundChannel(1_000_000_000n).rpc(); // 1 SOL buys ~500 rounds at 0.002 SOL each.
```

## Per-round UX — ONE TX from the client

```ts
await yourProgram.methods
  .play(bet, wager, roundNonce)
  .accounts({ player, game, vault, diceChannel, diceProgram: DICE_PROGRAM_ID, ... })
  .rpc();
// ~4 seconds later, game.settled === true with the outcome on-chain.
```

Compared to the non-CPI pattern used by the reference `dice-roll` / `lucky-wheel` / `prediction-market` dApps (which require the client to send TWO TXs per action — first the game action, then the DICE `request_randomness_auto`), this template **halves client-side complexity** and removes the "what if TX A lands but B fails?" race.

## Observed round latency (v7.5 + L8, devnet)

- Streaming selection (coord picks devices): **~3.8 s avg, 4.3 s p95**
- Audit selection (on-chain Fisher-Yates): **~4.1 s avg, 4.7 s p95**
- 100 % hardware-backed entropy · 4× ESP32-S3 nodes · 0.002 SOL per request.

## Files

- `Cargo.toml` — one dep: `dice = { features = ["cpi", "no-entrypoint"] }`.
- `src/lib.rs` — complete compilable dApp (~180 lines with comments, ~80 lines of code).

## What you still get for free

- Commit-reveal security: any 1 of 4 honest devices → result unbiasable.
- Verifiable-selection mode (opt-in via the SDK, no dApp change needed).
- 70 % of the fee routed to node operators (you don't pay them, DICE does).
- mTLS device ↔ coordinator channel (ops burden on DICE, not you).
