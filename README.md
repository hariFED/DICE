# DICE Network

**Distributed Infrastructure for Cryptographic Entropy**

Hardware-backed verifiable randomness oracle on Solana. ESP32-S3 nodes generate entropy, participate in a commit-reveal protocol, and deliver verifiable randomness to on-chain programs at 0.002 SOL per request.

---

## Architecture

```
Applications (smart contracts that need randomness)
        |
Solana Smart Contract (commit-reveal, on-chain verification)
        |
Coordinator (node selection, job dispatch, round management)
        |
Hardware Entropy Nodes (ESP32-S3, ECDSA signing, mTLS)
```

**Protocol:** Commit-reveal with ECDSA secp256k1 signatures. SHA-256 entropy combination. 4-7 nodes per round, minimum 4 reveals required. One honest node guarantees unpredictable output.

**Fee split:** 70% node operators / 20% protocol treasury / 10% reserve fund.

---

## Devnet Deployment

| | |
|---|---|
| **Program ID** | `78Qv6cyKkRZN2YngiLSSBCe2iyRc6jgtCs3incCaMRcv` |
| **Network** | Solana Devnet |
| **Explorer** | [View on Solana Explorer](https://explorer.solana.com/address/78Qv6cyKkRZN2YngiLSSBCe2iyRc6jgtCs3incCaMRcv?cluster=devnet) |

---

## Project Structure

```
DICE/
  programs/dice/          Anchor smart contract (8 instructions, 6 account types)
  coordinator/            Rust coordinator server (WebSocket, REST API, Prometheus)
  sdk/dice-vrf/           Rust SDK for on-chain CPI integration
  sdk/dice-vrf-macros/    Proc-macro helpers for the SDK
  firmware/               ESP32-S3 firmware (C, ESP-IDF v5.x)
  tests/
    dice.ts               TypeScript integration tests (Anchor)
    harness/
      mock_firmware_node/ Simulated ESP32 nodes (k256 ECDSA, CBOR, WebSocket)
      load_generator/     Load testing tool
  pki/                    Private PKI (step-ca, device certificates)
  scripts/                Verification scripts (IDL, PDA, protocol compat)
  docker/                 Dockerfiles and compose configs
  docs/                   Documentation, architecture docs, test reports
  .github/workflows/      CI pipeline (check, test, clippy, audit)
```

---

## Local Testing Guide

Everything you need to build, run, and test DICE locally. Three modes: pure in-memory simulation, simulation with real devnet transactions, and full unit/integration test suites.

### Prerequisites

| Tool | Required For | Install |
|------|-------------|---------|
| **Rust** (1.82+) | All Rust code | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| **Node.js** (18+) | TypeScript tests | `nvm install 20` or [nodejs.org](https://nodejs.org) |
| **Solana CLI** | Devnet deploy/test | `sh -c "$(curl -sSfL https://release.anza.xyz/stable/install)"` |
| **Anchor CLI** (0.30) | BPF builds | `cargo install --git https://github.com/coral-xyz/anchor avm && avm install 0.30.1` |

**Windows users:** Run all commands in WSL or Git Bash. The project builds on Windows but the Solana toolchain runs in WSL.

### 1. Build Everything

```bash
# Clone and enter the project
cd DICE

# Install Rust dependencies and check compilation (takes ~2 min first time)
cargo check --workspace --message-format=short

# Install TypeScript test dependencies
npm install

# Build the BPF binary (requires WSL with Anchor CLI)
# Only needed if you want to deploy to devnet
anchor build --no-idl
```

**Expected output:** `Finished dev profile ... 0 errors`

---

### 2. Run Simulation (In-Memory, No Blockchain)

This is the fastest way to see the full commit-reveal protocol in action. No Solana, no database, no certificates. Everything runs in memory.

**Open 3 terminals:**

**Terminal 1 — Start the coordinator:**
```bash
cargo run --bin dice-coordinator -- --simulation
```

You'll see:
```
INFO  SIMULATION MODE — plain WebSocket, no DB, no Solana RPC
DICE Coordinator ready:
  Dashboard : http://localhost:8080/
  WebSocket : ws://localhost:8443/
  Metrics   : http://localhost:9090/metrics
  Simulate  : curl -X POST http://localhost:8080/simulate
```

If `coordinator-keypair.json` exists in the project root, you'll also see:
```
INFO  on-chain transactions ENABLED
```
This means the simulation will also submit real transactions to Solana devnet.

**Terminal 2 — Start 10 simulated nodes:**
```bash
cargo run --bin mock-firmware-node -- --count 10
```

You'll see 10 nodes spawn, each with a unique k256 ECDSA keypair:
```
INFO  spawning node index=0 node_id=02a3f1...
INFO  spawning node index=1 node_id=03cc84...
...
INFO  node connected node="02a3f1"
```

Nodes send heartbeats every 5 seconds. Wait until all 10 show "WebSocket connected".

**Terminal 3 — Trigger a round:**
```bash
curl -s -X POST http://localhost:8080/simulate | python3 -m json.tool
```

**Or open the dashboard:** Go to `http://localhost:8080` in your browser and click the **POST /simulate** button.

**What happens next (watch Terminal 1):**

```
INFO  commit accepted  request="a3f1cc84..." node="02a3f1..." status="collecting_commits"
INFO  commit accepted  request="a3f1cc84..." node="03cc84..." status="collecting_commits"
... (10 commits)
INFO  all commits received — entering reveal phase
INFO  reveal accepted, waiting for more  request="a3f1cc84..." node="02a3f1..."
... (10 reveals)
INFO  round finalized! request="a3f1cc84..." randomness="8f3a91c0..." elapsed_ms=623
```

**What happens on the nodes (Terminal 2):**
```
INFO  job assignment received  node="02a3f1" request="a3f1cc84..."
INFO  commit sent  node="02a3f1" commit="4e91a3..."
INFO  reveal sent  node="02a3f1" request="a3f1cc84..."
INFO  round finalized  node="02a3f1" randomness="8f3a91c0..."
```

**Dashboard shows:**
- Connected nodes table with latency, uptime, jobs completed
- Round status: `collecting_commits` -> `collecting_reveals` -> `finalized`
- Final 32-byte randomness value

You can trigger as many rounds as you want. Each produces a unique randomness output.

---

### 3. Run Simulation with Devnet Transactions

Same as above, but every protocol step also submits a real Solana transaction. You can verify everything on [Solana Explorer](https://explorer.solana.com/?cluster=devnet).

**Setup (one time):**
```bash
# Generate a Solana keypair for the coordinator
solana-keygen new -o coordinator-keypair.json --no-bip39-passphrase

# Fund it with devnet SOL (free)
solana airdrop 5 $(solana-keygen pubkey coordinator-keypair.json) --url devnet

# If airdrop fails (rate limited), use the web faucet:
# https://faucet.solana.com
# Paste your address: solana-keygen pubkey coordinator-keypair.json
```

**Run it (same 3 terminals as above):**

```bash
# Terminal 1 — Coordinator with devnet enabled
cargo run --bin dice-coordinator -- --simulation

# Terminal 2 — Mock nodes
cargo run --bin mock-firmware-node -- --count 7

# Terminal 3 — Trigger
curl -s -X POST http://localhost:8080/simulate | python3 -m json.tool
```

**The /simulate response now includes Solana transaction info:**
```json
{
  "request_id": "a3f1cc84...",
  "dispatched": 7,
  "sequence": 1748383200,
  "tx_signature": "5xK9v2...",
  "explorer": "https://explorer.solana.com/tx/5xK9v2...?cluster=devnet"
}
```

Click the `explorer` link to see `request_randomness` on Solana Explorer. The coordinator logs will also show `submit_commit TX sent` and `finalize_randomness TX sent` with their own signatures.

**On-chain transactions per round:**
| Step | Transaction | What it does |
|------|------------|--------------|
| 1 | `request_randomness` | Creates RandomnessRequest + Escrow PDAs, locks 0.002 SOL |
| 2 | `submit_commit` (x N) | Posts each node's commit hash on-chain |
| 3 | `finalize_randomness` | Combines entropy, writes RandomnessResult PDA |

---

### 4. Run Rust Unit Tests

```bash
cargo test --workspace --message-format=short
```

**Expected: 13 tests pass, 0 fail**

```
test test_id ... ok
test verify_callback_discriminator ... ok
test verify_reveal_roundtrip ... ok
test verify_reveal_wrong_entropy ... ok
test combine_entropy_deterministic ... ok
test combine_entropy_order_matters ... ok
test decode_randomness_result_too_short ... ok
test decode_randomness_result_zeroed ... ok
test decode_randomness_result_valid ... ok
test dice_callback_discriminator_is_stable ... ok
test discriminator_is_deterministic ... ok
test discriminator_differs_by_name ... ok
test request_randomness_ix_data_layout ... ok

test result: ok. 13 passed; 0 failed
```

These tests verify:
- Smart contract callback discriminator computation
- ECDSA signature verification (k256)
- SHA-256 commit-reveal hash integrity
- Entropy combination determinism
- SDK instruction data layout
- RandomnessResult account deserialization

---

### 5. Run TypeScript Integration Tests (Solana Devnet)

These tests execute **real on-chain transactions** against the deployed program on Solana devnet.

**Prerequisites:**
- `coordinator-keypair.json` with devnet SOL (see step 3 setup)
- `npm install` completed
- Program deployed to devnet (already done: `78Qv6cyKkRZN2YngiLSSBCe2iyRc6jgtCs3incCaMRcv`)

```bash
ANCHOR_PROVIDER_URL=https://api.devnet.solana.com \
ANCHOR_WALLET=./coordinator-keypair.json \
npx ts-mocha -p ./tsconfig.json -t 1000000 tests/dice.ts
```

**Expected: 10-11 tests pass**

```
  dice
    [OK] registers a hardware device               (creates DeviceRegistry PDA)
    [OK] rejects duplicate device registration      (PDA already exists)
    [OK] creates a randomness request and escrow    (0.002 SOL locked)
    [OK] accepts commits from all selected nodes    (5 CommitRecord PDAs)
    [OK] rejects a duplicate commit                 (PDA collision)
    [OK] records a commit for each device           (state verification)
    [OK] initialises a standalone escrow account    (seq=42)
    [OK] funds an existing escrow account           (1M lamports)
    [OK] rejects duplicate sequence                 (request PDA exists)
    [OK] rejects commit on non-existent request     (missing PDA)
    [OK] derives all PDAs deterministically         (pure math)

  10-11 passing (10s)
```

**Note:** Test 1 (`registers a hardware device`) may show as "already in use" on repeat runs because the DeviceRegistry PDA persists on devnet. This is expected — the account was created successfully on the first run.

**Cost:** Each full test run uses ~0.01 SOL in transaction fees and account rent.

---

### 6. Run with Docker

One-command startup of the full stack (PostgreSQL + Coordinator + 7 mock nodes):

```bash
cd docker
docker compose up --build
```

**Services started:**
| Service | Port | URL |
|---------|------|-----|
| PostgreSQL | 5432 | `postgres://dice:dice@localhost/dice` |
| Coordinator Dashboard | 8080 | http://localhost:8080 |
| Coordinator WebSocket | 8443 | ws://localhost:8443 |
| Prometheus Metrics | 9090 | http://localhost:9090/metrics |

Open `http://localhost:8080` and click **POST /simulate** to trigger rounds.

**Stop:**
```bash
docker compose down -v   # -v removes the postgres volume
```

---

### 7. API Reference (for manual testing)

All endpoints are on the coordinator's REST API (default port 8080).

```bash
# Health check
curl http://localhost:8080/health

# List connected nodes
curl -s http://localhost:8080/nodes | python3 -m json.tool

# List recent rounds (in-memory)
curl -s http://localhost:8080/rounds | python3 -m json.tool

# Trigger a new round
curl -s -X POST http://localhost:8080/simulate | python3 -m json.tool

# Prometheus metrics
curl http://localhost:9090/metrics
```

---

### 8. Troubleshooting

| Problem | Solution |
|---------|----------|
| **"no nodes connected"** when calling /simulate | Wait 3-5 seconds after starting mock nodes — they register on first heartbeat |
| **Reveals rejected** ("not in RevealCollection state") | Increase reveal delay: `--reveal-delay-ms 1000` |
| **"Address already in use"** | Kill previous coordinator: `pkill dice-coordinator` or use different ports: `--api-port 3000 --ws-port 9000` |
| **cargo check ICE** (internal compiler error) | Use `--message-format=short` — known rustc 1.94.x bug with JSON renderer |
| **TypeScript test "already in use"** | Expected on devnet — PDA was created in a previous run |
| **Airdrop rate limited** | Use https://faucet.solana.com or wait 24 hours |
| **WSL port not accessible from Windows** | The coordinator binds to `0.0.0.0` so ports should forward. If not, access via WSL IP: `wsl hostname -I` |

---

### 9. CLI Flags Reference

**Coordinator (`dice-coordinator`):**

| Flag | Default | Description |
|------|---------|-------------|
| `--simulation` | false | Plain WebSocket, no DB, no TLS |
| `--api-port` | 8080 | REST API + dashboard |
| `--ws-port` | 8443 | WebSocket for nodes |
| `--metrics-port` | 9090 | Prometheus metrics |
| `--solana-rpc-url` | `https://api.devnet.solana.com` | Solana RPC endpoint |
| `--coordinator-keypair-path` | `coordinator-keypair.json` | Solana keypair for signing TXs |
| `--min-nodes` | 4 | Minimum reveals to finalize |
| `--max-nodes` | 7 | Max nodes per round |

**Mock Firmware Node (`mock-firmware-node`):**

| Flag | Default | Description |
|------|---------|-------------|
| `--count` | 5 | Number of simulated devices |
| `--coordinator` | `ws://localhost:8443` | Coordinator WebSocket URL |
| `--heartbeat-ms` | 5000 | Heartbeat interval |
| `--commit-delay-ms` | 50 | Delay before sending commit |
| `--reveal-delay-ms` | 500 | Delay between commit and reveal |

See [docs/SIMULATION.md](docs/SIMULATION.md) for the complete reference and [docs/TEST_REPORT.md](docs/TEST_REPORT.md) for full test results with on-chain account addresses.

---

## Smart Contract Instructions

| Instruction | Description |
|-------------|-------------|
| `register_device` | Register an ESP32-S3 node by secp256k1 pubkey |
| `request_randomness` | Developer requests randomness (pays 0.002 SOL) |
| `submit_commit` | Coordinator posts a node's commit hash |
| `submit_reveal` | Coordinator posts a node's entropy + ECDSA signature |
| `finalize_randomness` | Combine entropy via SHA-256, write result, CPI callback |
| `claim_rewards` | Distribute fees (70/20/10 split) |
| `init_escrow` | Create developer escrow account |
| `fund_escrow` | Add SOL to escrow |

---

## SDK Integration (for Solana developers)

```rust
// Cargo.toml
[dependencies]
dice-vrf = { path = "sdk/dice-vrf" }
```

```rust
// Request randomness with CPI callback
let ix = dice_vrf::cpi::request_randomness_ix(
    &accounts,
    sequence,
    &my_program_id,  // your callback program
);
solana_program::program::invoke(&ix, account_infos)?;
```

The DICE program calls your `dice_callback` instruction automatically with the 32-byte randomness value.

---

## Documentation

| Document | Description |
|----------|-------------|
| [docs/SIMULATION.md](docs/SIMULATION.md) | Local simulation guide with CLI reference |
| [docs/PROGRESS.md](docs/PROGRESS.md) | Build progress and devnet roadmap |
| [docs/TEST_REPORT.md](docs/TEST_REPORT.md) | Full test results with on-chain account addresses |
| [docs/DICE_Complete_Architecture.docx](docs/DICE_Complete_Architecture.docx) | Complete technical architecture |
| [docs/DICE_Tech_Stack_OpSec.docx](docs/DICE_Tech_Stack_OpSec.docx) | Technology stack and operational security |

---

## Build Health

```
cargo check --workspace    0 errors
cargo test  --workspace    13 pass, 0 fail
anchor build --no-idl      dice.so built
TypeScript tests (devnet)  10 pass, 0 fail, 1 skip*
```

*\*Device 1 registration skipped because PDA already exists from previous devnet run.*

---

## License

Proprietary. Internal use only.
