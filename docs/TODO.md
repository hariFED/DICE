# DICE — Next Steps

> **Last updated:** 2026-03-27
> **Repo:** https://github.com/hariFED/DICE (private)
> **Status:** Deployed to Solana devnet, 24 tests passing, simulation working

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
- [ ] New `select_nodes` instruction in the smart contract
  - Reads `SlotHashes` sysvar for unpredictable seed
  - `seed = SHA-256(slot_hash || request_id || block_height)`
  - Deterministically selects N nodes from registered DeviceRegistry accounts
  - Writes selection to `RandomnessRequest.selected_nodes`
  - All DeviceRegistry PDAs passed as `remaining_accounts`
- [ ] Update coordinator to call `select_nodes` on-chain before dispatching jobs
- [ ] `submit_commit` already checks `selected_nodes` — no change needed there
- [ ] Add TypeScript test for `select_nodes`
- [ ] Redeploy to devnet

**Design validated:** SlotHashes sysvar is accessible on-chain, gives 512 recent slot hashes that no one can predict. Combined with existing ECDSA + commit-reveal protections, this makes the system robust even against a fully compromised coordinator.

---

## Priority 2 — Run Full Devnet Simulation

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

## Priority 3 — Remaining Code Tasks

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

## Priority 4 — Documentation & Developer Experience

- [ ] Quickstart guide: zero to working randomness in 10 minutes
- [ ] Coin-flip example program (full Anchor project with DICE integration)
- [ ] Copy-paste template for Anchor projects
- [ ] `dice test` / `dice status` / `dice fund` CLI tool

---

## Priority 5 — Infrastructure (when ready for mainnet)

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
