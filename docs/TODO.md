# DICE — Next Steps

> **Last updated:** 2026-04-14
> **Branch:** `v7`
> **Repo:** https://github.com/hariFED/DICE (private)
> **Status:** v7 NodeVault + streaming VRF deployed on devnet. Real ESP32-S3 hardware binding confirmed end-to-end (TX `5PzuCRN9...`). 229 Rust tests passing, 0 regressions. Frontend ready for public ship. See `docs/v7-universal-payout.md` for the v7 architecture.

---

## v7 — Shipped (2026-04-14)

- [x] NodeVault universal payout primitive — `register_node_vault`, `rotate_payout_wallet`, `withdraw_from_vault`, `claim_rewards_v2`
- [x] Streaming VRF — `init_feed`, `publish_feed_value`, `close_feed` + SDK subscriber example
- [x] Firmware binding flow — hardware-signed `PayoutBindingRequest` over mTLS WebSocket
- [x] Coordinator CORS + `/api/v1/stats` endpoint for public frontend consumption
- [x] Treasury + reserve config via `DICE_TREASURY` / `DICE_RESERVE` env vars
- [x] v7 program upgrade deployed to devnet (TX `2JBQbh89...`, 550,912 bytes)
- [x] Real ESP32-S3 end-to-end binding TX landed on devnet (`5PzuCRN9...`)
- [x] Frontend committed to git with deploy README + `NEXT_PUBLIC_API_URL` env

## v7 — Known deferred items (tracked as tasks)

- [ ] **Task #9**: Ship minimal TypeScript SDK — biggest integrator adoption blocker. Non-trivial scope (wrap Anchor IDL with `requestRandomness()` / `awaitResult()` helpers).
- [ ] **Task #13**: Fix latent secp256k1 recovery-ID bug in `submit_reveal.rs` (the `.or_else` chain that can silently accept valid-but-wrong pubkeys — fixed in `register_node_vault::verify_binding_signature`, not yet backported).
- [ ] **Task #14**: Fix v1 `claim_rewards` double-payment and `is_claimed` bugs. Legacy path. Not v7-critical.

---

---

## What's Done (completed this session)

- [x] Solana smart contract — 8 instructions, CPI callback, device_id PDA fix
- [x] Deployed to devnet: `78Qv6cyKkRZN2YngiLSSBCe2iyRc6jgtCs3incCaMRcv`
- [x] Coordinator — full commit-reveal wiring, on-chain TX submission, Solana watcher
- [x] Mock firmware nodes — 10 simulated ESP32 devices with real ECDSA
- [x] Live dashboard at localhost:8080
- [x] 13 Rust tests + 11 TypeScript tests (devnet) — all passing
- [x] Docker compose, CI pipeline, documentation
- [x] Pushed to GitHub: hariFED/DICE

---

## Priority 1 — On-Chain Verifiable Node Selection

**Why:** The coordinator currently picks which nodes participate. A compromised coordinator could select colluding nodes. Moving selection on-chain makes it trustless.

**What to build:**
- [x] New `select_nodes` instruction in the smart contract
  - Reads `SlotHashes` sysvar for unpredictable seed
  - `seed = SHA-256(slot_hash || channel_key || round_id || block_height)`
  - Deterministically selects N nodes from registered DeviceRegistry accounts via Fisher-Yates shuffle
  - Writes selection to `DiceChannel.device_ids` and `DiceChannel.device_pubkeys`
  - All DeviceRegistry PDAs passed as `remaining_accounts`
- [ ] Update coordinator to call `select_nodes` on-chain before dispatching jobs
- [x] Coordinator instruction builder for `select_nodes` (`build_select_nodes_ix`)
- [ ] Add TypeScript test for `select_nodes` (needs local validator with SlotHashes)
- [ ] Redeploy to devnet

**Design validated:** SlotHashes sysvar is accessible on-chain, gives 512 recent slot hashes that no one can predict. Combined with existing ECDSA + commit-reveal protections, this makes the system robust even against a fully compromised coordinator.

---

## Priority 2 — Define the Developer Integration Flow (How VRF Actually Works)

**Why:** Need to be crystal clear on how a developer (e.g., lottery app) actually uses DICE to pick a winner.

**Key question:** Does the developer send player data to DICE and DICE picks a winner? Or does DICE just give a random number and the developer uses it?

**Answer (how VRF works everywhere — Chainlink, Switchboard, DICE):**

DICE does NOT know about your players, your game, or your logic. It only gives you a **verifiable random number** (32 bytes). Your program uses that number to make the decision.

**Lottery example flow:**
```
1. Your lottery program has 100 players stored on-chain
2. When it's time to pick a winner, your program calls:
       dice::request_randomness(sequence=1)
3. DICE nodes do the commit-reveal protocol (takes ~2-5 seconds)
4. DICE calls back your program's dice_callback with:
       randomness = 0x8f3a91c0...  (32 random bytes)
5. YOUR program picks the winner:
       winner_index = randomness % 100   // 0-99
       winner = players[winner_index]
6. Done — the winner is provably random and verifiable on-chain
```

**What DICE gives you:** A single `[u8; 32]` — 32 bytes of verifiable randomness. That's it.

**What YOUR program does with it:** Whatever you want:
- Lottery: `winner = players[randomness % player_count]`
- NFT traits: `rarity = randomness[0] % 100` (0-99 rarity score)
- Card game: `card = deck[randomness % deck_size]`
- Dice roll: `roll = (randomness % 6) + 1`
- Coin flip: `heads = randomness[0] & 1 == 0`
- Shuffle: use randomness as seed for Fisher-Yates shuffle

**Why this is better than the developer picking:**
- The random number is generated by 4-7 independent hardware devices
- Nobody (not even DICE) can predict or manipulate it
- The entire process is verifiable on-chain — anyone can audit
- The developer's program is deterministic given the randomness — no cheating possible

**TODO items:**
- [ ] Build a complete lottery example program showing this flow end-to-end
- [ ] Build a coin-flip example (simplest possible integration)
- [ ] Document the callback pattern clearly in SDK docs
- [ ] Add a "How VRF Works" section to README for non-technical readers

---

**Why:** See real transactions flowing on Solana Explorer end-to-end.

**What to do:**
- [ ] Ensure coordinator keypair has SOL: `solana airdrop 5 3df8FZoosdv3mrYwWS82TEqQps97qAdmnnijUNhz6tp9 --url devnet`
- [ ] Run simulation with devnet:
  ```bash
  # Terminal 1
  cargo run --bin dice-coordinator -- --simulation
  # Terminal 2
  cargo run --bin mock-firmware-node -- --count 7
  # Terminal 3
  curl -X POST http://localhost:8080/simulate
  ```
- [ ] Check the `explorer` link in the response — verify transactions on Solana Explorer
- [ ] Screenshot the dashboard for docs/demo

---

## Priority 3 — Hardware Firmware & Onboarding (ESP32-S3-N16R8) 🔴 HIGH

**Target board:** ESP32-S3-N16R8 (16MB Flash, 8MB PSRAM)

**Goal:** Plug-and-play experience. User plugs in device, connects to its WiFi, opens a browser page, enters WiFi + wallet address, done — device starts earning.

### Captive Portal Setup Flow
- [ ] WiFi AP mode — device broadcasts `DICE-XXXX` hotspot (last 4 hex of device ID)
- [ ] HTTP server on `192.168.4.1` — serves setup page when user connects
- [ ] Setup web page (embedded HTML/CSS/JS in firmware):
  - WiFi network name + password input
  - Solana wallet address input (paste)
  - Device ID display (read-only)
  - Firmware version display
  - "Save & Connect" button
- [ ] NVS encrypted storage — save WiFi creds + wallet address + device keypair
- [ ] WiFi station mode — after setup, connect to user's router automatically
- [ ] Auto-reconnect — if WiFi drops, retry with backoff; if fails 5x, revert to AP mode for reconfiguration
- [ ] DNS redirect — redirect all DNS queries to `192.168.4.1` so captive portal auto-opens on any browser

### LED Status Indicators
- [ ] 🔵 Blue = Setup mode (captive portal active, waiting for config)
- [ ] 🟡 Yellow = Connecting to WiFi / coordinator
- [ ] 🟢 Green = Online, connected to coordinator, waiting for jobs
- [ ] 💚 Green blink = Actively participating in a commit-reveal round
- [ ] 🔴 Red = Error (no WiFi, coordinator unreachable, key missing)
- [ ] GPIO pin assignment for N16R8 onboard RGB LED (GPIO48 or Neopixel)

### Commit-Reveal Protocol (firmware)
- [ ] WebSocket client — connect to coordinator via WSS (mTLS in production)
- [ ] CBOR message encode/decode — heartbeat, job assignment, commit, reveal, round result
- [ ] Hardware RNG entropy — `esp_random()` accumulator, 32 bytes per round
- [ ] ECDSA secp256k1 signing — sign commits and entropy with device keypair (mbedTLS)
- [ ] SHA-256 commit hash — `commit_hash = SHA-256(entropy)`
- [ ] Heartbeat task — periodic heartbeat with node_id, latency, uptime, jobs_completed

### Factory Provisioning
- [ ] Provisioning Python script (`scripts/provision_device.py`):
  - Flash firmware via `esptool.py`
  - Generate device ECDSA keypair (secp256k1)
  - Write keypair + coordinator CA cert to encrypted NVS partition
  - Burn Secure Boot v2 eFuses via `espefuse.py`
  - Burn Flash Encryption eFuses
  - Register device on-chain (`register_device` instruction)
  - Log device ID + wallet + serial to provisioning database
- [ ] Provisioning station setup guide (LUKS FDE, step-ca, air-gapped root CA)
- [ ] Batch provisioning — flash 20 devices with unique keys in sequence
- [ ] Device manifest JSON — track all provisioned devices (ID, pubkey, cert hash, flash date)

### Build & Test
- [ ] ESP-IDF v5.x CMakeLists.txt — validate `idf.py build` compiles clean
- [ ] Unit tests for commit-reveal state machine (C, runs on host)
- [ ] Unit tests for ECDSA signing (C, runs on host)
- [ ] Integration test — device connects to coordinator simulation, completes a round
- [ ] OTA update mechanism — firmware update without re-provisioning keys

---

## Priority 4 — Remaining Code Tasks

### Smart Contract
- [ ] Add `select_nodes` instruction (Priority 1 above)
- [ ] Trident fuzz testing setup
- [ ] Full reveal/finalize TypeScript tests (need real secp256k1 sigs in TS — use `@noble/curves`)

### Coordinator
- [ ] Backup node selection on round timeout (retry with different nodes)
- [ ] Node blacklist for non-revealers (exclude from future selection)
- [ ] Wire `claim_rewards` TX submission after finalization

### SDK
- [ ] `FeePayer::User` mode (fee added to user's TX instead of escrow)
- [ ] Auto-refill escrow when balance drops below threshold
- [ ] Full `dice-vrf-macros` proc-macro implementation
- [ ] Publish `dice-vrf` crate to crates.io (when ready)

### TypeScript SDK
- [ ] Create `@dice-network/sdk` npm package
- [ ] `DiceClient` class with PDA resolution + polling helper
- [ ] Wallet adapter integration

---

## Priority 5 — Documentation & Developer Experience

- [ ] Quickstart guide: zero to working randomness in 10 minutes
- [ ] Coin-flip example program (full Anchor project with DICE integration)
- [ ] Copy-paste template for Anchor projects
- [ ] `dice test` / `dice status` / `dice fund` CLI tool

---

## Priority 6 — Future Enhancements (post-launch)

### Payment
- [ ] `FeePayer::User` mode — 0.002 SOL fee added to end user's TX instead of developer's channel balance. Zero developer overhead, user pays per request.
- [ ] Auto-refill channel when balance drops below threshold

### Performance
- [ ] Geyser plugin detector — replace WebSocket `logsSubscribe` with Geyser gRPC stream for ~100ms detection latency (vs ~500ms WebSocket). Pluggable via `RequestDetector` trait so it's a config change, not a rewrite.
- [ ] Geyser-as-a-service integration (Helius, Triton) for teams without self-hosted validators

---

## Priority 7 — Infrastructure (when ready for mainnet)

- [ ] Physical PKI: root CA (air-gapped), intermediate CA, device certs
- [ ] Provisioning scripts: `esptool.py` + `espefuse.py` + `step-ca`
- [ ] Flash 20 ESP32-S3 devices
- [ ] VPS hardening (firewall, SSH, Docker non-root)
- [ ] HashiCorp Vault or SOPS for secrets management
- [ ] Squads multisig for program upgrade authority (2-of-3)
- [ ] Smart contract audit (sec3 + OtterSec/Neodyme/Halborn)

---

## Key Addresses

| What | Address |
|------|---------|
| Program ID | `78Qv6cyKkRZN2YngiLSSBCe2iyRc6jgtCs3incCaMRcv` |
| Coordinator Wallet | `3df8FZoosdv3mrYwWS82TEqQps97qAdmnnijUNhz6tp9` |
| Program Data PDA | `DGUpEXGc2C8KCUVtSBBTxxhkWHR3DfGvPT1F4ExA6GvC` |

---

## Quick Resume Commands

```bash
# Check everything still compiles
cargo check --workspace --message-format=short

# Run Rust tests
cargo test --workspace --message-format=short

# Run TypeScript tests (needs SOL on devnet)
ANCHOR_PROVIDER_URL=https://api.devnet.solana.com ANCHOR_WALLET=./coordinator-keypair.json npx ts-mocha -p ./tsconfig.json -t 1000000 tests/dice.ts

# Run simulation
cargo run --bin dice-coordinator -- --simulation
cargo run --bin mock-firmware-node -- --count 10
# Then: http://localhost:8080 → click POST /simulate

# Build BPF binary (WSL)
anchor build --no-idl

# Deploy to devnet (WSL)
solana program deploy target/deploy/dice.so --url devnet --keypair coordinator-keypair.json --program-id 78Qv6cyKkRZN2YngiLSSBCe2iyRc6jgtCs3incCaMRcv
```
