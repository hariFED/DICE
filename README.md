# DICE Network

**Distributed Infrastructure for Cryptographic Entropy**

Hardware-backed verifiable randomness oracle on Solana. ESP32-S3 nodes generate entropy, sign with secp256k1, and participate in a commit–reveal protocol. Developers call one instruction, pay 0.002 SOL, and get a 32-byte random value with an on-chain audit trail.

- **Current branch:** `v7.7`
- **Live frontend:** https://dice-ten-ashen.vercel.app
- **Pre-order:** /preorder (Starter $89 · Pro $249 · Rack $799)

See [INDEX.md](INDEX.md) for the catalog of every top-level file and folder.

---

## Architecture

```
Applications (smart contracts that need randomness)
        |
Solana Smart Contract (commit-reveal, on-chain node selection, CPI callback)
        |
Coordinator (WebSocket dispatch, round orchestration, ALT-bundled v2 TX)
        |
Hardware Entropy Nodes (ESP32-S3, ECDSA secp256k1, mTLS)
```

**Protocol.** Commit–reveal with ECDSA secp256k1 signatures, SHA-256 entropy combination. 4–7 nodes per round, minimum 4 reveals to finalize. One honest node is enough to make the output unpredictable.

**On-chain node selection (v7.3).** `request_randomness_auto` CPIs to `select_nodes` using the `SlotHashes` sysvar as an unbiasable seed. The coordinator cannot influence which nodes serve a round.

**ALT-bundled v2 flow (v7.5).** `submit_round_v2` + `claim_rewards_v2` land in a single transaction via an Address Lookup Table, collapsing the old three-TX commit→reveal→finalize dance. Measured devnet latency: **avg 3.9 s** (p50 3.7, p95 4.4) over 50 back-to-back rounds — down from 8 s on v7.0.

**Fee split.** 70 % node operators · 20 % protocol treasury · 10 % reserve fund.

---

## Devnet Deployment (v7.7)

| | |
|---|---|
| **Program ID (v7.7)** | `FMwPuCjkfZXN2MuNJQiUzZC3hnxHcD8mrTuntsqA84XD` |
| **Network** | Solana Devnet |
| **Explorer** | [View on Solana Explorer](https://explorer.solana.com/address/FMwPuCjkfZXN2MuNJQiUzZC3hnxHcD8mrTuntsqA84XD?cluster=devnet) |
| **Anchor version** | 1.0.0 |
| **Predecessor (v7.5)** | `78Qv6cyKkRZN2YngiLSSBCe2iyRc6jgtCs3incCaMRcv` (still live, side-by-side) |

**Example dApps (also on devnet):**

| Program | ID |
|---|---|
| `coin_toss` | `7r6UstdP6qTFK4HSqU4mFGPGyCVWd3JVjBeafQPyvspH` |
| `dice_roll` | `CLpaMPxyu5Up4fuZb1JiY2uzj4s4iYVg9RfQHNFRuzAj` |
| `lucky_wheel` | `FzUuegZpKT4BHhzms1eJX7L2f6r3NTxMRexs8uqxtnbf` |
| `prediction_market` | `EHf5YLG2p7Wca9nUqJXRB6yATZidrBzJKM4Qj4k1EUvc` |

---

## Project Structure

```
DICE/
  programs/               Core Anchor programs
    dice/                 Core VRF program (v7.7)
  dapp-examples/          Reference integrations that CPI into `dice`
    coin-toss/            Example dApp: 50/50 coin flip
    dice-roll/            Example dApp: classic dice roll
    lucky-wheel/          Example dApp: weighted spinning wheel
    prediction-market/    Example dApp: binary outcome settlement
    pulse/                Streaming-VRF example (reads RandomnessFeed PDA)
  coordinator/            Rust coordinator (WebSocket, REST, Prometheus, Solana RPC client)
  sdk/
    dice-vrf/             Rust CPI SDK for on-chain callers
    dice-vrf-macros/      Proc-macro helpers
    dice-vrf-ts/          TypeScript SDK (@dice-network/sdk, unpublished)
  firmware/               ESP32-S3 firmware (ESP-IDF v5.x, C)
  frontend/               Next.js 15 App Router site (v5 editorial redesign, Vercel)
  tests/
    dice.ts               Core Anchor integration suite (devnet)
    dice_v2.ts            v2/v7 channel + streaming suite
    devnet_setup.ts       Bootstrap fixtures on devnet
    harness/              Rust drivers — mock nodes, smoke, stress, adversarial, coin-toss, pulse, v73
    e2e/                  End-to-end scripts
    fixtures/             Shared test inputs
  marketing/              HTML→PDF kit — deck, product cards, brandbook (separate from web)
  docs/                   Architecture, progress, changelog, test reports
  deploy/coord-do/        DigitalOcean Droplet deploy kit for the coordinator
  docker/                 Dockerfiles + compose (dev + prod + test)
  pki/                    step-ca private PKI, device certs
  packaging/              Printed welcome card + enclosure ideas + ops manual
  research/               VRF/DePIN market research, competitive analysis
  pitch_deck/             Investor pitch (HTML + PDF build)
  shoot/                  Weekly narrative video scripts
  how-it-works/           Long-form explainer pages (HTML)
  scripts/                IDL / PDA / protocol compat verifiers
  test_v7_results/        Latency + stress run outputs (v7.3 → v7.7)
  archive/                Retired files, kept for git history
    legacy/               Pre-v3 orchestrator + Makefile phase runner
    v7-test-dashboards/   v7 (pre-7.7) test dashboards superseded by v77 runs
  .github/workflows/      CI — check, test, clippy, audit
```

---

## Local Testing Guide

Three modes: pure in-memory simulation, simulation with real devnet transactions, and full unit + integration test suites.

### Prerequisites

| Tool | Required For | Install |
|------|-------------|---------|
| **Rust** (1.82+, BPF toolchain 1.84) | All Rust code | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| **Node.js** (20+) | TypeScript tests, frontend | `nvm install 20` |
| **pnpm** (10.x) | Frontend + marketing kit | `npm i -g pnpm@10` |
| **Solana CLI** (1.18) | Devnet deploy | `sh -c "$(curl -sSfL https://release.anza.xyz/stable/install)"` |
| **Anchor CLI** (1.0.0) | BPF builds | `cargo install --git https://github.com/coral-xyz/anchor avm && avm install 1.0.0` |

**Windows:** run all Rust + Anchor commands in WSL. Frontend works natively on Windows.

### 1. Build Everything

```bash
# Workspace compile check (~2 min first time)
cargo check --workspace --message-format=short

# TS test deps
npm install

# BPF binary (needs WSL + Anchor 1.0.0). IDL is maintained manually —
# the anchor-syn 0.30.1 bug prevents auto-IDL on Rust 1.93+.
anchor build --no-idl
```

Expected: `Finished ... 0 errors`.

### 2. Simulation (in-memory, no blockchain)

**Three terminals.**

**T1 — coordinator:**
```bash
cargo run --bin dice-coordinator -- --simulation
```

```
INFO  SIMULATION MODE — plain WebSocket, no DB, no Solana RPC
DICE Coordinator ready:
  Dashboard : http://localhost:8080/
  WebSocket : ws://localhost:8443/
  Metrics   : http://localhost:9090/metrics
```

If `coordinator-keypair.json` exists at the repo root, you'll also see `on-chain transactions ENABLED` and every round will submit real devnet TXs.

**T2 — 10 simulated nodes:**
```bash
cargo run --bin mock-firmware-node -- --count 10
```

**T3 — trigger a round:**
```bash
curl -s -X POST http://localhost:8080/simulate | python3 -m json.tool
```

Or click **POST /simulate** in the dashboard at `http://localhost:8080`.

Watch T1 for `round finalized! ... randomness=8f3a91c0... elapsed_ms=~3900`.

### 3. Simulation with devnet TXs

Same three terminals; also need a funded devnet keypair:

```bash
solana-keygen new -o coordinator-keypair.json --no-bip39-passphrase
solana airdrop 5 $(solana-keygen pubkey coordinator-keypair.json) --url devnet
# If rate-limited: https://faucet.solana.com
```

`/simulate` now returns `tx_signature` + explorer URL. Each round lands **one** bundled TX (v7.5 ALT) instead of three.

### 4. Rust unit + integration tests

```bash
cargo test --workspace --message-format=short
```

**Current:** 229 tests pass, 0 fail (spread across `dice-vrf`, `dice-coordinator`, 6 programs, 6 harness crates). Covers callback discriminator stability, ECDSA sig verification (k256), SHA-256 commit/reveal integrity, entropy combination determinism, RandomnessResult layout, ALT construction, and v2 channel paths.

### 5. TypeScript integration tests (devnet)

```bash
ANCHOR_PROVIDER_URL=https://api.devnet.solana.com \
ANCHOR_WALLET=./coordinator-keypair.json \
npx ts-mocha -p ./tsconfig.json -t 1000000 tests/dice.ts
```

Cost: ~0.01 SOL per full run. `tests/dice_v2.ts` exercises the v2 channel + streaming flow against the v7.7 program. Some tests show `already in use` on repeat runs — the PDA persists on devnet, so it's expected.

### 6. Docker stack

```bash
cd docker
docker compose up --build
```

| Service | URL |
|---|---|
| Coordinator dashboard | http://localhost:8080 |
| WebSocket | ws://localhost:8443 |
| Prometheus | http://localhost:9090/metrics |
| Postgres | `postgres://dice:dice@localhost/dice` |

### 7. Frontend (Next.js)

```bash
cd frontend
pnpm install
pnpm dev           # http://localhost:3000
pnpm build && pnpm start
```

Vercel deploy is wired: `rootDirectory=frontend`, pnpm@10 pinned. Production builds 34 static pages including `/preorder`, `/docs/getting-started`, explorer, manifesto, roadmap.

### 8. Marketing PDF kit

```bash
cd marketing
pnpm install
pnpm pdf           # Chromium-renders HTML slides → dist/*.pdf
```

Produces the 12-slide investor deck, 4-up product cards, operator/dev how-to cards, and the brand book.

---

## Smart Contract Instructions (v7.7, `dice` program)

| Instruction | Description |
|-------------|-------------|
| `register_device` | Register an ESP32-S3 node by secp256k1 pubkey |
| `register_node_vault` | Bind a node to its payout wallet (NodeVault, v7) |
| `rotate_payout_wallet` | Hardware-signed rotation of payout address |
| `request_randomness` | Developer requests randomness, pays 0.002 SOL |
| `request_randomness_auto` | v7.3 — CPIs `select_nodes` internally for trustless quorum |
| `submit_commit` | Post a node's commit hash (legacy per-round path) |
| `submit_reveal` | Post a node's entropy + ECDSA signature |
| `submit_round_v2` | v7.5 — one-shot bundled commit+reveal ingest |
| `finalize_randomness` | SHA-256 entropy combine, CPI callback to caller program |
| `claim_rewards` / `claim_rewards_v2` | Distribute fees (70/20/10) — v2 fuses with submit in an ALT TX |
| `init_feed` / `publish_feed_value` / `close_feed` | Streaming VRF feed lifecycle (v7) |
| `init_escrow` / `fund_escrow` | Developer escrow account lifecycle |
| `select_nodes` | SlotHashes-seeded unbiasable node-pool draw |
| `withdraw_from_vault` | NodeVault payout withdrawal |

---

## SDK Integration

**Rust CPI:**
```rust
// Cargo.toml
[dependencies]
dice-vrf = { path = "sdk/dice-vrf" }
```

```rust
let ix = dice_vrf::cpi::request_randomness_ix(
    &accounts,
    sequence,
    &my_program_id,   // your callback program
);
solana_program::program::invoke(&ix, account_infos)?;
```

**TypeScript (not yet on npm):**
```ts
import { DiceClient } from "@dice-network/sdk";  // sdk/dice-vrf-ts
const dice = new DiceClient({ connection, wallet });
const { signature, requestPda } = await dice.requestRandomness({ sequence });
const result = await dice.awaitResult(requestPda);
// result.randomness: Uint8Array(32)
```

The DICE program invokes your `dice_callback` instruction automatically with the 32-byte randomness.

---

## Documentation

| Document | Description |
|----------|-------------|
| [INDEX.md](INDEX.md) | Catalog of every top-level file and folder (what / why) |
| [docs/PROGRESS.md](docs/PROGRESS.md) | Version history and v7.7 changelog |
| [docs/TODO.md](docs/TODO.md) | Open tasks — what's shipped, what's blocked |
| [docs/SIMULATION.md](docs/SIMULATION.md) | Local simulation guide, CLI reference |
| [docs/TEST_REPORT.md](docs/TEST_REPORT.md) | Full test results with on-chain account addresses |
| [docs/ANCHOR_1_0_MIGRATION.md](docs/ANCHOR_1_0_MIGRATION.md) | 0.31 → 1.0.0 migration notes |
| [docs/CHANNEL_DESIGN.md](docs/CHANNEL_DESIGN.md) | v2 reusable DiceChannel PDA design |
| [docs/V2_CHANGELOG.md](docs/V2_CHANGELOG.md) | v1 → v2 instruction map |
| [docs/v7-universal-payout.md](docs/v7-universal-payout.md) | NodeVault + streaming-VRF design |
| [DEPLOY.md](DEPLOY.md) | Deployment notes (being swapped from Fly to DigitalOcean) |
| [tests/DICE_HARDWARE_TEST_REPORT.md](tests/DICE_HARDWARE_TEST_REPORT.md) | 545-round run on real ESP32-S3 mesh |
| [tests/ONCHAIN_VRF_TEST_RESULTS.md](tests/ONCHAIN_VRF_TEST_RESULTS.md) | On-chain VRF correctness run |
| [research/](research/) | VRF/DePIN market, competitor, and novel-delivery reports |

---

## Build Health

```
cargo check --workspace     0 errors
cargo test  --workspace     229 pass, 0 fail
anchor build --no-idl       6 programs built clean
TS tests (devnet)           passing
Frontend (Vercel prod)      34 static pages, 0 build errors
```

Latest latency run: 50 rounds, `test_v7_results/v77_latency_50.json` — avg 3.9 s, p95 4.4 s.

---

## License

Proprietary. Internal use only.
