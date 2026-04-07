# DICE v5 Implementation Procedure: Keeper + Notary

**Branch:** `v5-keeper-notary`
**Base:** `v4` (which is on top of `v3` — 545+ VRF rounds, 162 tests, mTLS, PostgreSQL)
**Goal:** Add Keeper automation and Notary timestamping as parallel services alongside VRF.
**Constraint:** ZERO changes to firmware, state_machine.rs, protocol/messages.rs, or programs/dice/.

---

## Architecture Principle

**Parallel paths, not refactoring.** Keeper and Notary run as independent modules. They share infrastructure (NodeRegistry, OnChainCtx, PgPool) but have zero interaction with the VRF commit-reveal state machine.

```
coordinator/src/
  ├── main.rs                ← spawn keeper task, wire notary routes
  ├── keeper.rs              ← NEW: keeper loop + task management
  ├── notary.rs              ← NEW: attestation handler + receipt generation
  ├── state_machine.rs       ← UNTOUCHED
  ├── protocol/messages.rs   ← UNTOUCHED
  ├── protocol/validation.rs ← UNTOUCHED
  ├── queue.rs               ← UNTOUCHED
  ├── solana_watcher.rs      ← UNTOUCHED
  ├── solana_ws.rs           ← UNTOUCHED
  ├── solana_rpc.rs          ← SHARED (sign_and_send)
  ├── solana_tx.rs           ← ADD keeper instruction builders
  ├── node_session.rs        ← SHARED (registry read-only)
  ├── config.rs              ← ADD keeper/notary config flags
  ├── db/schema.sql          ← ADD keeper_tasks, keeper_executions, notary_attestations tables
  ├── db/queries.rs          ← ADD keeper + notary query functions
  ├── api/routes.rs          ← ADD /keeper/*, /notarize endpoints + dashboard update
  └── metrics.rs             ← ADD keeper + notary metrics
```

---

## Phase 1: Keeper Network (Days 1-3)

### Step 1.1 — Config flags
**File:** `coordinator/src/config.rs`
**What:** Add keeper configuration to the existing `Config` struct.

```rust
// ADD these fields to the Config struct:

/// Enable the keeper automation service
#[arg(long, env = "DICE_KEEPER_ENABLED", default_value_t = false)]
pub keeper_enabled: bool,

/// Keeper evaluation interval in seconds (how often to check triggers)
#[arg(long, env = "DICE_KEEPER_INTERVAL_SECS", default_value_t = 10)]
pub keeper_interval_secs: u64,

/// Maximum concurrent keeper executions
#[arg(long, env = "DICE_KEEPER_MAX_CONCURRENT", default_value_t = 5)]
pub keeper_max_concurrent: u32,
```

**Test:** `cargo check --bin dice-coordinator`

---

### Step 1.2 — Keeper data structures
**File:** `coordinator/src/keeper.rs` (NEW — ~350 lines)
**What:** Define keeper task types, state, and the evaluation loop.

```rust
// --- Data types ---

/// Trigger: when should this task execute?
pub enum KeeperTrigger {
    /// Cron-like schedule: "*/10 * * * * *" (every 10 seconds for demo)
    Cron { schedule: String, next_fire: Instant },
    /// One-shot: execute once at a specific time
    Once { fire_at: Instant },
    /// Interval: execute every N seconds
    Interval { every_secs: u64, next_fire: Instant },
}

/// A registered keeper task.
pub struct KeeperTask {
    pub id: Uuid,
    pub name: String,
    pub trigger: KeeperTrigger,
    pub target_program: Pubkey,
    pub instruction_data: Vec<u8>,
    pub accounts: Vec<AccountMeta>,
    pub enabled: bool,
    pub created_at: Instant,
    // Execution stats
    pub total_executions: u64,
    pub total_failures: u64,
    pub last_execution: Option<Instant>,
    pub last_tx_signature: Option<String>,
}

/// Record of a single keeper execution.
pub struct KeeperExecution {
    pub task_id: Uuid,
    pub task_name: String,
    pub tx_signature: Option<String>,
    pub success: bool,
    pub error: Option<String>,
    pub latency_ms: u64,
    pub slot: Option<u64>,
    pub timestamp: u64,
}

/// Shared keeper state.
pub type KeeperState = Arc<Mutex<KeeperManager>>;

pub struct KeeperManager {
    pub tasks: Vec<KeeperTask>,
    pub history: VecDeque<KeeperExecution>,  // ring buffer, max 200
    pub running: u32,
}
```

**Key functions to implement:**

```rust
/// The main keeper loop — spawned as a tokio task.
/// Runs every `interval_secs`, evaluates triggers, executes due tasks.
pub async fn keeper_loop(
    state: KeeperState,
    on_chain: OnChainCtx,
    metrics: Metrics,
    db: Option<PgPool>,
    interval_secs: u64,
    max_concurrent: u32,
)

/// Evaluate which tasks are due to fire right now.
fn evaluate_triggers(tasks: &mut [KeeperTask]) -> Vec<usize>

/// Execute a single keeper task: build tx, sign, submit, record result.
async fn execute_task(
    task: &mut KeeperTask,
    on_chain: &OnChainCtx,
) -> Result<KeeperExecution>

/// Register a new keeper task.
pub fn register_task(manager: &mut KeeperManager, task: KeeperTask) -> Uuid

/// List all tasks with their status.
pub fn list_tasks(manager: &KeeperManager) -> Vec<serde_json::Value>
```

**Test:** Unit test `evaluate_triggers` with mock tasks. Test cron/interval/once triggers fire at correct times.

---

### Step 1.3 — Demo keeper program (on-chain)
**File:** `programs/dice-keeper-demo/src/lib.rs` (NEW — ~80 lines)
**What:** A trivial Anchor program the keeper cranks. Proves keeper works on devnet.

```rust
use anchor_lang::prelude::*;

declare_id!("KEEPER_DEMO_PROGRAM_ID_HERE");

#[program]
pub mod dice_keeper_demo {
    use super::*;

    /// Initialize the counter PDA.
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let counter = &mut ctx.accounts.counter;
        counter.authority = ctx.accounts.authority.key();
        counter.count = 0;
        counter.last_cranked_at = Clock::get()?.unix_timestamp;
        counter.last_cranked_slot = Clock::get()?.slot;
        Ok(())
    }

    /// Increment the counter — this is what the keeper cranks.
    pub fn crank(ctx: Context<Crank>) -> Result<()> {
        let counter = &mut ctx.accounts.counter;
        counter.count += 1;
        counter.last_cranked_at = Clock::get()?.unix_timestamp;
        counter.last_cranked_slot = Clock::get()?.slot;
        msg!("DICE Keeper cranked! Count: {}", counter.count);
        Ok(())
    }
}

#[account]
pub struct Counter {
    pub authority: Pubkey,
    pub count: u64,
    pub last_cranked_at: i64,
    pub last_cranked_slot: u64,
}

// ... Accounts structs for Initialize and Crank
```

**Deploy:** `anchor build && anchor deploy --provider.cluster devnet`
**Test:** Manual `crank` instruction via CLI to verify it works before keeper integration.

---

### Step 1.4 — Keeper instruction builder
**File:** `coordinator/src/solana_tx.rs` (ADD to existing file)
**What:** Add functions to build + submit keeper transactions.

```rust
/// Build and submit a keeper crank transaction.
/// Returns (tx_signature, slot) on success.
pub async fn execute_keeper_task(
    rpc: &SolanaRpc,
    keypair: &Keypair,
    target_program: &Pubkey,
    instruction_data: &[u8],
    accounts: &[AccountMeta],
) -> Result<(String, Option<u64>)> {
    let ix = Instruction {
        program_id: *target_program,
        accounts: accounts.to_vec(),
        data: instruction_data.to_vec(),
    };

    let blockhash = rpc.get_latest_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&keypair.pubkey()),
        &[keypair],
        blockhash,
    );

    let sig = rpc.sign_and_send(keypair, tx).await?;
    Ok((sig, None))  // slot from confirmation
}
```

**Test:** Integration test submitting a crank to devnet.

---

### Step 1.5 — Database tables for keeper
**File:** `coordinator/src/db/schema.sql` (APPEND)

```sql
-- Keeper tasks
CREATE TABLE IF NOT EXISTS keeper_tasks (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name             TEXT NOT NULL,
    trigger_type     TEXT NOT NULL,          -- 'cron' | 'interval' | 'once'
    trigger_config   JSONB NOT NULL,         -- {"schedule": "*/10 * * * * *"} or {"every_secs": 10}
    target_program   BYTEA NOT NULL,         -- 32-byte Pubkey
    instruction_data BYTEA NOT NULL,
    accounts_json    JSONB NOT NULL,         -- serialized Vec<AccountMeta>
    enabled          BOOLEAN DEFAULT TRUE,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    total_executions BIGINT DEFAULT 0,
    total_failures   BIGINT DEFAULT 0,
    last_executed_at TIMESTAMPTZ
);

-- Keeper execution log
CREATE TABLE IF NOT EXISTS keeper_executions (
    id            BIGSERIAL PRIMARY KEY,
    task_id       UUID REFERENCES keeper_tasks(id),
    tx_signature  TEXT,
    success       BOOLEAN NOT NULL,
    error_msg     TEXT,
    latency_ms    INTEGER NOT NULL,
    slot          BIGINT,
    executed_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_keeper_exec_task ON keeper_executions(task_id);
CREATE INDEX IF NOT EXISTS idx_keeper_exec_time ON keeper_executions(executed_at DESC);
```

**File:** `coordinator/src/db/queries.rs` (ADD functions)

```rust
pub async fn create_keeper_task(pool: &PgPool, ...) -> Result<Uuid>
pub async fn list_keeper_tasks(pool: &PgPool) -> Result<Vec<KeeperTaskRow>>
pub async fn record_keeper_execution(pool: &PgPool, ...) -> Result<()>
pub async fn update_keeper_task_stats(pool: &PgPool, task_id: Uuid, success: bool) -> Result<()>
pub async fn toggle_keeper_task(pool: &PgPool, task_id: Uuid, enabled: bool) -> Result<()>
pub async fn delete_keeper_task(pool: &PgPool, task_id: Uuid) -> Result<()>
pub async fn get_keeper_history(pool: &PgPool, limit: i64) -> Result<Vec<KeeperExecutionRow>>
```

---

### Step 1.6 — API endpoints for keeper
**File:** `coordinator/src/api/routes.rs` (ADD)
**What:** REST endpoints for managing and monitoring keeper tasks.

```
POST   /keeper/tasks          — Register a new keeper task
GET    /keeper/tasks          — List all tasks with stats
GET    /keeper/tasks/:id      — Get task details
DELETE /keeper/tasks/:id      — Remove a task
POST   /keeper/tasks/:id/toggle — Enable/disable a task
GET    /keeper/history        — Recent execution log (last 50)
GET    /keeper/stats          — Aggregate stats (total executions, success rate, avg latency)
```

**Request body for POST /keeper/tasks:**
```json
{
  "name": "demo-counter-crank",
  "trigger": {
    "type": "interval",
    "every_secs": 10
  },
  "target_program": "KEEPER_DEMO_PROGRAM_ID",
  "instruction_data": "base64_encoded_discriminator",
  "accounts": [
    { "pubkey": "CounterPDA...", "is_signer": false, "is_writable": true },
    { "pubkey": "CoordinatorPubkey", "is_signer": true, "is_writable": true },
    { "pubkey": "11111111111111111111111111111111", "is_signer": false, "is_writable": false }
  ]
}
```

**Response:**
```json
{
  "task_id": "uuid",
  "name": "demo-counter-crank",
  "status": "active",
  "trigger": { "type": "interval", "every_secs": 10 },
  "next_fire_in_secs": 7
}
```

---

### Step 1.7 — AppState + main.rs wiring
**File:** `coordinator/src/api/routes.rs` — Add to AppState:

```rust
pub struct AppState {
    // ... existing fields ...
    pub keeper_state: Option<keeper::KeeperState>,
}
```

**File:** `coordinator/src/main.rs` — Add keeper task spawning:

```rust
// After step 9 (Solana WebSocket subscriber), add:

// 10. Spawn Keeper evaluation loop (if enabled).
let keeper_handle = if cfg.keeper_enabled {
    let ks = keeper::new_keeper_state();
    // Store in api_state for route handlers
    // ...
    let oc = on_chain.clone().expect("keeper requires on-chain context");
    let m = metrics.clone();
    let db_opt = pool.clone();
    let interval = cfg.keeper_interval_secs;
    let max_conc = cfg.keeper_max_concurrent;
    Some(tokio::spawn(async move {
        keeper::keeper_loop(ks, oc, m, db_opt, interval, max_conc).await;
    }))
} else {
    info!("Keeper service DISABLED (use --keeper-enabled to activate)");
    None
};
```

**File:** `coordinator/src/main.rs` — Add `mod keeper;` to module declarations.

---

### Step 1.8 — Dashboard update for keeper
**File:** `coordinator/src/api/routes.rs` — Update `DASHBOARD_HTML`
**What:** Add a "Keeper Automation" section below the VRF section.

New dashboard section shows:
- Keeper status: ACTIVE / DISABLED
- Total executions, success rate, avg latency
- Active tasks table: name, trigger, last execution, next fire, status
- Recent executions table: task name, tx signature (linked to explorer), success/fail, latency

The dashboard JS polls `/keeper/stats` and `/keeper/history` on the same 2-second interval.

---

### Step 1.9 — Metrics for keeper
**File:** `coordinator/src/metrics.rs` (ADD)

```rust
// Add to Metrics struct:
pub keeper_executions_total: Counter,
pub keeper_failures_total: Counter,
pub keeper_execution_latency_seconds: Histogram,
pub keeper_active_tasks: IntGauge,
```

---

### Step 1.10 — Tests for keeper
**File:** `coordinator/src/keeper.rs` — `#[cfg(test)] mod tests`

```rust
#[test] fn test_interval_trigger_fires_on_time()
#[test] fn test_interval_trigger_does_not_fire_early()
#[test] fn test_cron_trigger_parsing()
#[test] fn test_once_trigger_fires_then_disables()
#[test] fn test_register_and_list_tasks()
#[test] fn test_execution_history_ring_buffer()
#[test] fn test_max_concurrent_limit()
```

**Regression:** `cargo test --workspace` — all 162+ existing tests must still pass.

---

## Phase 2: Notary / Timestamping (Day 3-4)

### Step 2.1 — Notary data structures
**File:** `coordinator/src/notary.rs` (NEW — ~250 lines)

```rust
/// A notarization request (from API).
pub struct NotarizeRequest {
    pub content_hash: [u8; 32],
    pub hash_algorithm: String,  // "sha256"
    pub metadata: Option<serde_json::Value>,
    pub store_on_chain: bool,    // whether to write PDA (costs ~0.002 SOL)
}

/// A single witness attestation.
pub struct WitnessAttestation {
    pub device_pubkey: [u8; 33],
    pub signature: [u8; 64],
    pub commit_hash: [u8; 32],   // SHA-256(content_hash || nonce)
    pub nonce: [u8; 32],
}

/// Complete notary receipt — self-contained, independently verifiable.
#[derive(Serialize)]
pub struct NotaryReceipt {
    pub version: String,           // "1.0"
    pub receipt_type: String,      // "dice-notary-receipt"
    pub receipt_id: Uuid,

    // Attestation data
    pub content_hash: String,      // hex
    pub hash_algorithm: String,
    pub timestamp_unix: u64,
    pub timestamp_iso: String,
    pub solana_slot: Option<u64>,
    pub solana_signature: Option<String>,
    pub attestation_pda: Option<String>,

    // Witness signatures
    pub witnesses: Vec<WitnessInfo>,
    pub witness_count: u8,
    pub threshold: u8,

    // Protocol metadata
    pub network: String,
    pub coordinator_pubkey: String,

    // Verification instructions
    pub verification: VerificationInfo,
}

/// Shared notary state.
pub type NotaryState = Arc<Mutex<NotaryManager>>;

pub struct NotaryManager {
    pub history: VecDeque<NotaryReceipt>,  // ring buffer, max 100
    pub total_attestations: u64,
}
```

**Key functions:**

```rust
/// Handle a notarization request:
/// 1. Validate input (hash is 32 bytes, algorithm supported)
/// 2. Select N nodes from registry
/// 3. Dispatch hash to nodes as a JobAssignment (reuse VRF pipeline!)
/// 4. Collect commit signatures (ECDSA over the hash)
/// 5. Build receipt
/// 6. Optionally write on-chain attestation PDA
/// 7. Return receipt
pub async fn handle_notarize(
    request: NotarizeRequest,
    registry: &NodeRegistry,
    rounds: &RoundMap,
    on_chain: Option<&OnChainCtx>,
    db: Option<&PgPool>,
    min_witnesses: usize,
) -> Result<NotaryReceipt>

/// Build a self-contained, independently verifiable receipt.
fn build_receipt(
    content_hash: &[u8; 32],
    witnesses: Vec<WitnessAttestation>,
    solana_slot: Option<u64>,
    tx_sig: Option<String>,
    pda: Option<String>,
    coordinator_pubkey: &str,
) -> NotaryReceipt
```

**The key insight for hackathon:** To get witness signatures from hardware nodes WITHOUT changing firmware, we piggyback on the existing JobAssignment/CommitSubmission flow:

1. Send `JobAssignment { request_id: content_hash, ... }` to selected nodes
2. Node firmware treats it like a VRF round — generates entropy, commits SHA-256(entropy), signs the commit
3. We collect the `CommitSubmission` — the ECDSA signature over the commit_hash is our attestation
4. The commit_hash itself serves as a binding: node provably saw and processed this request_id (which IS the content_hash)

This means the signature isn't directly over the content_hash but over SHA-256(entropy). However, the node's participation in a round keyed by the content_hash, with its ECDSA-signed commit, constitutes attestation that:
- This node was online at this time
- This node processed a request containing this content_hash
- The signature is from a hardware-bound key

For production (post-hackathon), we'd add a new message type `NotarizeRequest` where the node signs the content_hash directly.

---

### Step 2.2 — Notary API endpoint
**File:** `coordinator/src/api/routes.rs` (ADD)

```
POST /notarize              — Submit a document hash for attestation
GET  /notarize/:id          — Get receipt by ID
GET  /notarize/history      — Recent attestations (last 50)
GET  /notarize/verify/:id   — Verify a receipt (re-check signatures)
```

**Request body for POST /notarize:**
```json
{
  "hash": "a1b2c3d4e5f6...",          // 64-char hex (32 bytes SHA-256)
  "hash_algorithm": "sha256",          // optional, defaults to sha256
  "metadata": {                         // optional
    "description": "Contract v2.1",
    "type": "legal"
  },
  "store_on_chain": false               // optional, defaults to false
}
```

**Response (NotaryReceipt):**
```json
{
  "version": "1.0",
  "type": "dice-notary-receipt",
  "receipt_id": "uuid",

  "attestation": {
    "content_hash": "sha256:a1b2c3d4e5f6...",
    "hash_algorithm": "sha256",
    "timestamp_unix": 1712534400,
    "timestamp_iso": "2026-04-07T12:00:00Z",
    "solana_slot": null,
    "solana_signature": null,
    "attestation_pda": null
  },

  "witnesses": [
    {
      "device_pubkey": "02abcdef...",
      "signature": "3045022100...",
      "commit_hash": "sha256:..."
    }
  ],
  "witness_count": 5,
  "threshold": 4,

  "protocol": {
    "network": "solana-devnet",
    "coordinator": "CoordPubkey..."
  },

  "verification": {
    "instructions": "To verify: (1) Confirm each witness signature is valid ECDSA over commit_hash using device_pubkey. (2) Confirm device_pubkey is registered in the DICE DeviceRegistry. (3) If on-chain, query the attestation PDA for matching data."
  }
}
```

---

### Step 2.3 — Database tables for notary
**File:** `coordinator/src/db/schema.sql` (APPEND)

```sql
-- Notary attestations
CREATE TABLE IF NOT EXISTS notary_attestations (
    id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    content_hash   BYTEA NOT NULL,
    hash_algorithm TEXT NOT NULL DEFAULT 'sha256',
    metadata       JSONB,
    witness_count  SMALLINT NOT NULL,
    threshold      SMALLINT NOT NULL,
    tx_signature   TEXT,                -- Solana tx sig (if stored on-chain)
    attestation_pda TEXT,               -- PDA address (if stored on-chain)
    receipt_json   JSONB NOT NULL,      -- full receipt for retrieval
    created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_notary_hash ON notary_attestations(content_hash);
CREATE INDEX IF NOT EXISTS idx_notary_time ON notary_attestations(created_at DESC);
```

**File:** `coordinator/src/db/queries.rs` (ADD)

```rust
pub async fn create_notary_attestation(pool: &PgPool, ...) -> Result<Uuid>
pub async fn get_notary_attestation(pool: &PgPool, id: Uuid) -> Result<Option<NotaryRow>>
pub async fn get_notary_history(pool: &PgPool, limit: i64) -> Result<Vec<NotaryRow>>
```

---

### Step 2.4 — Notary metrics
**File:** `coordinator/src/metrics.rs` (ADD)

```rust
pub notary_attestations_total: Counter,
pub notary_attestation_latency_seconds: Histogram,
pub notary_witnesses_per_attestation: Histogram,
```

---

### Step 2.5 — Dashboard update for notary
**File:** `coordinator/src/api/routes.rs` — Update `DASHBOARD_HTML`

Add a "Notary" section to the dashboard:
- Total attestations
- Recent attestations table: receipt_id, content_hash (truncated), witness_count, timestamp
- "Try Notarize" button (submits a test hash, shows the receipt)

---

### Step 2.6 — Tests for notary
**File:** `coordinator/src/notary.rs` — `#[cfg(test)] mod tests`

```rust
#[test] fn test_build_receipt_structure()
#[test] fn test_receipt_contains_all_required_fields()
#[test] fn test_receipt_is_valid_json()
#[test] fn test_witness_count_meets_threshold()
#[test] fn test_invalid_hash_rejected()
#[test] fn test_history_ring_buffer()
```

---

## Phase 3: Integration + Polish (Day 4)

### Step 3.1 — Wire everything together in main.rs

```rust
mod keeper;   // ADD
mod notary;   // ADD
```

Update `AppState` to include `keeper_state` and `notary_state`.
Update `build_router` to add keeper and notary routes.
Spawn keeper loop in main.

---

### Step 3.2 — Integration test: full lifecycle

```
1. Start coordinator with --simulation --keeper-enabled
2. Start 7 mock nodes
3. POST /simulate → verify VRF round completes (UNCHANGED)
4. POST /keeper/tasks → register demo-counter-crank task
5. Wait 15 seconds → verify GET /keeper/history shows executions
6. POST /notarize → submit test hash
7. Wait for response → verify receipt has ≥4 witness signatures
8. GET /notarize/:id → verify receipt retrieval
9. All 162+ existing tests still pass
```

---

### Step 3.3 — Dashboard: unified view

Update the dashboard banner from "v2.0 CHANNEL DESIGN" to "v5 MULTI-SERVICE":
```
DICE Coordinator — Multi-Service Platform
VRF | Keeper | Notary
```

Three tabs or sections:
1. **VRF** — existing dashboard (unchanged)
2. **Keeper** — tasks, executions, stats
3. **Notary** — attestations, receipts

---

### Step 3.4 — Ready banner update

```
DICE Coordinator ready (v5 — Multi-Service):
  Dashboard : http://localhost:8080/
  WebSocket : wss://localhost:8443/
  Metrics   : http://localhost:9090/metrics
  VRF       : POST /simulate
  Keeper    : POST /keeper/tasks
  Notary    : POST /notarize
```

---

## Verification Checklist

After all phases complete:

| # | Check | Command |
|---|-------|---------|
| 1 | **Existing tests pass** | `cargo test --workspace` — all 162+ tests pass |
| 2 | **Cargo check clean** | `cargo check --workspace --message-format=short` — 0 errors |
| 3 | **VRF still works** | `POST /simulate` → round finalizes with randomness |
| 4 | **Keeper registers** | `POST /keeper/tasks` → returns task_id |
| 5 | **Keeper executes** | Wait 15s → `GET /keeper/history` shows successful executions |
| 6 | **Keeper dashboard** | Dashboard shows keeper stats + execution table |
| 7 | **Notary works** | `POST /notarize {"hash":"abc..."}` → returns receipt with witness sigs |
| 8 | **Notary receipt valid** | Receipt contains: version, witnesses, signatures, timestamps |
| 9 | **Notary retrieval** | `GET /notarize/:id` → returns stored receipt |
| 10 | **Dashboard unified** | Shows VRF + Keeper + Notary sections |
| 11 | **Real hardware** | Connect ESP32, VRF rounds complete while keeper runs in parallel |
| 12 | **No firmware changes** | `git diff firmware/` → empty |

---

## Files Created/Modified Summary

### New Files (5)
| File | Lines | Purpose |
|------|-------|---------|
| `coordinator/src/keeper.rs` | ~350 | Keeper loop, task management, trigger evaluation |
| `coordinator/src/notary.rs` | ~250 | Notarization handler, receipt generation |
| `programs/dice-keeper-demo/src/lib.rs` | ~80 | Demo counter program for keeper to crank |
| `programs/dice-keeper-demo/Cargo.toml` | ~15 | Anchor program manifest |
| `IMPLEMENTATION-v5-keeper-notary.md` | this file | Implementation procedure |

### Modified Files (7)
| File | Changes |
|------|---------|
| `coordinator/src/main.rs` | Add `mod keeper; mod notary;`, spawn keeper task, update AppState, update ready banner |
| `coordinator/src/config.rs` | Add keeper_enabled, keeper_interval_secs, keeper_max_concurrent flags |
| `coordinator/src/api/routes.rs` | Add AppState fields, keeper/notary routes, dashboard update |
| `coordinator/src/solana_tx.rs` | Add `execute_keeper_task()` function |
| `coordinator/src/db/schema.sql` | Add keeper_tasks, keeper_executions, notary_attestations tables |
| `coordinator/src/db/queries.rs` | Add keeper + notary query functions |
| `coordinator/src/metrics.rs` | Add keeper + notary Prometheus metrics |

### Untouched Files (critical)
| File | Why |
|------|-----|
| `firmware/*` | Zero firmware changes — nodes don't know about keeper/notary |
| `coordinator/src/state_machine.rs` | VRF state machine untouched |
| `coordinator/src/protocol/messages.rs` | CBOR protocol untouched |
| `coordinator/src/protocol/validation.rs` | Signature verification untouched |
| `coordinator/src/queue.rs` | VRF request queue untouched |
| `coordinator/src/solana_watcher.rs` | VRF account watcher untouched |
| `coordinator/src/solana_ws.rs` | Solana WebSocket subscriber untouched |
| `coordinator/src/selection.rs` | Node selection untouched |
| `programs/dice/src/*` | VRF smart contract untouched |

---

## Dependency Changes

### No new crate dependencies needed

All required functionality already exists in the workspace:
- `tokio` — async runtime, timers, spawn
- `serde` / `serde_json` — serialization
- `uuid` — receipt IDs
- `chrono` — timestamps
- `sqlx` — database
- `solana_sdk` — instruction building
- `k256` / `sha2` — crypto (existing)
- `hex` — encoding
- `axum` — API routes

The only potential addition is a cron parser crate if we want full cron expression support:
- `cron` = "0.12" — lightweight, no extra deps
- Or: just use `Interval` trigger for hackathon (simpler, no new dep)

**Recommendation:** Start with `Interval` only (no cron dep). Add cron parsing post-hackathon.

---

## Execution Order

```
Day 1:  Steps 1.1 → 1.5  (config, data structures, demo program, tx builder, DB schema)
Day 2:  Steps 1.6 → 1.10 (API endpoints, wiring, dashboard, metrics, tests)
Day 3:  Steps 2.1 → 2.6  (notary module, API, DB, metrics, dashboard, tests)
Day 4:  Steps 3.1 → 3.4  (integration, unified dashboard, verification)
```

Each step ends with a compilable, testable state. No step depends on a later step. If we run out of time, keeper alone (Phase 1) is a complete, demoable feature.

---

*Implementation procedure written 2026-04-07. Branch: v5-keeper-notary. Base: DICE v3/v4.*
