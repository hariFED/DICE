# DICE Simulation Guide

Local end-to-end simulation of the full commit-reveal protocol — no hardware, no Solana devnet, no PostgreSQL, no TLS certificates required.

---

## What the simulation runs

| Component | Binary | Role |
|-----------|--------|------|
| Coordinator | `dice-coordinator` | Manages rounds, receives commits/reveals, broadcasts results |
| Mock nodes | `mock-firmware-node` | Simulates N ESP32-S3 devices with real k256 ECDSA keys |
| Dashboard | built into coordinator | Live browser UI at `http://localhost:8080` |

**Protocol flow per round:**
1. `POST /simulate` → coordinator selects all connected nodes, sends `JobAssignment` (CBOR/WebSocket)
2. Each mock node generates 32-byte entropy, computes `SHA-256(entropy)` as commit hash, signs it with ECDSA secp256k1, sends `CommitSubmission`
3. Coordinator validates each commit signature and records it in the state machine
4. After 500 ms, each node sends `RevealSubmission` with the raw entropy + ECDSA signature
5. Coordinator verifies `SHA-256(entropy) == commit_hash` for each reveal
6. Once `≥ min_required` reveals are received, randomness = `SHA-256(entropy1 ‖ entropy2 ‖ …)` is finalized
7. Coordinator broadcasts `RoundResult` with the final 32-byte randomness to all selected nodes

---

## Prerequisites

- Rust toolchain installed (`rustup`)
- WSL (or any bash shell on Windows)
- No database, no TLS certs, no Solana CLI needed

Verify Rust is available:

```powershell
wsl bash -c "cargo --version"
```

Expected output: `cargo 1.94.x`

---

## Quick start (3 steps)

### Step 1 — Build the workspace

```powershell
wsl bash -c "cd /mnt/c/Users/Abcom/DICE && cargo build --bins --message-format=short"
```

This compiles both `dice-coordinator` and `mock-firmware-node`. Takes ~2 min on first build, seconds after that.

---

### Step 2 — Start the coordinator

Open **PowerShell Window 1** and run:

```powershell
wsl bash -c "cd /mnt/c/Users/Abcom/DICE && cargo run --bin dice-coordinator -- --simulation"
```

Or using an interactive WSL session:

```powershell
wsl
cd /mnt/c/Users/Abcom/DICE
cargo run --bin dice-coordinator -- --simulation
```

**Expected output:**

```
INFO  dice_coordinator: SIMULATION MODE — plain WebSocket, no DB, no Solana RPC
INFO  dice_coordinator: DICE Coordinator starting ws_port=8443 api_port=8080 metrics_port=9090 simulation=true
INFO  dice_coordinator::api::routes: REST API listening addr=0.0.0.0:8080
INFO  dice_coordinator: Plain WebSocket server listening (simulation mode) addr=0.0.0.0:8443
INFO  dice_coordinator: Prometheus metrics listening addr=0.0.0.0:9090
DICE Coordinator ready:
  Dashboard : http://localhost:8080/
  WebSocket : ws://localhost:8443/
  Metrics   : http://localhost:9090/metrics
  Simulate  : curl -X POST http://localhost:8080/simulate
```

---

### Step 3 — Start mock nodes

Open **PowerShell Window 2** and run:

```powershell
wsl bash -c "cd /mnt/c/Users/Abcom/DICE && cargo run --bin mock-firmware-node -- --count 7"
```

Or interactive:

```powershell
wsl
cd /mnt/c/Users/Abcom/DICE
cargo run --bin mock-firmware-node -- --count 7
```

**Expected output:**

```
INFO  mock_firmware_node: mock-firmware-node starting count=7 coordinator="ws://localhost:8443" insecure=false
INFO  mock_firmware_node: spawning node index=0 node_id=02a3f1...
INFO  mock_firmware_node: spawning node index=1 node_id=03cc84...
...
INFO  mock_firmware_node: node connected node="02a3f1"
INFO  mock_firmware_node: node connected node="03cc84"
...
```

All 7 nodes connect and begin sending heartbeats every 5 seconds.

---

## Trigger a round

### Option A — Browser (recommended)

Open `http://localhost:8080/` in any browser.

- The **Connected Nodes** table shows all 7 nodes with latency/uptime stats
- Click **▶ POST /simulate** to trigger a round
- The **Recent Rounds** table updates automatically every 2 seconds
- Watch status change: `collecting_commits` → `collecting_reveals` → `finalized`

### Option B — curl from PowerShell

```powershell
wsl bash -c "curl -s -X POST http://localhost:8080/simulate | python3 -m json.tool"
```

**Example response:**

```json
{
  "request_id": "a3f1cc84...",
  "round_id": "550e8400-e29b-41d4-a716-446655440000",
  "selected_nodes": ["02a3f1...", "03cc84...", "..."],
  "min_required": 4,
  "dispatched": 7
}
```

### Option C — PowerShell native (no WSL needed for this)

```powershell
Invoke-RestMethod -Method Post -Uri http://localhost:8080/simulate | ConvertTo-Json
```

---

## Watching a round happen

After clicking simulate, watch **Window 1** (coordinator logs):

```
INFO  handle_node_connection: commit accepted request="a3f1cc84" node="02a3f1" status="collecting_commits"
INFO  handle_node_connection: commit accepted request="a3f1cc84" node="03cc84" status="collecting_commits"
... (7 commits total)
INFO  state_machine: all commits received — entering reveal phase commits=7
INFO  handle_node_connection: reveal accepted, waiting for more request="a3f1cc84" node="02a3f1"
... (more reveals arriving)
INFO  state_machine: round finalized request="a3f1cc84" reveals=7 randomness="8f3a91c0..."
INFO  handle_node_connection: round finalized! request="a3f1cc84" randomness="8f3a91c0..." elapsed_ms=623
```

And **Window 2** (mock node logs):

```
INFO  mock_firmware_node: job assignment received node="02a3f1" request="a3f1cc84" seq=1748000000
INFO  mock_firmware_node: commit sent node="02a3f1" commit="4e91a3..."
INFO  mock_firmware_node: reveal sent node="02a3f1" request="a3f1cc84"
INFO  mock_firmware_node: round finalized ✓ node="02a3f1" randomness="8f3a91c0..."
```

---

## CLI reference

### `dice-coordinator`

```
cargo run --bin dice-coordinator -- [OPTIONS]
```

| Flag | Env var | Default | Description |
|------|---------|---------|-------------|
| `--simulation` | `DICE_SIMULATION` | `false` | Plain WS, no DB, no TLS |
| `--ws-port` | `DICE_WS_PORT` | `8443` | WebSocket port for nodes |
| `--api-port` | `DICE_API_PORT` | `8080` | REST API + dashboard port |
| `--metrics-port` | `DICE_METRICS_PORT` | `9090` | Prometheus metrics port |
| `--database-url` | `DATABASE_URL` | *(default)* | PostgreSQL URL (skipped in sim) |
| `--min-nodes` | `DICE_MIN_NODES` | `4` | Min reveals required to finalize |
| `--max-nodes` | `DICE_MAX_NODES` | `7` | Max nodes selected per round |

**Example — custom ports:**

```powershell
wsl bash -c "cd /mnt/c/Users/Abcom/DICE && cargo run --bin dice-coordinator -- --simulation --ws-port 9443 --api-port 3000"
```

---

### `mock-firmware-node`

```
cargo run --bin mock-firmware-node -- [OPTIONS]
```

| Flag | Default | Description |
|------|---------|-------------|
| `--count N` | `5` | Number of simulated nodes to spawn |
| `--coordinator URL` | `ws://localhost:8443` | Coordinator WebSocket URL |
| `--insecure` | `false` | Skip TLS verification (for ws://) |
| `--heartbeat-ms MS` | `5000` | Heartbeat interval in milliseconds |
| `--commit-delay-ms MS` | `50` | Delay between job receipt and commit |
| `--reveal-delay-ms MS` | `500` | Delay between commit and reveal |

**Example — 20 nodes with faster reveals:**

```powershell
wsl bash -c "cd /mnt/c/Users/Abcom/DICE && cargo run --bin mock-firmware-node -- --count 20 --reveal-delay-ms 200"
```

**Example — connect to a different coordinator port:**

```powershell
wsl bash -c "cd /mnt/c/Users/Abcom/DICE && cargo run --bin mock-firmware-node -- --count 5 --coordinator ws://localhost:9443"
```

---

## REST API endpoints

All served at `http://localhost:8080` (or your configured `--api-port`).

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/` | HTML live dashboard |
| `GET` | `/health` | `{"status":"ok"}` liveness check |
| `GET` | `/nodes` | List of all connected nodes with stats |
| `GET` | `/rounds` | In-memory list of recent rounds (last 50) |
| `GET` | `/rounds/:uuid` | Single round by UUID (requires DB) |
| `POST` | `/simulate` | Trigger a new round |
| `GET` | `/metrics` | Prometheus metrics (text format) |

**Check connected nodes:**

```powershell
wsl bash -c "curl -s http://localhost:8080/nodes | python3 -m json.tool"
```

**List recent rounds:**

```powershell
wsl bash -c "curl -s http://localhost:8080/rounds | python3 -m json.tool"
```

---

## Prometheus metrics

Available at `http://localhost:9090/metrics`:

```
dice_nodes_connected            # currently connected nodes
dice_rounds_total               # total rounds started this session
dice_rounds_failed_total        # rounds that timed out or failed
dice_round_duration_seconds     # histogram of round completion time
dice_mtls_handshake_failed_total # mTLS failures (0 in simulation mode)
```

---

## Troubleshooting

### "no nodes connected" error from /simulate

The mock nodes haven't connected yet. Wait a second after starting `mock-firmware-node` — it registers on first heartbeat, not on TCP connect.

### Reveals are rejected ("round is not in RevealCollection state")

The commit phase isn't complete yet. The coordinator only accepts reveals once **all** selected nodes have committed. If one node is slow or disconnected, the others' early reveals will be rejected, then the node will not retry.

**Fix:** Use `--reveal-delay-ms 1000` (1 second) to give all nodes time to commit before any reveal is sent.

```powershell
wsl bash -c "cd /mnt/c/Users/Abcom/DICE && cargo run --bin mock-firmware-node -- --count 7 --reveal-delay-ms 1000"
```

### Rounds stuck in "collecting_commits"

A node disconnected after receiving the job but before sending a commit. Trigger a new round — the stuck round will remain in memory but new rounds work independently.

### Address already in use

Another process is using port 8080, 8443, or 9090. Either kill it or use different ports:

```powershell
# Coordinator on alternate ports
wsl bash -c "cd /mnt/c/Users/Abcom/DICE && cargo run --bin dice-coordinator -- --simulation --ws-port 9000 --api-port 3000 --metrics-port 9091"

# Mock nodes pointed at the new WS port
wsl bash -c "cd /mnt/c/Users/Abcom/DICE && cargo run --bin mock-firmware-node -- --count 7 --coordinator ws://localhost:9000"
```

### cargo check / cargo test ICE (internal compiler error)

Known issue: rustc 1.94.x JSON renderer panics on future-compat warnings from some deps.

**Workaround:** Always use `--message-format=short` for check/test (not needed for `run`):

```powershell
wsl bash -c "cd /mnt/c/Users/Abcom/DICE && cargo check --workspace --message-format=short"
wsl bash -c "cd /mnt/c/Users/Abcom/DICE && cargo test --workspace --message-format=short"
```

---

## Running all tests

```powershell
wsl bash -c "cd /mnt/c/Users/Abcom/DICE && cargo test --workspace --message-format=short"
```

Expected: **11 tests pass, 0 fail**

```
test result: ok. 1 passed   ← basic smoke test
test result: ok. 4 passed   ← protocol validation (verify_commit, verify_reveal, combine_entropy)
test result: ok. 6 passed   ← CPI/SDK (discriminators, instruction layout, decode)
```

---

## Production mode (not simulation)

Production mode requires real TLS certificates (from step-ca), a PostgreSQL database, and a Solana keypair. Do **not** use `--simulation`.

```bash
cargo run --bin dice-coordinator -- \
  --database-url "postgres://dice:password@localhost/dice" \
  --solana-rpc-url "https://api.mainnet-beta.solana.com" \
  --coordinator-keypair-path "/etc/dice/keypair.json" \
  --tls-cert-path "/etc/dice/certs/coordinator.crt" \
  --tls-key-path "/etc/dice/certs/coordinator.key" \
  --ca-cert-path "/etc/dice/certs/ca.crt"
```

Hardware ESP32-S3 nodes connect via `wss://` with mutual TLS — never plain `ws://`.
