# V7 Universal Payout System — NodeVault Implementation Progress

**Branch:** v7
**Started:** 2026-04-13
**Status:** Phase 1 — in progress
**Owner:** hariFED + Claude (Opus 4.6, 1M context)

---

## Why this document exists

This document is a **context lifeline**. Claude's context window can compact mid-session, so every architectural decision, every file touched, every test run, every deployment step gets recorded here. If this work is resumed in a new session, reading this file top-to-bottom should bring anyone back to full speed in under five minutes.

**Resume rule:** before writing any new code in this feature, re-read this file end-to-end and update the "Status" fields.

---

## Goal (plain English)

Build a universal payout primitive so that:

1. Every ESP32-S3 node has **one** on-chain vault account (`NodeVault`).
2. Every DICE service (VRF v1, VRF v2, v8 PoL, v8 Sensor Attestation, v8 HSM, any future service) deposits into the same vault.
3. The operator plugs in the device, enters their Solana wallet address **once** via the captive portal, and earnings from every service land in that wallet.
4. The wallet binding is **hardware-signed by the device** so it cannot be forged by the coordinator, intercepted on the wire, or phished through a fake form.
5. All this ships in **v7** — no deferral to v8 — because:
   - v2 VRF channels currently have **zero** payout path (silent revenue bug)
   - Pitch claims "operators earn on every round" — that must be true
   - Building it per-service in v8 means rewriting economics four times

---

## Why this is in v7 (not v8)

The audit originally scoped Task #2 as "wire the existing claim_rewards TX post-finalization." When I traced the code, I found:

- **v1 `claim_rewards`** exists and works correctly. Just never called. Easy fix.
- **v2 channels have NO equivalent**. `request_randomness_auto` decrements `channel.balance` — but this is an accounting operation only. No lamports move. The fee sits inert in the channel PDA forever. **Nodes earn nothing on v2 rounds.**
- The v2 channel flow is the modern path. v1 is legacy.

So "wire the existing instruction" fixes only the legacy path. Real v7 traffic still pays nodes nothing. That's a total failure relative to the pitch, so we're building it right:

- Universal `NodeVault`
- `credit_vault` helper reused by v1, v2, and every v8 service
- Hardware-signed wallet binding

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      NodeVault PDA                           │
│         Seeds: ["node_vault", device_pubkey]                 │
├─────────────────────────────────────────────────────────────┤
│  device_pubkey        ← [u8; 33] compressed secp256k1 key    │
│  payout_wallet        ← Pubkey (zero before binding)         │
│  total_earned         ← u64 (lifetime, all services)         │
│  total_withdrawn      ← u64 (lifetime)                       │
│  binding_slot         ← u64 (cooldown anchor for rotation)   │
│  binding_signature    ← [u8; 64] latest ECDSA binding        │
│  binding_nonce        ← [u8; 32] last coordinator nonce      │
│  binding_timestamp    ← i64                                   │
│  status               ← Unbound / Bound / Frozen             │
│  created_slot         ← u64                                   │
│  last_credit_slot     ← u64                                   │
│  service_counts       ← [u64; 8] per-service earning count   │
└─────────────────────────────────────────────────────────────┘
         ↑                                    ↓
    credit_vault()                     withdraw_from_vault()
    (internal helper)                  (operator-signed only)
         │                                    ↓
    called by:                          payout_wallet
  ┌──────┴──────────────┐                   (pays self)
  │ claim_rewards (v1)  │
  │ claim_rewards_v2    │   ← NEW instruction
  │ v8 PoL payout       │   ← future
  │ v8 sensor payout    │   ← future
  │ v8 HSM signing fee  │   ← future
  └─────────────────────┘
```

### Key properties

1. **One PDA per device** — deterministic discovery via device pubkey.
2. **Services never know the operator wallet** — they call `credit_vault(device_pubkey, amount)` and walk away.
3. **Wallet binding is hardware-signed** — operator types wallet into captive portal; device signs `(device_pubkey || payout_wallet || timestamp || coordinator_nonce)` with its ECDSA key; coordinator submits `register_node_vault` with the signature; Anchor verifies via `secp256k1_recover`.
4. **Unbound vaults still accumulate** — if a device earns before binding, no payout is lost.
5. **Rotation requires dual signature** (device + current wallet) + cooldown — prevents theft scenarios.
6. **Treasury and reserve stay separate** — they receive fees via direct transfer at service-instruction time, as today. Only per-device shares go through vaults.

### Binding flow (end to end)

```
Operator plugs in device
         │
         ▼
Captive portal form: WiFi creds + Payout Wallet
         │
         ▼
Device computes:
  msg = SHA256(
    "DICE_PAYOUT_BINDING_V1" ||
    device_pubkey            ||
    payout_wallet            ||
    timestamp                ||
    coordinator_nonce
  )
  signature = ecdsa_sign(device_hw_key, msg)
         │
         ▼
Device → Coordinator (mTLS WebSocket):
  PayoutBindingRequest {
    device_pubkey, payout_wallet,
    timestamp, nonce, signature
  }
         │
         ▼
Coordinator submits on-chain:
  register_node_vault(
    device_pubkey,
    payout_wallet,
    timestamp,
    nonce,
    signature
  )
         │
         ▼
Anchor instruction:
  1. Derive vault PDA
  2. require!(status == Unbound)
  3. Rebuild msg bytes
  4. secp256k1_recover(msg_hash, sig, recovery_id) → pubkey
  5. require!(recovered == device_pubkey)
  6. Store wallet + signature + slot + nonce + ts
  7. status = Bound
```

---

## Phases

### Phase 1 — NodeVault primitive (Anchor only)
**Status:** 🟢 complete (local build + unit tests), awaiting devnet deploy

**Goal:** ship the on-chain state account, the four management instructions, and the `credit_vault` helper — fully tested, compiling, unit-tested. No VRF integration yet. No coordinator changes yet. No firmware changes yet.

**Files to create:**
- `programs/dice/src/state/node_vault.rs` — `NodeVault` + `NodeVaultStatus`
- `programs/dice/src/instructions/register_node_vault.rs`
- `programs/dice/src/instructions/rotate_payout_wallet.rs`
- `programs/dice/src/instructions/withdraw_from_vault.rs`
- `programs/dice/src/instructions/credit_vault_helper.rs` — pub fn, not an instruction

**Files to modify:**
- `programs/dice/src/state/mod.rs`
- `programs/dice/src/instructions/mod.rs`
- `programs/dice/src/lib.rs` — register 3 new entrypoints
- `programs/dice/src/constants.rs` — `SEED_NODE_VAULT`, `ROTATION_COOLDOWN_SLOTS`, `PAYOUT_BINDING_DOMAIN`
- `programs/dice/src/error.rs` — new error codes

**Test targets:**
- `NodeVault::space()` is correct
- `NodeVault::credit()` updates totals and slot
- `NodeVault::withdraw()` enforces balance
- Signature verification round-trips
- Unbound → Bound state transition
- Bound → Bound rotation (dual sig, cooldown)
- Rejection: wrong signer, zero amount, insufficient balance, stale nonce, cooldown not elapsed
- Seed collision check against existing PDAs

**Completion criteria:**
- `cargo check -p dice` clean
- `cargo test -p dice --lib` → all tests pass, zero regressions against existing 37 tests
- Tests for: space, credit, withdraw, bind, rebind, rotate, negative cases

### Phase 2 — VRF integration
**Status:** 🟢 complete (local build + unit tests)
**Scope decision:** v2-only. New `claim_rewards_v2` instruction. v1 legacy `claim_rewards` left alone — its latent bugs are tracked in Task #14 and its refactor would expand scope without helping v7's pitch (which runs on v2 channels). Also caught a real PDA seed bug — Solana PDA seeds are max 32 bytes, and I was using 33-byte device pubkeys directly. Fixed to use `device_id = SHA256(device_pubkey)` (32 bytes) which mirrors existing DICE patterns.

**Goal:** make v1 and v2 VRF pay nodes through NodeVault.

**Changes:**
- Refactor `claim_rewards` (v1) to call `credit_vault` helper instead of transferring directly to `node_wallet`
- Write new `claim_rewards_v2` instruction that:
  - Iterates `channel.device_pubkeys`
  - Calls `credit_vault` helper for each contributing node's share
  - Transfers treasury + reserve shares to the configured protocol wallets
  - Marks the v2 round as rewards-distributed (new flag on DiceChannel or new state)
  - Prevents double-claim on the same round
- SDK additions: `claim_rewards_v2_ix` CPI builder + tests
- Unit tests: payout math, double-claim rejection, treasury/reserve math

**Completion criteria:**
- `cargo test -p dice --lib` green
- `cargo test -p dice-vrf --lib` green
- Proof that a synthetic v2 round finalization + claim reduces channel lamports by exactly the fee, splits correctly into vault/treasury/reserve

### Phase 3 — Coordinator wiring
**Status:** 🟢 complete
**What shipped:**
- `OnChainCtx` gained `treasury: Pubkey` and `reserve: Pubkey` fields
- `Config` gained `--treasury` / `DICE_TREASURY` and `--reserve` / `DICE_RESERVE` optional CLI/env flags
- `main.rs:95-130` parses them, warns if unset, passes into OnChainCtx
- `solana_tx.rs` gained `anchor_discriminator` helper, `SEED_NODE_VAULT`, `node_vault_pda`, `build_register_node_vault_ix`, `build_rotate_payout_wallet_ix`, `build_withdraw_from_vault_ix`, `build_claim_rewards_v2_ix`
- `main.rs:729-770` wires `claim_rewards_v2` into the v2 finalize flow — if treasury + reserve are configured and nodes are selected, `finalize_v2 + claim_rewards_v2` are bundled in ONE transaction so they atomically succeed or fail together
- 7 new coordinator unit tests cover discriminator determinism, PDA derivation, instruction layout

**Goal:** coordinator actually submits claim_rewards post-finalization and handles binding requests from firmware.

**Changes:**
- Add `treasury_pubkey`, `reserve_pubkey` fields to `OnChainCtx` (configured via CLI flag or env var)
- Add `NodeVault` PDA cache keyed by device pubkey
- After `finalize_v2` succeeds in `main.rs:705`, submit `claim_rewards_v2` in the same flow
- Similarly wire v1 path after `finalize_randomness`
- New coordinator-side message handler: `PayoutBindingRequest` from firmware → submit `register_node_vault` or `rotate_payout_wallet`
- New SolanaTx builder: `build_register_node_vault_ix`, `build_rotate_payout_wallet_ix`
- Integration tests

**Completion criteria:**
- `cargo test -p dice-coordinator --bins` green
- Full coordinator still compiles (probably 2+ minutes — warn user)

### Phase 4 — Firmware binding flow
**Status:** 🟢 code complete (manual ESP-IDF build + flash still needed — no toolchain in this session)

**What shipped:**
- `firmware/main/crypto.h` + `crypto.c` — `dice_crypto_sign_payout_binding()` helper: assembles `DOMAIN || device_pubkey || payout_wallet || timestamp_le || nonce`, calls existing `dice_crypto_sign()` which SHA-256s and signs via hardware ECDSA
- `firmware/main/payout_binding.h` + `payout_binding.c` — NEW: `dice_payout_binding_maybe_send()` reads `sol_wallet` from NVS, decodes base58, generates random nonce, signs, encodes as CBOR, ships via WebSocket; `dice_payout_binding_clear_flag()` wipes the "already sent" marker so next boot re-sends
- `firmware/main/app_main.c` — calls `dice_payout_binding_maybe_send()` after WebSocket connects, before entering main loop. Waits up to 10 s for WS to come up.
- `firmware/main/factory_reset.c` — calls `dice_payout_binding_clear_flag()` when wiping `sol_wallet`, so a new binding is sent after the operator re-provisions
- `firmware/main/CMakeLists.txt` — added `payout_binding.c` to SRCS
- `firmware/components/dice_protocol/dice_protocol.h` + `.c` — `DICE_MSG_PAYOUT_BINDING = 5` message type + encoder for the CBOR integer-key map `{0:5, 1:node_id, 2:payout_wallet, 3:timestamp, 4:nonce, 5:signature}`
- Coordinator `protocol/messages.rs` — new `PayoutBindingRequest` struct, integer-key decoder, array-envelope decoder, encoder
- Coordinator `main.rs` — new match arm handles `PayoutBindingRequest`, validates field lengths, submits `register_node_vault` on-chain

### Important: on-chain hash change (Phase 1 follow-up)

Phase 1 originally used `keccak256` for the binding message hash, matching the legacy `submit_reveal.rs` convention. But the firmware's `dice_crypto_sign` internally uses **SHA-256** (via mbedTLS), not keccak. Rather than pull a new keccak library into the ESP32 build, the on-chain `verify_binding_signature` was switched to SHA-256. The firmware can now reuse its existing signer verbatim — no new crypto code on the device.

All 8 Phase 1 signature tests were updated to use SHA-256 and all still pass.

**Goal:** operator plugs in device, enters wallet, device signs binding, coordinator registers it on-chain.

**Changes:**
- Captive portal HTML: add "Payout Wallet" field
- Firmware: parse wallet on form submit, call ECDSA sign over domain-separated binding message
- Firmware: send `PayoutBindingRequest` to coordinator over WebSocket
- Firmware: persist bound wallet in NVS so factory reset is required for re-binding
- Factory reset: clear bound wallet

**Completion criteria:**
- Firmware builds under ESP-IDF v5.2.6 (manual build — not in cargo)
- Binding message layout matches exactly what the on-chain instruction re-computes
- Manual end-to-end test documented in this doc

---

## Security considerations (threat model)

| Threat | Defense |
|---|---|
| Attacker sniffs captive portal submission | mTLS-encrypted WebSocket + hardware ECDSA binding signature |
| Attacker runs a fake captive portal | Device only accepts form input directly, not over network |
| Attacker steals physical device | Re-binding requires factory reset (physical button press) |
| Attacker phishes operator wallet | Wallet rotation requires device signature too (dual signature) |
| Coordinator is compromised | Cannot forge bindings — signature is hardware-generated; coordinator can only propose, Anchor decides |
| Replay of old binding message | Timestamp + coordinator-supplied nonce make each binding unique |
| Rapid rotation churn attack | `ROTATION_COOLDOWN_SLOTS` enforces minimum delay between rotations |
| Double claim on same round | Per-round claimed flag on vault credit source (v1 escrow `is_claimed`, v2 channel new flag) |
| Zero-contributor round | `require!(num_contributors > 0)` before division |
| Lamport overflow on credit | `checked_add` throughout |

---

## Files changed (running list)

### Added (Phase 1 — complete)
- [x] `docs/v7-universal-payout.md` (this file)
- [x] `programs/dice/src/state/node_vault.rs` — 346-byte account + 11 unit tests
- [x] `programs/dice/src/instructions/register_node_vault.rs` — ECDSA verify + 8 signature tests
- [x] `programs/dice/src/instructions/rotate_payout_wallet.rs` — dual-sig + cooldown
- [x] `programs/dice/src/instructions/withdraw_from_vault.rs` — rent-safe PDA withdraw
- [x] `programs/dice/src/instructions/credit_vault_helper.rs` — shared helper for all services

### Modified (Phase 1 — complete)
- [x] `programs/dice/src/state/mod.rs`
- [x] `programs/dice/src/instructions/mod.rs`
- [x] `programs/dice/src/lib.rs` — 3 new #[program] entrypoints
- [x] `programs/dice/src/constants.rs` — SEED_NODE_VAULT + payout constants
- [x] `programs/dice/src/error.rs` — 10 new VaultError variants
- [x] `programs/dice/Cargo.toml` — k256, sha2 dev-deps for signature tests

### Known latent bug (not fixed in this task)
- `programs/dice/src/instructions/submit_reveal.rs:114-116` uses the same buggy `.or_else` chain pattern that I fixed in `verify_binding_signature`. In the submit_reveal case, `secp256k1_recover` with recovery_id=0 can return a valid-but-wrong pubkey, making the `.or_else(|_| try_1)` path dead. In practice, canonical low-s ECDSA signatures bias toward one recovery id so most real traffic works, but edge cases exist. **Flagging for a dedicated fix task in v7.** Do NOT pattern-copy submit_reveal's verifier for new code — use `register_node_vault::verify_binding_signature`'s loop instead.

---

## Tests (running list)

### Pre-existing baseline (must not regress)
- `dice` lib: **37 → 56 tests passing** (+11 NodeVault state, +8 binding signature)
- `dice-vrf` lib: **46 tests passing** (unchanged in Phase 1)
- `dice-coordinator` bins: **102 tests passing** (unchanged in Phase 1)

### Phase 1 test results (all passing)

**NodeVault state tests (11):**
- `space_is_expected_constant_and_under_limit` — vault is 346 bytes, under 10 KB init limit
- `balance_is_earned_minus_withdrawn`
- `balance_saturates_to_zero_if_inconsistent` — defensive
- `record_credit_updates_totals_and_counters` — multi-service credit flow
- `record_credit_rejects_unknown_service` — service_id bounds check
- `record_credit_detects_overflow` — checked_add on u64::MAX edge
- `record_withdraw_respects_balance`
- `record_withdraw_rejects_over_balance`
- `record_withdraw_rejects_zero`
- `partial_withdraw_leaves_residual_balance`
- `status_round_trips_through_borsh`

**Binding signature tests (8):**
- `verify_accepts_valid_signature` — happy path
- `verify_rejects_wrong_device_pubkey`
- `verify_rejects_tampered_wallet` — attacker swap
- `verify_rejects_tampered_timestamp`
- `verify_rejects_tampered_nonce` — replay defense
- `verify_rejects_garbage_signature`
- `verify_domain_separator_is_enforced` — must contain DOMAIN prefix
- `verify_accepts_both_recovery_ids` — iterates 20 seeds, exercises both recovery paths

### Still needed (not in Phase 1 — see later phases)
- Integration tests for instruction handlers (Anchor test harness — Phase 2)
- End-to-end payout flow test (Phase 2/3)
- Firmware binding message generation (Phase 4)

---

## Deployment status

### Devnet
- **Program ID:** `78Qv6cyKkRZN2YngiLSSBCe2iyRc6jgtCs3incCaMRcv` (same as current DICE)
- **Deployer wallet:** `coordinator-keypair.json` OR `~/.config/solana/id.json` — read the pubkey with `solana address` once Solana CLI is installed
- **Status:** ⚪ **all 4 phases code-complete locally, deploy blocked on toolchain availability**

### Why deploy wasn't done in this session
The shell Claude is running in has no `solana`, `anchor`, or `cargo-build-sbf` on PATH. Checked `/c/Users/Abcom/.local/share/solana/install/active_release/bin/`, `/c/solana*/bin/`, `/c/Users/Abcom/AppData/Local/solana*` — none exist. Attempted `where.exe solana` and `Get-Command solana` — neither command is available via git bash. User has indicated they'll handle devnet funding manually (do NOT run `solana airdrop`).

### Deploy runbook (user to execute)

When you're ready to deploy, run these commands from a shell that has `solana` + `anchor` on PATH (PowerShell, cmd, or a properly-configured git bash):

```bash
cd C:\Users\Abcom\DICE

# 1. Confirm deployer pubkey (this is what needs SOL)
solana address

# 2. Point at devnet
solana config set --url devnet

# 3. Check balance — need at least 5 SOL for a ~500 KB program upgrade
solana balance

# 4. If low, airdrop 2 SOL (may require a few retries or use a faucet website)
#    solana airdrop 2
#    (OR fund from your own wallet — user preference)

# 5. Build the BPF binary
#    Using --no-idl because anchor-syn 0.30.1 has a proc_macro2 incompatibility
#    on Rust 1.93+ — IDL is maintained manually at target/idl/dice.json.
anchor build --no-idl

# 6. Deploy the upgraded program. The program ID is pinned in Anchor.toml so
#    this is an UPGRADE, not a fresh deploy. Upgrade costs less SOL than fresh.
anchor deploy --provider.cluster devnet

# 7. Verify the upgrade took effect
solana program show 78Qv6cyKkRZN2YngiLSSBCe2iyRc6jgtCs3incCaMRcv --url devnet
```

### Post-deploy smoke test runbook

```bash
# 1. Start the coordinator with treasury/reserve configured
#    (use any valid Solana pubkey for treasury/reserve on devnet; they're
#    just recipients, no special setup needed)
export DICE_TREASURY="<some_devnet_wallet_pubkey>"
export DICE_RESERVE="<some_other_devnet_wallet_pubkey>"
export SOLANA_RPC_URL="https://api.devnet.solana.com"
cargo run -p dice-coordinator --bin dice-coordinator --release -- --simulation

# 2. From another shell, simulate a round via the dashboard:
curl -X POST http://localhost:8080/simulate

# 3. Look for "finalize_v2 TX sent" in coordinator logs. If payouts_enabled=true,
#    the claim_rewards_v2 instruction is bundled in the same TX. Check Solana
#    Explorer for the signature to confirm:
#    - finalize_v2 logged OK
#    - claim_rewards_v2 split 2_000_000 lamports → 1_400_000 to vaults + 400_000 + 200_000
#    - channel transitioned Finalized -> Idle

# 4. Verify a NodeVault was created for each participating device:
solana account $(solana program show 78Qv6cyKkRZN2YngiLSSBCe2iyRc6jgtCs3incCaMRcv --url devnet | grep -oE '[A-Za-z0-9]{32,44}' | head -1)

# 5. Check vault balance matches what claim_rewards_v2 credited.
```

### Firmware build + flash runbook (Phase 4)

When the ESP-IDF toolchain is available:

```bash
cd firmware
idf.py set-target esp32s3
idf.py build
idf.py -p /dev/ttyUSB0 flash  # or the device's serial port
idf.py -p /dev/ttyUSB0 monitor

# In the captive portal, enter:
#   - WiFi SSID / password
#   - Solana wallet address (32-44 base58 chars)
# Device saves to NVS and reboots.
# After WiFi + WebSocket connect, `dice_payout_binding_maybe_send()` fires
# and ships the PayoutBindingRequest. Coordinator logs should show
# "register_node_vault TX sent".
```

---

## Resume here

If you're picking this up after context loss:

1. **Read this file end-to-end first.**
2. Check `git log v7 --oneline -20` to see what's landed since you last worked.
3. Check `cargo test -p dice --lib` baseline — should be 37 passing + however many new ones have been added in Phase 1.
4. Check this doc's "Files changed" list — anything checked `✅` is in the tree; unchecked items are pending.
5. Current work is whatever phase is marked 🟡 in the Phases section.
6. Do NOT skip the security checklist. Any change to the binding flow, signature layout, or domain separator requires updating the threat table in this file.
7. Do NOT run `solana airdrop` or any faucet. Ask user for devnet SOL by providing the deployer pubkey.

---

## Change log

| Date | Phase | Change |
|---|---|---|
| 2026-04-13 | — | Progress doc created. Phase 1 starting. |
| 2026-04-13 | 1 | Phase 1 complete locally. 56 tests passing (19 new). Awaiting devnet deploy. Latent bug flagged in submit_reveal.rs secp256k1 recovery chain. |
| 2026-04-13 | 2 | Phase 2 complete locally. claim_rewards_v2 instruction + SDK helpers added. 61 dice tests + 56 dice-vrf tests = 117 total, 0 regressions. PDA seed bug found and fixed (device_pubkey is 33 bytes, Solana PDA seeds max 32 — now using SHA256 hash). Scope limited to v2; legacy v1 deferred to Task #14. |
| 2026-04-13 | 3 | Phase 3 complete locally. Coordinator wired for treasury/reserve config, node_vault PDA derivation, all 4 new instruction builders, and atomic `finalize_v2 + claim_rewards_v2` bundling. 109 coordinator tests (7 new), 0 regressions. End-to-end Anchor-side economics now work: when a v2 round finalizes, one TX distributes 70/20/10 into NodeVaults + treasury + reserve. |
| 2026-04-13 | 4 | Phase 4 firmware code complete. New `dice_crypto_sign_payout_binding()` helper, new `payout_binding.c`/`.h` module, CBOR message type 5, app_main wiring, factory_reset clear-on-wipe. Also: switched on-chain verify_binding_signature from keccak256 → SHA-256 to match firmware's existing SHA-256 signer. All 117 Rust tests still pass (61 dice + 56 dice-vrf). Firmware build + deploy pending user's ESP-IDF and Solana CLI toolchain. Deploy runbook documented. |
| 2026-04-14 | deploy | Program upgraded on devnet: sig `2JBQbh89vAv5MNd7Zyxdfn5M2RL54ZhAAcKPmBXmZfr3CNecqyaudHmv6u849aNGtgT5sB5HwVYGFfPPAGfGC4JW`. Program data 498560 → 550912 bytes. Build via WSL anchor build --no-idl after patching missing v1.52 platform-tools by symlinking v1.54. |
| 2026-04-14 | smoke | `smoke_v7` binary (tests/harness/smoke_v7/) exercised register_node_vault end-to-end: TX `23m5epsGS52H1FuP7wf731U3Nh1oFC4ezMZbTkCh5L5uBHwZqvFP22aoaw4fVtq4xjbuAcjc5hdUDMMdaHpNzBsy` created a real NodeVault PDA bound to a test wallet. Proves on-chain path works with real secp256k1 sigs. |
| 2026-04-14 | firmware | v7 firmware built via firmware/build_firmware.bat (ESP-IDF v5.2.6) and flashed to COM7 via firmware/flash_firmware.bat. Boots cleanly — all new C files (crypto.c binding signer, payout_binding.c, factory_reset.c hook, dice_protocol.c new message type 5) link and run without crashing. Device enters captive portal on fresh NVS. |
| 2026-04-14 | E2E bring-up | Real-mode coordinator started (PID 17224) listening on `0.0.0.0:8443` with `certs/coordinator.crt` + `certs/ca.crt`. Neon postgres connected via DATABASE_URL in .env (git-ignored). FIRST NVS flash attempt used same secp256k1 key for both DICE identity AND mTLS client cert — rustls 0.21 rejected it with `InvalidCertificate(BadSignature)`. Fixed by splitting into two keys: secp256k1 for priv_key_der (DICE identity), secp256r1 reused from certs/device.* for mTLS. Second NVS flash complete. Device rebooted, connected to Airfiber, mTLS handshake **SUCCESS**, WebSocket upgraded, PayoutBindingRequest sent. |
| 2026-04-14 | E2E success | **REAL HARDWARE BINDING CONFIRMED ON DEVNET.** TX `5PzuCRN9f2PVBC21amnHD3yws39iuWtuttSqT1kbv6Axa9fWmghNcqrnsvKZnDMVmXNH1m9Q5M1FuetP3c1PPUfL` finalized. NodeVault PDA `8giSVw9zJzUV8ViJQyYr1ELtuz6q1KpaQ2bddwQAQvdM` state decoded: device_pubkey=`03ad3382d1b1155d35d6fd3ad8a27d98592cd7538c0a800a3405f51185bc78ef28` (matches real ESP32-S3 on COM7), payout_wallet=`4n9V4tTKNAJjvhJ4AeqpyEUMgLNMNsAGrmkB4c9oRAs6`, status=Bound, total_earned=0 (no VRF rounds yet). Binding_slot=455357558, CU consumed=71202/200000. Every layer now proven end-to-end: ESP32-S3 firmware → mTLS WebSocket → coordinator → register_node_vault → Anchor program → NodeVault PDA. Task #17 COMPLETE. |
