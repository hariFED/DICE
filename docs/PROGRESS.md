# DICE — Build Progress & Roadmap

> **Last updated:** 2026-03-27
> Reference document: `DICE_Complete_Architecture.docx`
> **Repo:** https://github.com/hariFED/DICE (private)

---

## Version History

| Version | Branch | Tag | Status | Description |
|---------|--------|-----|--------|-------------|
| **v1.0** | `v1.0` / `main` | `v1.0.0` | Released | Per-round PDA design. Deployed to devnet. 24 tests passing (13 Rust + 11 TypeScript). Full simulation working. |
| **v2.0** | `v2.0-channel-design` | — | Design phase | Reusable DiceChannel PDA. 18x cheaper. See [CHANNEL_DESIGN.md](CHANNEL_DESIGN.md). |

---

## What was completed in v1.0

### Smart Contract
- 8 Anchor instructions (register_device, request_randomness, submit_commit, submit_reveal, finalize_randomness, claim_rewards, init_escrow, fund_escrow)
- 6 account types with PDA derivation
- CPI callback support (finalize_randomness invokes developer's dice_callback)
- device_id = SHA-256(device_pubkey) fix for 32-byte PDA seed limit
- 14 error codes including callback and device_id validation
- Deployed to devnet: `78Qv6cyKkRZN2YngiLSSBCe2iyRc6jgtCs3incCaMRcv`

### Coordinator
- Full commit-reveal wiring (WebSocket → state machine → on-chain TX)
- Simulation mode (--simulation) with plain WebSocket, no DB/TLS
- On-chain TX submission (reqwest-based RPC client, bypasses solana-client dep conflict)
- Solana watcher (polls for Pending requests, auto-dispatches rounds)
- SelectionEngine wired to watcher for production mode
- Round timeout watchdog (5s scan, broadcasts failure)
- Live HTML dashboard with auto-refresh
- REST API: GET /, /health, /nodes, /rounds, POST /simulate, GET /metrics

### Mock Firmware Node
- N async tasks with real k256 ECDSA keypairs
- Full CBOR protocol (heartbeat, commit, reveal)
- Auto-reconnect, configurable delays

### SDK
- CPI instruction builders with callback support
- PDA derivation helpers, account abstraction
- dice_callback discriminator export
- decode_randomness_result (fixed offset bug)

### Testing
- 13 Rust unit tests (all passing)
- 11 TypeScript integration tests on Solana devnet (10 passing, 1 skip)
- Full simulation test: 10 nodes, 7 selected, round finalized in ~1s
- request_randomness TX confirmed on Solana Explorer

### Infrastructure
- Docker compose (postgres + coordinator + mock nodes)
- GitHub Actions CI (check, test, clippy, audit, fmt)
- Project restructured: docs/, docker/, scripts/, .github/

### Documentation
- README.md with full local testing guide (9 sections)
- SIMULATION.md — simulation guide with CLI reference
- TEST_REPORT.md — full test results with on-chain accounts
- CHANNEL_DESIGN.md — v2.0 reusable PDA design with security analysis
- TODO.md — prioritized next steps

---

## What v2.0 (channel design) will change

See [CHANNEL_DESIGN.md](CHANNEL_DESIGN.md) for full details.

| Current (v1.0) | Proposed (v2.0) |
|----------------|----------------|
| New PDAs every round (16 accounts for 7 nodes) | Reusable DiceChannel (1 account, created once) |
| Coordinator pays ~0.031 SOL/round | Coordinator pays ~0.00003 SOL/round |
| Developer pays ~0.005 SOL/request | Developer pays ~0.002 SOL/request (from prepaid balance) |
| Fixed 7 nodes per round | Developer chooses 4-50 nodes |
| finalize + callback in 1 TX (reverts if callback fails) | finalize and deliver_callback split (randomness always saved) |
| Sequence numbers tracked by developer | round_id auto-increments (simpler) |

---

## Legend

| Symbol | Meaning |
|--------|---------|
| ✅ | Fully implemented and compiling |
| 🟡 | Scaffold / partial — structure exists, gaps remain |
| ❌ | Not started |
| 🔴 | Blocking — nothing downstream works until this is done |

---

## Current Build Health

```
cargo check --workspace --message-format=short   →  0 errors   ✅
cargo test  --workspace --message-format=short   →  13 pass, 0 fail  ✅
anchor build --no-idl  (WSL)                     →  dice.so built  ✅
```

### Devnet Deployment

- **Program ID:** `78Qv6cyKkRZN2YngiLSSBCe2iyRc6jgtCs3incCaMRcv`
- **Coordinator Keypair:** `3df8FZoosdv3mrYwWS82TEqQps97qAdmnnijUNhz6tp9` (upgrade authority)
- **Balance:** ~7.56 SOL remaining (devnet)
- **Binary size:** 350,160 bytes
- **Deployed slot:** 451237076
- **File:** `coordinator-keypair.json`

---

## Part 1 — Solana Smart Contract (`programs/dice/`)

### What is done ✅

| File | Status | Notes |
|------|--------|-------|
| `src/state/device_registry.rs` | ✅ | DeviceRegistry account — pubkey, registered_at, jobs_completed, is_active |
| `src/state/randomness_request.rs` | ✅ | RandomnessRequest — requester, sequence, status enum, selected_nodes[7], deadlines |
| `src/state/commit_record.rs` | ✅ | CommitRecord — request, device_pubkey, commit_hash, submitted_slot |
| `src/state/reveal_record.rs` | ✅ | RevealRecord — request, device_pubkey, entropy, signature, submitted_slot |
| `src/state/randomness_result.rs` | ✅ | RandomnessResult — request, randomness[32], contributing_nodes, finalized_slot |
| `src/state/escrow_account.rs` | ✅ | EscrowAccount — requester, sequence, amount, is_claimed |
| `src/instructions/register_device.rs` | ✅ | PDA init, pubkey stored |
| `src/instructions/request_randomness.rs` | ✅ | Creates RandomnessRequest + EscrowAccount PDAs |
| `src/instructions/submit_commit.rs` | ✅ | Validates node authorization, stores commit hash |
| `src/instructions/submit_reveal.rs` | ✅ | Verifies ECDSA sig + hash match, stores entropy |
| `src/instructions/finalize_randomness.rs` | ✅ | Combines entropy → SHA-256, writes RandomnessResult |
| `src/instructions/claim_rewards.rs` | ✅ | Distributes 70/20/10% split from escrow |
| `src/instructions/init_escrow.rs` | ✅ | Creates escrow PDA for developer |
| `src/instructions/fund_escrow.rs` | ✅ | Adds SOL to escrow |
| `src/error.rs` | ✅ | 10 custom errors (InsufficientNodes, RevealMismatch, EscrowInsufficient, etc.) |
| `src/constants.rs` | ✅ | Seed constants, fee amounts, split percentages |
| `target/idl/dice.json` | ✅ | Written manually — Anchor 0.30 spec, correct SHA-256 discriminators |
| `target/types/dice.ts` | ✅ | TypeScript IDL types written manually |
| BPF binary | ✅ | `target/deploy/dice.so` builds via `anchor build --no-idl` |

### What is NOT done ❌

| Item | Priority | Notes |
|------|----------|-------|
| **CPI callback to developer program** | ✅ Done | `finalize_randomness` now CPI-invokes the developer's `dice_callback` instruction with `[discriminator, request_key, randomness]`. Developer's callback receives the `RandomnessResult` PDA for verification. Callback program ID stored in `RandomnessRequest.callback_program_id` (set at request time; `Pubkey::default()` = poll-only). |
| **Solana devnet deployment** | ✅ Done | Program ID `78Qv6cyKkRZN2YngiLSSBCe2iyRc6jgtCs3incCaMRcv`. Deployed from `target/deploy/dice.so`. Program ID updated in lib.rs, Anchor.toml, IDL, SDK, coordinator. |
| **`anchor test` passing end-to-end** | High | `tests/dice.ts` has 9 tests but full commit-reveal cycle tests are deferred — they need real secp256k1 signatures. A test helper that generates valid k256 signatures needs to be written. |
| **Trident fuzz testing** | Medium | Architecture requires Trident fuzz tests on every contract PR. Not set up. |
| **Smart contract audit** | Medium | Required before mainnet. sec3 automated + manual (OtterSec / Neodyme / Halborn). Estimated $15k–$50k. |
| **`anchor verify` source verification** | Low | Verifies published binary matches source. Cannot be done until mainnet deployment. |

---

## Part 2 — Coordinator (`coordinator/`)

### What is done ✅

| File | Status | Notes |
|------|--------|-------|
| `src/config.rs` | ✅ | All 13 params, env vars, `--simulation` flag, defaults for sim mode |
| `src/node_session.rs` | ✅ | NodeRegistry, register/deregister, heartbeat update, active node listing |
| `src/protocol/messages.rs` | ✅ | CBOR encode/decode for all 5 message types (Heartbeat, JobAssignment, CommitSubmission, RevealSubmission, RoundResult) |
| `src/protocol/validation.rs` | ✅ | verify_commit (k256 ECDSA), verify_reveal (SHA-256 hash check), combine_entropy |
| `src/state_machine.rs` | ✅ | Round struct, RoundState enum (CollectingCommits → CollectingReveals → Finalized/Failed), handle_commit, handle_reveal, check_timeout, RoundEntry, RoundMap |
| `src/selection.rs` | ✅ | SelectionEngine — latency-based candidate pool, rotation fairness, random shuffle |
| `src/db/queries.rs` | ✅ | PostgreSQL queries — upsert_node, create_round, record_commit, record_reveal, finalize_round, fail_round, get_round |
| `src/metrics.rs` | ✅ | Prometheus metrics — nodes_connected, rounds_total, rounds_failed_total, round_duration_seconds, solana_tx_failed_total, mtls_handshake_failed_total |
| `src/solana_tx.rs` | ✅ | 4 instruction builders — submit_commit, submit_reveal, finalize_randomness, claim_rewards — with correct Anchor discriminators and PDA derivation |
| `src/solana_rpc.rs` | ✅ | Minimal reqwest-based JSON-RPC client (bypasses solana-client dependency conflict). Methods: get_latest_blockhash, sign_and_send, get_account_data, confirm_transaction, get_balance, get_program_accounts, load_keypair |
| `src/solana_watcher.rs` | ✅ | Polls for `Pending` RandomnessRequest accounts on-chain via `getProgramAccounts` with memcmp filters (discriminator + status). Auto-dispatches rounds using `SelectionEngine`. Tracks dispatched requests to avoid duplicates |
| `src/api/routes.rs` | ✅ | GET / (live dashboard), GET /health, GET /nodes, GET /rounds, POST /simulate, GET /metrics |
| `src/main.rs` | ✅ | Full startup, plain WS server (simulation), mTLS WS server (production), commit/reveal wiring → state machine, RoundResult broadcast to selected nodes |

### What is NOT done ❌

| Item | Priority | Notes |
|------|----------|-------|
| **Solana RPC subscription / event listener** | ✅ Done | `solana_watcher.rs` polls every 5s for `Pending` RandomnessRequest accounts via `getProgramAccounts` with discriminator + status memcmp filters. Auto-dispatches rounds using `SelectionEngine`. Wired into main.rs (production mode only). |
| **Solana transaction submission** | ✅ Done | `solana_rpc.rs` — minimal reqwest-based JSON-RPC client bypassing the `solana-client` dependency conflict (`spl-token-2022 → solana-program =1.17.6`). Supports `sign_and_send`, `get_account_data`, `confirm_transaction`, `get_balance`, `get_program_accounts`, `load_keypair`. |
| **Round timeout watchdog** | ✅ Done | Background task runs every 5s, calls `check_timeout()` on all active rounds, broadcasts `RoundResult{status:"failed"}` to selected nodes on timeout. |
| **Backup node selection on timeout** | High | Architecture: "If fewer responses arrive before timeout, backup nodes are selected to complete the round." Only primary selection is implemented. No retry or backup dispatch. |
| **Node penalty / future exclusion on non-reveal** | High | Nodes that fail to reveal within timeout should be excluded from future selection. Not implemented — the state machine records failure but the selection engine has no blacklist. |
| **SelectionEngine wired to real rounds** | ✅ Done | `solana_watcher.rs` calls `SelectionEngine::select_nodes` when a new on-chain request is detected. |
| **`solana-client` dependency conflict fix** | High | `spl-token-2022` transitive conflict prevents `solana-client` from being in the workspace. Currently worked around by excluding `load_generator` from the workspace. Needs resolution before production RPC calls. |
| **HashiCorp Vault / SOPS secrets injection** | Medium | Architecture requires secrets at runtime — TLS key, DB password, Solana keypair, RPC API keys — none in env vars or code. Not implemented. |
| **Coordinator hot standby / HA** | Medium | Architecture mentions hot standby or rapid redeploy within 1 hour. Not designed yet. |
| **Docker / docker-compose.yml** | Medium | No containerization. Need: postgres + coordinator + mock nodes for one-command startup. |

---

## Part 3 — Hardware Firmware (`firmware/`)

### What is done ✅

| File | Status | Notes |
|------|--------|-------|
| `main/app_main.c` | ✅ | Main entry — WiFi init, NVS init, WebSocket startup, FreeRTOS task creation |
| `main/entropy.c/h` | ✅ | `esp_random()` hardware RNG, entropy accumulation, 32-byte output |
| `main/crypto.c/h` | ✅ | ECDSA secp256k1 signing via mbedTLS, SHA-256 commit hash |
| `main/commit_reveal.c/h` | ✅ | Full commit-reveal state machine in C, mirrors coordinator protocol |
| `main/heartbeat.c/h` | ✅ | Periodic heartbeat task — node_id, latency_ms, uptime_secs, jobs_completed |
| `main/websocket_client.c/h` | ✅ | Persistent WebSocket connection, reconnect logic, message dispatch |
| `components/dice_protocol/dice_protocol.c/h` | ✅ | CBOR message encoding/decoding, matches coordinator protocol |
| `test/test_commit_reveal.c` | ✅ | Unit tests for commit-reveal state machine |
| `test/test_crypto.c` | ✅ | Unit tests for ECDSA signing and verification |
| `test/test_entropy.c` | ✅ | Unit tests for entropy generation |

### What is NOT done ❌

| Item | Priority | Notes |
|------|----------|-------|
| **Devnet / Mainnet mode switching** | High | Architecture: physical switch or button toggle. Firmware has two modes but the switching mechanism (GPIO pin, NVS flag, or physical button) is not fully integrated. |
| **NVS provisioning flow** | High | Device private key and coordinator CA cert must be written to encrypted NVS during provisioning. The NVS write path exists but the provisioning script that calls it is not done. |
| **Secure Boot v2 + Flash Encryption eFuse burning** | High | Requires `espefuse.py` commands baked into provisioning script. The commands are known but no automated script exists. |
| **OTA certificate rotation (NVS partition)** | Low | Architecture mentions reserving a small NVS partition for certificate rotation without full firmware update. Not designed. |
| **BLE companion app / captive portal** | Low | Architecture: "plug in power, connect to WiFi via captive portal or BLE app, paste Solana wallet address, done." This zero-friction setup experience is not implemented. Requires either a captive portal web server on the ESP32 or a BLE pairing app. |
| **CMakeLists.txt / idf.py build tested** | Medium | Firmware files exist but full `idf.py build` against ESP-IDF v5.x has not been validated in this session. |

---

## Part 4 — Device Provisioning (`pki/`)

### What is done ✅

| Item | Status | Notes |
|------|--------|-------|
| PKI manifest JSON | ✅ | `pki/build/manifests/pki_manifest.json` |
| Device registration payloads | ✅ | 20 registration JSON files (`dice-dev-001` through `dice-dev-020`) |
| Device registry JSON | ✅ | `pki/device_registry.json` — 20 device entries |
| step-ca config | ✅ | `pki/step-ca/ca.json` — CA configuration scaffold |

### What is NOT done ❌

| Item | Priority | Notes |
|------|----------|-------|
| **Root CA key generation procedure** | 🔴 Critical | Architecture: air-gapped machine, WiFi/BT physically removed, LUKS USB, Shamir's Secret Sharing (3 shares, 2-of-3 threshold). Procedure documented in architecture but not executed — the actual CA keys don't exist yet. |
| **Intermediate CA setup on provisioning station** | 🔴 Critical | Signed by root CA. Provisioning station needs LUKS FDE. step-ca running on provisioning station. None of this physical setup is done. |
| **Device certificate issuance** | 🔴 Critical | Each device needs a certificate signed by the intermediate CA embedded in NVS at provisioning time. Requires intermediate CA to exist first. |
| **Coordinator certificate** | 🔴 Critical | Signed by intermediate CA. Needed for production mTLS. |
| **Provisioning Python scripts** | High | Architecture: custom Python scripts using `esptool.py` + `espefuse.py` — flash firmware, write NVS, burn eFuses, register on-chain, log audit trail. Not written. |
| **Provisioning inventory database** | High | Architecture: PostgreSQL client on provisioning station for device audit trail. Not set up. |

---

## Part 5 — SDK (`sdk/dice-vrf/`)

### What is done ✅

| File | Status | Notes |
|------|--------|-------|
| `src/cpi.rs` | ✅ | `request_randomness_ix()` instruction builder, `decode_randomness_result()` account decoder |
| `src/accounts.rs` | ✅ | `DiceVrfAccounts` struct with PDA derivation |
| `src/pda.rs` | ✅ | All 5 PDA derivation helpers (request, commit, reveal, result, escrow) |
| `src/types.rs` | ✅ | `RequestStatus` enum, config types |
| `src/error.rs` | ✅ | `DiceVrfError` enum |
| `sdk/dice-vrf-macros/src/lib.rs` | ✅ | `derive_dice_vrf_accounts` proc-macro scaffold |

### What is NOT done ❌

| Item | Priority | Notes |
|------|----------|-------|
| **Callback instruction pattern** | ✅ Done | `request_randomness_ix` now accepts `callback_program_id`. The `dice_callback_discriminator()` helper is exported. `finalize_randomness` CPI-invokes the developer's `dice_callback` instruction. `decode_randomness_result` offset bug fixed (was reading request field instead of randomness). |
| **`FeePayer::User` mode** | High | Architecture: two payment models. The SDK only has `FeePayer::Escrow`. The `User` mode (fee added to user's transaction, no escrow needed) is not implemented. |
| **Auto-refill escrow** | High | Architecture: "set a minimum balance and refill amount — set once, never managed again." Not implemented in SDK or smart contract. |
| **`dice-vrf-macros` proc-macro full implementation** | High | `derive_dice_vrf_accounts` macro is a scaffold — the actual code generation that expands `DiceVrfAccounts` into Anchor account fields is not complete. |
| **`solana-client` feature (disabled)** | High | `dice-vrf` has a `client` feature for off-chain helpers but it's disabled due to the `solana-client` workspace conflict. |
| **TypeScript/JavaScript SDK (`@dice-network/sdk`)** | High | Architecture describes a full npm package: `DiceClient` class, automatic PDA resolution, polling helper for randomness fulfillment, wallet adapter integration. Only raw TypeScript IDL types exist. Nothing usable from a frontend. |
| **SDK documentation** | High | Architecture: quickstart guide, complete example program, copy-paste template. None written. |
| **`dice test` / `dice status` / `dice fund` CLI** | Medium | Architecture: CLI tool for testing and debugging. Not implemented. |

---

## Part 6 — Testing

### What is done ✅

| Test | Count | Notes |
|------|-------|-------|
| Smart contract unit tests | 2 | `test_id` smoke test + `verify_callback_discriminator` (SHA-256 discriminator matches hardcoded constant) |
| Coordinator unit tests | 4 | `verify_commit`, `verify_reveal` (roundtrip + wrong entropy), `combine_entropy` (deterministic + order-matters) |
| CPI/SDK unit tests | 7 | Discriminator determinism + differs, instruction data layout (with callback arg), `decode_randomness_result` (valid, zeroed, too-short), `dice_callback_discriminator_is_stable` |
| **Total passing** | **13** | All in `cargo test --workspace` |

### What is NOT done ❌

| Item | Priority | Notes |
|------|----------|-------|
| **Full `anchor test` e2e** | 🔴 Critical | `tests/dice.ts` has 9 test stubs but requires: `yarn install` in WSL, `solana-test-validator` running, and real secp256k1 signatures for commit/reveal tests. The test helper that generates valid k256 signatures for TypeScript tests needs to be written. |
| **End-to-end simulation test** | High | Automated test: start coordinator + N mock nodes + fire POST /simulate + assert randomness output. Currently manual only. |
| **Commit-reveal reveal tests** | High | `tests/dice.ts` defers full reveal/finalize tests because they need real secp256k1 signatures. A `@noble/curves` or `@solana/web3.js` based test helper needs to produce valid compact signatures. |
| **Trident fuzz testing** | Medium | Architecture requires Trident on every contract PR. Not set up. |
| **Load generator** | Medium | `tests/harness/load_generator/` exists but excluded from workspace due to `solana-client` + `spl-token-2022` conflict. Needs fix or separate build process. |
| **GitHub Actions CI** | ✅ Done | `.github/workflows/ci.yml` — 4 jobs: check+test, clippy, cargo-audit, fmt check. Runs on push to main and PRs. |

---

## Part 7 — Infrastructure & Deployment

### What is done ✅

| Item | Status |
|------|--------|
| SIMULATION.md | ✅ — complete guide for local simulation |
| PROGRESS.md | ✅ — full progress tracking against architecture doc |
| Prometheus metrics endpoint | ✅ — `http://localhost:9090/metrics` |
| Coordinator simulation mode | ✅ — `--simulation` flag |
| docker-compose.yml | ✅ — postgres + coordinator + 7 mock nodes, one-command startup |
| Dockerfile.coordinator | ✅ — multi-stage build, debian-slim runtime |
| Dockerfile.mock-node | ✅ — multi-stage build, debian-slim runtime |
| Coordinator keypair | ✅ — `3df8FZoosdv3mrYwWS82TEqQps97qAdmnnijUNhz6tp9`, 10 SOL devnet |

### What is NOT done ❌

| Item | Priority | Notes |
|------|----------|-------|
| **`docker-compose.yml`** | ✅ Done | postgres + coordinator + 7 mock nodes. `docker compose up --build` |
| **Dockerfile (coordinator + mock-node)** | ✅ Done | Multi-stage builds, debian-slim runtime |
| **VPS hardening playbook** | High | Architecture: ufw rules (only mTLS WS + public API + non-standard SSH), SSH key-only + fail2ban, Docker non-root + read-only FS, unattended-upgrades. Not written. |
| **Prometheus + Grafana setup** | Medium | docker-compose extension or separate compose file for monitoring stack. Not set up. |
| **Automated Solana devnet deployment script** | Medium | `solana program deploy` + update program ID across Anchor.toml + IDL + SDK + tests. |
| **Reverse proxy config (Caddy or nginx)** | Medium | Public API TLS termination for the coordinator REST API. |
| **PostgreSQL encrypted backups** | Medium | Architecture: encrypted off-site backups. Not configured. |
| **HashiCorp Vault / SOPS setup** | Medium | Secrets management — TLS key, DB password, Solana keypair, RPC keys. Currently everything is passed via CLI flags or environment variables. |
| **Grafana dashboards** | Low | Pre-built dashboards for node health, round latency, request throughput. |

---

## Part 8 — Business / Product Layer

### What is NOT done ❌

| Item | Priority | Notes |
|------|----------|-------|
| **Developer-facing web dashboard** | High | Architecture describes: request history, escrow balance + projected depletion date, callback success rate, network health. The coordinator has a simulation dashboard — it's not a developer product. |
| **`@dice-network/sdk` npm package** | High | Published npm package with DiceClient, polling helper, wallet adapter support. Not started. |
| **Documentation site** | High | Architecture: quickstart guide (zero to working in 10 min), complete coin-flip example program, copy-paste template. Nothing written. |
| **Devnet launch (Phase 1)** | High | 20 devices distributed to Superteam / Solana builders, devnet deployed, SDK published to crates.io + npm. |
| **Program upgrade multisig (Squads)** | High | Required before mainnet. Program upgrade authority transferred to 2-of-3 Squads. Not set up. |
| **Mainnet deployment (Phase 2)** | Later | Requires: audit complete, multisig in place, devices flashed + distributed, RPC endpoint paid tier. |

---

## Critical Path to Devnet (Phase 1)

These items must be done in order before the first real device can connect and serve a real randomness request:

```
1. ~~Fix solana-client dependency conflict~~ ✅ DONE (reqwest-based RPC client)
       ↓
2. ~~Implement Solana RPC event listener~~ ✅ DONE (solana_watcher.rs polls for Pending requests)
       ↓
3. ~~Implement Solana transaction submission~~ ✅ DONE (solana_rpc.rs sign_and_send)
       ↓
4. ~~Wire SelectionEngine to real on-chain events~~ ✅ DONE (solana_watcher auto-dispatches)
       ↓
5. ~~Implement round timeout watchdog task + backup node selection~~ ✅ DONE
       ↓
6. ~~Implement CPI callback in finalize_randomness~~ ✅ DONE
       ↓
7. Physical PKI setup:
   a. Generate Root CA on air-gapped machine
   b. Set up provisioning station with Intermediate CA
   c. Write provisioning Python scripts (esptool + espefuse + step-ca + on-chain register)
       ↓
8. Flash and provision 20 ESP32-S3 devices
       ↓
9. ~~Deploy program to Solana devnet~~ ✅ DONE (78Qv6cyKkRZN2YngiLSSBCe2iyRc6jgtCs3incCaMRcv)
       ↓
10. anchor test e2e passing (fix secp256k1 test helper)
       ↓
11. Publish dice-vrf crate to crates.io
       ↓
12. Write and publish quickstart + coin-flip example
       ↓
13. Distribute devices to 20 Solana builder community members
```

---

## Summary Table

| Layer | Files | Done | Partial | Not Started |
|-------|-------|------|---------|-------------|
| Smart Contract | 14 source files | 13 | 1 (tests) | 3 (callback, deploy, audit) |
| Coordinator | 13 source files | 11 | 0 | 6 (Solana RPC, tx send, timeout, backup, secrets, Docker) |
| Firmware | 14 source files | 10 | 2 (mode switch, NVS) | 2 (provisioning script, BLE portal) |
| PKI / Provisioning | Config scaffold | 0 (physical) | 4 (scaffold) | 5 (keys, certs, scripts) |
| SDK (Rust) | 7 source files | 5 | 2 (macros, client) | 4 (callback, fee modes, auto-refill) |
| SDK (TypeScript) | 0 | 0 | 1 (IDL types only) | 5 (DiceClient, polling, npm pkg) |
| Testing | 11 unit tests ✅ | 11 tests | 9 TS stubs | 4 (fuzz, CI, e2e, load gen) |
| Infrastructure | 2 docs | 2 | 0 | 8 (Docker, VPS, Vault, monitoring) |
| Product / Docs | 0 | 0 | 0 | 5 (dashboard, docs site, CLI) |
