# DICE — Build Progress & Roadmap

> **Last updated:** 2026-04-08 01:16 IST
> **Branch:** `v5-keeper-notary`
> **Repo:** https://github.com/hariFED/DICE (private)

---

## Version History

| Version | Branch | Status | Description |
|---------|--------|--------|-------------|
| **v1.0** | `v1.0` / `main` | Released | Per-round PDA design. 8 instructions. Devnet deployed. |
| **v2.0** | `v2.0-channel-design` | Merged into v3 | Reusable DiceChannel PDA. 13 new instructions. 18x cheaper. |
| **v3** | `v3` | Released | Full stack: firmware on real hardware, mTLS, PostgreSQL, queue system, 3 example dApps, 545+ VRF rounds tested on real ESP32-S3. |
| **v4** | `v4` | Released | Research + planning: keeper/notary deep research, expansion analysis. |
| **v5** | `v5-keeper-notary` | **Active** | Multi-service coordinator: Keeper automation + Notary timestamping added alongside VRF. Zero firmware changes. |

---

## v5 Achievements (2026-04-08)

### Keeper Automation Service
- Parallel `tokio::spawn` task — zero interaction with VRF state machine
- Trigger types: `interval` (recurring) and `once` (one-shot)
- Configurable evaluation interval (default 10s)
- Concurrent execution with configurable limit (default 5)
- Per-task tracking: executions, failures, success rate, last tx signature
- Ring buffer history (200 entries in-memory)
- Full CRUD + toggle API endpoints
- Dashboard section with live stats + execution table
- Prometheus metrics: `dice_keeper_executions_total`, `dice_keeper_failures_total`, `dice_keeper_execution_latency_seconds`, `dice_keeper_active_tasks`
- **10 unit tests passing**

### Notary Timestamping Service
- Multi-witness hardware attestation via existing commit-reveal pipeline
- Self-contained receipt format (JSON) — independently verifiable
- Piggybacks on VRF JobAssignment/CommitSubmission flow (zero firmware changes)
- Witness signature collection from connected ESP32 nodes
- API endpoints: submit, retrieve by ID, history
- Dashboard section with live attestation stats
- DB persistence (`notary_attestations` table)
- Prometheus metrics: `dice_notary_attestations_total`, `dice_notary_attestation_latency_seconds`
- **4 unit tests passing**

### Infrastructure Changes
- Config flags: `--keeper-enabled`, `--keeper-interval-secs`, `--keeper-max-concurrent`, `--notary-enabled`, `--notary-min-witnesses`
- DB schema: 3 new tables (`keeper_tasks`, `keeper_executions`, `notary_attestations`) + indexes
- Dashboard updated: v5 multi-service banner, keeper + notary sections
- Ready banner shows all active services

### Files Added/Modified (v5)
| File | Action | Lines |
|------|--------|-------|
| `coordinator/src/keeper.rs` | **NEW** | ~500 |
| `coordinator/src/notary.rs` | **NEW** | ~400 |
| `coordinator/src/config.rs` | Modified | +24 |
| `coordinator/src/main.rs` | Modified | +62 |
| `coordinator/src/api/routes.rs` | Modified | +445 |
| `coordinator/src/metrics.rs` | Modified | +69 |
| `coordinator/src/db/schema.sql` | Modified | +50 |
| `coordinator/src/db/queries.rs` | Modified | +162 |
| `coordinator/src/state_machine.rs` | Modified | +19 (commit_data accessor) |
| **Total new code** | | **~2,019 lines** |

### Files NOT touched (critical)
- `firmware/*` — zero firmware changes
- `coordinator/src/state_machine.rs` — VRF state machine core untouched
- `coordinator/src/protocol/messages.rs` — CBOR wire protocol untouched
- `coordinator/src/queue.rs` — VRF request queue untouched
- `programs/dice/src/*` — VRF smart contract untouched

---

## v3 Achievements (Previous)

### First Real Hardware VRF
- **545+ VRF rounds** on real ESP32-S3-N16R8 hardware
- **0 device crashes**, **0 coordinator crashes**
- **Avg round latency:** 1.7s (sequential), p50=1.3s
- **Device pubkey:** `025e62666100d9ee1973a02032dbe41f3e5d7b3e54bb11e9ba9cc839b43c35a01d`
- **Device MAC:** `1c:db:d4:46:c8:b4`

### What Was Built & Tested

| Component | Status | Evidence |
|-----------|--------|----------|
| ESP-IDF firmware compiled | Done | ESP-IDF v5.2.6, target esp32s3 |
| Firmware flashed to real ESP32-S3 | Done | COM4, 1013KB binary |
| Captive portal (WiFi AP + HTTP setup page) | Done | DICE-C8B4, 192.168.4.1 |
| LED status indicators (WS2812 GPIO48) | Done | Blue-Yellow-Green transitions |
| First-boot detection + auto-provisioning flow | Done | NVS check - portal or normal boot |
| Hardware entropy self-test | Done | 10 SHA-256 samples, uniqueness verified |
| secp256k1 key loading from NVS | Done | 135-byte DER, mbedTLS ECDSA |
| WiFi station connection (WPA2-PSK) | Done | Connected at RSSI -45 to -50 dBm |
| WebSocket client (plain ws:// and wss:// mTLS) | Done | Auto-detect from URI scheme |
| Heartbeat (25s interval) | Done | Timer stack fixed at 4096 bytes |
| CBOR protocol bridge (firmware - coordinator) | Done | Integer-key maps - array envelopes |
| Commit-reveal over real WebSocket | Done | 545+ rounds, all verified |
| Low-S ECDSA signature normalization | Done | mbedTLS high-S - k256 low-S |
| 16-slot firmware job queue | Done | Replaced single-slot, handles burst |
| Coordinator request queue | Done | 30/30 burst test, FIFO drain |
| Round history for dashboard | Done | Completed rounds persist in memory |
| mTLS (mutual TLS) | Done | CA - coordinator cert + device cert |
| PostgreSQL (Neon cloud) | Done | Schema auto-migrated, rounds persisted |
| Reveal signal broadcast | Done | Coordinator - device after all commits |
| 3 example dApps (CPI callback) | Done | Dice Roll, Lucky Wheel, Prediction Market |
| Dev provisioning tool | Done | Python: keygen + NVS gen + flash |

---

## Current Build Health (v5)

```
Branch:                              v5-keeper-notary
cargo check --workspace              0 errors
cargo test  --bin dice-coordinator   113 tests, 0 fail
Last test run:                       2026-04-08 01:16 IST
anchor build --no-idl (WSL)          5 .so files built
ESP-IDF build (v5.2.6, esp32s3)      dice_firmware.bin (1013KB)
```

---

## Devnet Deployment

| Program | ID | Status |
|---------|-----|--------|
| **DICE VRF** | `78Qv6cyKkRZN2YngiLSSBCe2iyRc6jgtCs3incCaMRcv` | Deployed + upgraded |
| **Dice Roll** | `CLpaMPxyu5Up4fuZb1JiY2uzj4s4iYVg9RfQHNFRuzAj` | Deployed |
| **Lucky Wheel** | `FzUuegZpKT4BHhzms1eJX7L2f6r3NTxMRexs8uqxtnbf` | Deployed |
| **Prediction Market** | `EHf5YLG2p7Wca9nUqJXRB6yATZidrBzJKM4Qj4k1EUvc` | Deployed |

- **Coordinator:** `3df8FZoosdv3mrYwWS82TEqQps97qAdmnnijUNhz6tp9`
- **Balance:** ~3.25 SOL remaining

---

## API Endpoints (v5)

### VRF (existing)
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/` | Dashboard (HTML) |
| GET | `/health` | Health check |
| GET | `/nodes` | Connected nodes |
| GET | `/rounds` | Active + completed rounds |
| GET | `/rounds/:id` | Single round by UUID |
| GET | `/queue` | Queue status |
| POST | `/simulate` | Trigger VRF round |
| GET | `/metrics` | Prometheus metrics |
| GET | `/api/v1/stats` | Network stats |

### Keeper (new in v5)
| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/keeper/tasks` | Register a new keeper task |
| GET | `/keeper/tasks` | List all tasks with stats |
| DELETE | `/keeper/tasks/:id` | Remove a task |
| POST | `/keeper/tasks/:id/toggle` | Enable/disable a task |
| GET | `/keeper/history` | Recent execution log |
| GET | `/keeper/stats` | Aggregate stats |

### Notary (new in v5)
| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/notarize` | Submit hash for attestation |
| GET | `/notarize/:id` | Fetch receipt by ID |
| GET | `/notarize/history` | Recent attestations |

---

## Part 1 — Smart Contract (`programs/dice/`)

### Status: Complete (21 instructions)

**v1.0 instructions (8):** register_device, request_randomness, submit_commit, submit_reveal, finalize_randomness, claim_rewards, init_escrow, fund_escrow

**v2.0 channel instructions (13):** init_channel, fund_channel, request_randomness_v2, request_randomness_auto, submit_commit_v2, submit_reveal_v2, finalize_v2, deliver_callback, withdraw_balance, close_channel, fail_round, resize_channel, select_nodes

**Unit tests:** 31 passing (constants, channel sizing, finalization)

---

## Part 2 — Coordinator (`coordinator/`)

### Status: v5 Multi-Service

| Component | Status |
|-----------|--------|
| Config (20 params + keeper/notary flags) | Done |
| Node registry + heartbeat | Done |
| CBOR protocol (both formats) | Done |
| ECDSA verification (low-S normalization) | Done |
| State machine (commit - reveal - finalize) | Done |
| Request queue (burst handling, FIFO, 60s expiry) | Done |
| Round history (dashboard display) | Done |
| Reveal signal broadcast | Done |
| PostgreSQL persistence | Done |
| mTLS WebSocket server | Done |
| Solana TX submission | Done |
| Solana WS subscriber | Done |
| REST API + Dashboard | Done |
| Prometheus metrics (VRF + Keeper + Notary) | Done |
| Selection engine | Done |
| Round timeout watchdog | Done |
| **Keeper automation** | **Done (v5)** |
| **Notary timestamping** | **Done (v5)** |

**Unit tests:** 113 passing (state machine, validation, TX builders, VRF proofs, integration, keeper, notary)

---

## Part 3 — Firmware (`firmware/`)

### Status: Tested on real ESP32-S3 (UNCHANGED in v5)

| Component | Status |
|-----------|--------|
| app_main.c (boot sequence, WiFi, main loop) | Done |
| entropy.c (TRNG + ADC + timing, SHA-256 mix) | Done |
| crypto.c (secp256k1 ECDSA, key from NVS) | Done |
| commit_reveal.c (16-slot job queue) | Done |
| websocket_client.c (ws:// and wss:// mTLS) | Done |
| heartbeat.c (25s timer) | Done |
| captive_portal.c (WiFi AP + HTTP + DNS) | Done |
| led_status.c (WS2812 GPIO48) | Done |
| dice_protocol (CBOR encode/decode) | Done |

**Build:** ESP-IDF v5.2.6, 1013KB binary, 3% free in factory partition

---

## Part 4 — PKI & Provisioning

### Status: Working (dev mode)

| Component | Status |
|-----------|--------|
| CA certificate (secp256r1, 10yr) | Done |
| Coordinator server cert (CA-signed, SAN) | Done |
| Device client cert (CA-signed) | Done |
| Dev provisioning script | Done |
| mTLS tested end-to-end | Done |

---

## Part 5 — Example dApps

### Status: 3 programs deployed to devnet

| Program | ID | VRF Tested |
|---------|-----|-----------|
| Dice Roll (1-6) | `CLpaMPxyu5Up4fuZb1JiY2uzj4s4iYVg9RfQHNFRuzAj` | 10 rolls |
| Lucky Wheel (weighted) | `FzUuegZpKT4BHhzms1eJX7L2f6r3NTxMRexs8uqxtnbf` | 15 spins |
| Prediction Market | `EHf5YLG2p7Wca9nUqJXRB6yATZidrBzJKM4Qj4k1EUvc` | 3 markets |
| Coin Toss (existing) | `3oJL6bXFaVJhegSU2ah9y1zqGmbFZZu4peQwr9XmfUtn` | Unit tests |

---

## Part 6 — Research

| Report | Status |
|--------|--------|
| Web3 Mentions Report | Done (MD + HTML) |
| Expansion Research (8 opportunities) | Done (MD + HTML) |
| Expansion Critical Analysis | Done (MD) |
| VRF-DePIN Ecosystem Report | Done (MD + HTML) |
| **Keeper + Notary Deep Research** | **Done (v5, MD + HTML)** |

---

## Part 7 — SDK

### Status: Rust SDK complete

| Component | Status |
|-----------|--------|
| CPI builders (v1 + v2 channel) | Done |
| PDA derivation helpers | Done |
| Account abstraction | Done |
| Callback discriminator | Done |
| 34 unit tests | Done |

---

## Production Readiness Checklist

| Item | Status |
|------|--------|
| Hardware VRF on real ESP32-S3 | Done (545+ rounds) |
| mTLS authentication | Done |
| PostgreSQL persistence | Done |
| Smart contract on devnet | Done (4 programs) |
| Randomness quality verified | Done |
| Security attack testing | Done (13 attacks, 0 vulns) |
| Stress testing | Done (30/30 burst, 42/40 sequential) |
| Request queue (burst handling) | Done |
| Coordinator dashboard | Done |
| Prometheus metrics | Done |
| Example dApps with docs | Done |
| Device provisioning tool | Done |
| **Keeper automation** | **Done (v5)** |
| **Notary timestamping** | **Done (v5)** |
| VPS deployment | Next |
| Frontend for users | Next |
| TypeScript SDK | Next |
| External security audit | Before mainnet |
| Mainnet deployment | After audit |

---

## Next Steps (Priority Order)

1. **Test keeper on devnet** — Deploy demo counter program, verify real crank transactions
2. **Test notary with real hardware** — Connect ESP32, verify multi-witness attestation
3. **VPS deployment** — Docker/systemd on Linux VPS, domain + HTTPS
4. **Frontend** — Landing page + developer dashboard
5. **TypeScript SDK** — npm package for dApp integration
6. **Anchor integration tests** — Full on-chain test suite
7. **Multi-node testing** — 4-7 nodes per round
8. **Security audit** — External (OtterSec / Neodyme / Halborn)
9. **Mainnet deployment** — After audit passes
