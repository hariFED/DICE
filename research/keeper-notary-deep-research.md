# DICE Keeper & Notary: Deep Research — Architecture, Security, UX

**Date:** 2026-04-07
**Scope:** Comprehensive research on keeper/automation networks and notary/timestamping systems, informing DICE's expansion from VRF-only to a multi-service hardware platform.
**Current state:** DICE v3 — 545+ VRF rounds on real ESP32-S3, 162 tests passing, 4 programs on devnet, mTLS + PostgreSQL working.

---

## Table of Contents

1. [Part 1: Keeper / Automation Networks](#part-1-keeper--automation-networks)
   - [1.1 Universal Keeper Pattern](#11-universal-keeper-pattern)
   - [1.2 Clockwork (Solana — Dead)](#12-clockwork-solana--dead)
   - [1.3 Tuk Tuk (Solana — Current)](#13-tuk-tuk-solana--current)
   - [1.4 Chainlink Automation (Ethereum — Gold Standard)](#14-chainlink-automation-ethereum--gold-standard)
   - [1.5 Gelato Network (Ethereum — Market Leader)](#15-gelato-network-ethereum--market-leader)
   - [1.6 Solana Transaction Landing](#16-solana-transaction-landing)
   - [1.7 Lessons for DICE Keeper](#17-lessons-for-dice-keeper)
2. [Part 2: Notary / Timestamping Systems](#part-2-notary--timestamping-systems)
   - [2.1 RFC 3161 — Traditional Timestamping](#21-rfc-3161--traditional-timestamping)
   - [2.2 OpenTimestamps — Bitcoin-Anchored](#22-opentimestamps--bitcoin-anchored)
   - [2.3 Solana Attestation Service (SAS)](#23-solana-attestation-service-sas)
   - [2.4 Multi-Witness Attestation Patterns](#24-multi-witness-attestation-patterns)
   - [2.5 On-Chain Notarization on Solana](#25-on-chain-notarization-on-solana)
   - [2.6 Security Considerations](#26-security-considerations)
   - [2.7 Receipt/Proof Format](#27-receiptproof-format)
   - [2.8 Lessons for DICE Notary](#28-lessons-for-dice-notary)
3. [Part 3: DICE Keeper Design](#part-3-dice-keeper-design)
4. [Part 4: DICE Notary Design](#part-4-dice-notary-design)
5. [Part 5: Coordinator Architecture — Integration Points](#part-5-coordinator-architecture--integration-points)
6. [Sources](#sources)

---

## Part 1: Keeper / Automation Networks

### 1.1 Universal Keeper Pattern

Every keeper system — Clockwork, Tuk Tuk, Chainlink, Gelato — follows the same core loop:

```
1. Developer REGISTERS a task    (what to execute + when to trigger)
2. Developer FUNDS the task      (escrow SOL / LINK / ETH / GEL)
3. Network MONITORS triggers     (off-chain or on-chain condition check)
4. When triggered → EXECUTE      (submit transaction on-chain)
5. Executor gets PAID            (fee deducted from escrow)
6. Repeat until cancelled or out of funds
```

**Trigger types** (consistent across all 4 systems):

| Trigger | Description | Example |
|---------|-------------|---------|
| **Time / Cron** | Execute on a schedule | "every 10 seconds", "Tuesday 3pm", cron expressions |
| **Account / State change** | Execute when on-chain data changes | Clockwork watched byte offsets, Tuk Tuk uses bitmaps |
| **Custom logic** | Arbitrary off-chain condition | Chainlink's `checkUpkeep()` — runs off-chain, gas-free |
| **Event / Log** | React to emitted on-chain events | Chainlink log triggers |
| **Immediate** | Execute once, right now | Clockwork "Now" trigger, Tuk Tuk "now" trigger |

**Payment models across systems:**

| System | Currency | Model | Cost per execution |
|--------|----------|-------|--------------------|
| **Clockwork** | SOL | Balance in Thread PDA | ~5,000 lamports |
| **Tuk Tuk** | SOL | Per-crank reward set by queue creator | ~5,000 lamports (~2x normal tx) |
| **Chainlink** | LINK | Deposited in Automation Registry | Gas + premium |
| **Gelato** | GEL / stablecoins | Deposited to Gelato balance | Gas + protocol fee |

---

### 1.2 Clockwork (Solana — Dead)

**Raised:** $4M from Multicoin Capital, Solana Ventures, Asymmetric VC
**Died:** August 2023 (infra shutdown Oct 31, 2023)
**Reason:** "Limited commercial upside"

#### Architecture

Clockwork was a **Solana Geyser plugin** installed on validators/RPC nodes. The plugin monitored user-defined "Threads" and executed their instructions when trigger conditions were met.

**Threads** were on-chain PDA accounts containing:
- A **trigger** definition (account-based or time-based)
- A set of **instructions** to execute (static or dynamically generated via CPI)
- A **SOL balance** used to pay execution fees

**Trigger types:**
- **Account-based:** Tracked specific byte offsets in an on-chain account. Developer specified: target account address, byte offset (e.g., offset 8 to skip discriminator), and data size to monitor.
- **Time-based:** Cron expressions (e.g., `"*/10 * * * * * *"` for every 10 seconds), slot-based intervals, or epoch-based timing.

#### Execution flow

```
1. Developer creates Thread PDA via CPI (from their Anchor program) or TypeScript SDK
   - Specifies: trigger, instructions, initial SOL funding
   - Thread authority = PDA derived from developer's program

2. Geyser plugin on validator detects trigger condition:
   - Account trigger: watched byte range changes
   - Time trigger: cron schedule matches current time

3. Plugin submits transaction executing the Thread's instructions
   - Thread authority PDA signs the transaction
   - SOL deducted from Thread balance for fees

4. If Thread has SOL remaining → re-arm for next trigger
   If Thread out of SOL → becomes inactive
```

#### Developer integration

```rust
// Rust (from on-chain program via CPI)
clockwork_sdk::cpi::thread_create(
    ctx,
    lamports_for_fees,
    thread_id,
    instructions_to_execute,
    Trigger::Account {
        address: target_account.key(),
        offset: 8,
        size: 1,
    },
)?;
```

```typescript
// TypeScript
const thread = await clockworkProvider.threadCreate(
  authority, id, instructions, trigger, amount
);
```

#### Why it failed — lessons

1. **Thin margins:** Per-crank fees (~5,000 lamports = ~$0.001) barely covered infrastructure costs
2. **Validator dependency:** Required Geyser plugin installation on validators — hard to get adoption
3. **No token flywheel:** No staking/reward token to subsidize early growth
4. **Standalone product:** Keepers alone don't generate enough revenue to sustain a company
5. **Centralization concern:** Geyser plugin approach meant execution was tied to specific validators

**Key lesson for DICE:** A standalone keeper product struggles to monetize. Bundling keeper with VRF revenue (same $8 device, combined revenue streams) is exactly what Clockwork couldn't do.

---

### 1.3 Tuk Tuk (Solana — Current)

**Builder:** Helium engineering team (Noah Prince)
**Status:** Active, launched at Accelerate 2025
**Repo:** github.com/helium/tuktuk

#### Architecture

Tuk Tuk is a **pure on-chain program** (no Geyser plugin). Uses **PDAs + bitmaps** for task management.

```
Task Queue (PDA)
  ├── capacity: max simultaneous tasks
  ├── funding: SOL balance for operations
  ├── crank_reward: lamports per execution
  ├── queue_authority: who can queue tasks
  └── bitmap: [u8; N] — each bit = one task slot (1=active, 0=inactive)

Task (PDA, derived from Queue)
  ├── trigger: when to execute
  ├── transaction: compiled instructions (CompileV0) OR remote URL (RemoteV0)
  └── state: queued / executing / completed / failed
```

**Bitmap system:** Each bit represents a task. Crankers scan bitmaps to find executable tasks — O(N/8) bytes to scan instead of loading N full accounts. Dramatically reduces compute overhead.

#### Task types

| Type | How it works | Use case |
|------|-------------|----------|
| **CompileV0** | Instructions compiled ahead of time, stored on-chain | Simple cron jobs, recurring token transfers |
| **RemoteV0** | HTTP POST to external server → returns base64 transaction | Oracle interactions, dynamic instruction generation, off-chain logic |

**Remote transactions** (RemoteV0): The external server receives `{task_key, queue_key, timestamp}`, returns a signed transaction. The on-chain program verifies the cryptographic signature to trust the response.

#### Cron / Recurring tasks

Tuk Tuk uses **recursive task returns** — a task executes, then re-queues itself for the next interval:
```
Execute task (Monday 10am)
  → Perform action
  → Re-queue self for next Monday 10am
  → Task stays in queue bitmap
```

#### Cranker model

**Permissionless:** Anyone with an RPC URL can run a cranker:
```bash
tuktuk-crank-turner --solana-url <RPC> --keypair-path <KEY> --min-crank-fee 1000
```

No staking, no slashing, no Geyser plugin. Just a Rust binary that:
1. Polls task queue bitmaps
2. Finds tasks past their trigger time
3. Builds + submits execution transaction
4. Gets paid the crank reward in SOL

**Config:** Min 1 SOL deposit per queue (anti-spam, refundable on close). Queue creator sets per-crank reward. Crankers filter by minimum fee threshold.

#### Developer integration

```bash
# CLI
tuktuk -u <rpc> task-queue create \
  --name my-queue \
  --capacity 10 \
  --funding-amount 100000000 \
  --queue-authority <pubkey> \
  --crank-reward 1000000
```

```typescript
// TypeScript SDK
import { init, queueTask } from "@helium/tuktuk-sdk";
const program = await init(provider);
await queueTask(program, { queue, trigger: "now", transaction });
```

```rust
// Rust CPI (from on-chain program)
tuktuk_sdk::cpi::queue_task_v0(ctx, QueueTaskV0Args {
    trigger: Trigger::Now,
    transaction: compiled_tx,
    crank_reward: None, // use queue default
})?;
```

#### Security model

- **Queue authority:** Controls who can queue tasks. "Should not be given out blindly — authority can queue tasks that use up task queue funding and use custom signers."
- **Signature verification:** Remote transaction servers must cryptographically sign instruction payloads
- **No slashing:** Permissionless model — crankers just don't get paid if they don't execute. No penalty for going offline.
- **Retry mechanism:** Configurable retries. Tasks abandoned after exhausting retries need manual `task run` CLI command.

#### Limitations

- No execution guarantees (permissionless = best-effort)
- No MEV protection
- No hardware attestation
- Cranker quality varies (anyone with an RPC)
- Queue authority is a trust bottleneck

---

### 1.4 Chainlink Automation (Ethereum — Gold Standard)

The most elegant keeper design pattern in production.

#### The checkUpkeep / performUpkeep Pattern

```solidity
// Off-chain: runs every block, gas-free
function checkUpkeep(bytes calldata checkData)
    external view returns (bool upkeepNeeded, bytes memory performData);

// On-chain: runs only when triggered, costs gas
function performUpkeep(bytes calldata performData) external;
```

**Why this is brilliant:**
1. `checkUpkeep` runs **off-chain** (simulated as `eth_call`) — complex condition checks happen for free
2. It can read on-chain state, do math, encode results into `performData`
3. `performUpkeep` receives pre-computed data — minimal on-chain computation
4. Heavy work off-chain (free) → minimal execution on-chain (paid)

#### Execution flow

```
1. Developer deploys contract implementing checkUpkeep + performUpkeep
2. Developer registers contract as "Upkeep" in Automation Registry
   - Specifies: trigger type, gas limit, initial LINK funding
3. Chainlink nodes simulate checkUpkeep() every block
4. When checkUpkeep returns true:
   - Node calls performUpkeep(performData) on-chain
   - Gas cost + premium deducted from Upkeep's LINK balance
5. Cycle repeats until cancelled or out of LINK
```

#### Trigger types

| Type | How it works |
|------|-------------|
| **Custom Logic** | `checkUpkeep()` contains arbitrary condition logic |
| **Time-Based** | Execute at cron-like intervals |
| **Log Trigger** | React to specific EVM log events emitted by contracts |

#### Security considerations (from Chainlink docs)

1. **`performUpkeep` MUST validate all inputs** — anyone can call it, not just Chainlink nodes
2. **Implement access controls** — restrict sensitive operations to Automation Registry address
3. **Keep `checkUpkeep` deterministic** — non-deterministic checks cause execution failures
4. **Set appropriate gas limits** — too low = reverts, too high = wasted funds
5. **Monitor funding balance** — continuous operation requires sufficient LINK

#### Payment model

- LINK token deposited in Automation Registry
- Per-upkeep: gas cost + protocol premium
- Supports LINK and native currency (ETH)
- Auto-top-up available

---

### 1.5 Gelato Network (Ethereum — Market Leader)

**Market share:** 78% of Ethereum smart contract automation
**Token:** GEL (governance + staking)

#### Architecture

**Executor slot model:** Gelato organizes executors into time slots. Each executor gets **exclusive right** to execute tasks during their slot. This prevents execution races and ensures accountability.

```
Time Slot 1: Executor A has exclusive execution rights
Time Slot 2: Executor B has exclusive execution rights
...
If Executor A misses tasks in Slot 1 → kicked from network
```

**Resolver/Checker pattern** (similar to Chainlink):
```javascript
// Off-chain: called every block
function checker() returns (bool canExec, bytes execPayload);

// On-chain: executed when canExec = true
function exec(bytes execPayload);
```

#### Trigger types

| Type | Description |
|------|-------------|
| **Time Interval** | Every N minutes/hours |
| **Cron Expression** | "Every Tuesday at 3 PM" |
| **On-Chain Event** | React to specific blockchain events |
| **Every Block** | Execute with each new block |

#### Security and staking

- **GEL staking:** Executors must stake GEL to get execution slots
- **Slashing (planned):** For censoring transactions, front-running, or going offline
- **Slot exclusivity:** Prevents execution races — clear accountability per time window
- **Kick mechanism:** If executor misses tasks in their slot → removed from network

#### Payment model

- Deposit funds to Gelato balance (stablecoins or native token)
- Executors claim fees per execution (gas + premium)
- Supports fee abstraction (protocol pays, not end user)

---

### 1.6 Solana Transaction Landing

Critical knowledge for any keeper system on Solana.

#### Priority fees

- Help validators prioritize during congestion
- Within each queue: transactions ranked by priority fee + arrival time
- **Higher fee ≠ guaranteed first execution**, but increases chances
- 100% of priority fee goes to validator (proposal passed, activating 2025)

#### Jito tips and bundles

- **Bundle:** Up to 5 transactions that execute **sequentially and atomically** within a single slot
- If any transaction fails → whole bundle discarded
- **Minimum tip:** 1,000 lamports
- Bundles go through Jito infrastructure → sequenced to prevent MEV extraction
- Best for: time-critical executions (liquidations, arbitrage)

#### Stake-weighted QoS (swQoS)

- **Most effective** mechanism for reducing transaction latency
- Trusted, stake-weighted connections between RPCs and validators
- Materially reduces time to inclusion
- Not directly controllable by application developers — depends on RPC provider

#### Best practices for keeper bots

1. **Always estimate compute units** — simulate transaction first, request only what's needed
2. **Use Jito bundles** for MEV-sensitive or time-critical executions
3. **Standard priority fees** for routine cron jobs
4. **Reliable RPC** with swQoS support (Helius, Triton) matters more than tip size
5. **Retry with backoff** — don't spam the same transaction

---

### 1.7 Lessons for DICE Keeper

| Lesson | Source | How DICE applies it |
|--------|--------|---------------------|
| Standalone keeper ≠ viable business | Clockwork death | Bundle with VRF revenue — same device, multiple streams |
| Bitmap task scanning is efficient | Tuk Tuk | Consider bitmap-style task indexing on-chain |
| checkUpkeep/performUpkeep split is elegant | Chainlink | Coordinator does "check" off-chain, on-chain program does "perform" |
| Permissionless cranking = no guarantees | Tuk Tuk | Hardware fleet = execution SLA (heartbeat-verified uptime) |
| Execution slot model = accountability | Gelato | Assign tasks to specific nodes, track execution per-node |
| Remote transactions enable dynamic logic | Tuk Tuk | Support compiled (static) + remote (dynamic) task types |
| Recursive re-queuing = cron | Tuk Tuk | Tasks re-queue themselves after execution |
| Validate performUpkeep inputs | Chainlink | On-chain program must validate even coordinator-submitted txs |
| Jito bundles for time-critical | Solana infra | Integrate Jito for liquidation/MEV-sensitive executions |
| Fund monitoring is essential | All systems | Dashboard must show task funding balance, warn when low |

**DICE's unique advantage:** Hardware-attested execution. No other keeper network has:
- Dedicated hardware nodes with Secure Boot + flash encryption
- Heartbeat-verified uptime (coordinator knows node liveness in real-time)
- ECDSA-signed execution receipts from hardware-bound keys
- Economic bundling (same $8 device earns from VRF + keeper + notary)

---

## Part 2: Notary / Timestamping Systems

### 2.1 RFC 3161 — Traditional Timestamping

The legal standard for digital timestamps. Defined in [RFC 3161](https://datatracker.ietf.org/doc/html/rfc3161).

#### How it works

```
1. Client hashes document → sends hash to Time Stamping Authority (TSA)
   (TSA never sees the original document — only its hash)

2. TSA creates TimeStampToken containing:
   - messageImprint: echoed hash algorithm + hash value
   - genTime: GeneralizedTime in UTC
   - accuracy: seconds + milliseconds + microseconds
   - serialNumber: unique integer (prevents token substitution)
   - nonce: echoed from request (replay protection)
   - TSA identity + certificate chain

3. TSA signs the TimeStampToken with its private key (CMS SignedData)

4. Client stores the signed token as proof of existence at that time
```

#### Security guarantees

- **Proof of existence:** Data provably existed before the timestamp
- **Non-repudiation:** TSA's signature binds hash to time
- **Integrity:** Any modification to timestamped data is detectable
- **Ordering:** When `ordering=TRUE`, strict sequencing guaranteed
- **Replay protection:** Nonce ensures fresh responses

#### Legal acceptance

TSA certificates must contain `id-kp-timeStamping` as sole extended key usage. Accepted under:
- eIDAS (EU)
- ESIGN Act (US)
- Various national electronic signature laws

**Critical limitation:** Single point of trust. If the TSA is compromised or key leaked, all timestamps become suspect. No built-in mechanism for multi-authority attestation.

---

### 2.2 OpenTimestamps — Bitcoin-Anchored

[OpenTimestamps](https://opentimestamps.org/) (Peter Todd, 2016) creates trust-minimized proofs anchored in Bitcoin.

#### How it works

```
1. Client computes SHA-256(file), concatenates random 128-bit nonce, re-hashes
2. Submits to calendar servers (5 independent operators)
3. Calendar servers aggregate many hashes into a Merkle tree
   (10,000 files → 1 Bitcoin transaction)
4. Merkle root embedded in a Bitcoin transaction
5. Once confirmed: block header's nTime = timestamp authority
```

#### Proof format (.ots file)

An **operation tree** (not a linear list):
- Root = message hash
- Edges = commitment operations (`sha256`, `append(data)`, `prepend(data)`, `ripemd160`)
- Leaves = attestations (`PendingAttestation` or `BitcoinBlockHeaderAttestation`)

#### Verification

```
1. Hash original file
2. Load .ots proof file
3. Apply each commitment operation in sequence
4. Final result should match a Bitcoin block header's merkle root
5. Verify block header exists on Bitcoin blockchain (any Bitcoin node)
```

#### Limitations

- **Timestamp accuracy:** Bitcoin block timestamps accurate to ~2-3 hours
- **Confirmation latency:** ~1 hour minimum (6 confirmations for high confidence)
- **No identity binding:** Cannot prove who created data, only that it existed
- **Calendar dependency:** Incomplete timestamps need calendar availability to upgrade

---

### 2.3 Solana Attestation Service (SAS)

Built by Range Security. Available at [attest.solana.com](https://attest.solana.com/).

#### Architecture

Three-party model:
- **Issuers:** Create attestation schemas + issue attestations
- **Holders:** Entities receiving attestations
- **Verifiers:** Anyone checking attestation validity

Schema-driven — register an attestation schema, then issue attestations against it. Supports revocation.

#### Conceptual predecessor: Ethereum Attestation Service (EAS)

- Two contracts: `SchemaRegistry.sol` + `EAS.sol`
- Schemas use Solidity ABI types
- Supports on-chain and off-chain attestations
- Revocation, referenced attestations (chains), resolver contracts
- EIP-712 for typed data signing

---

### 2.4 Multi-Witness Attestation Patterns

#### Single Authority vs. Multi-Party

| Property | Single Authority (RFC 3161) | Multi-Witness (DICE) |
|----------|----------------------------|----------------------|
| Trust model | Trust one TSA completely | Trust threshold of witnesses |
| Single point of failure | Yes | No (BFT) |
| Compromise impact | All timestamps suspect | System survives up to t compromised witnesses |
| Cost | One signature verification | Multiple verifications |
| Latency | One round trip | Coordination overhead |

#### Signature aggregation options for ECDSA

DICE uses secp256k1 ECDSA from ESP32 hardware. Unlike BLS or Schnorr, ECDSA has no efficient non-interactive aggregation:

| Approach | Pros | Cons |
|----------|------|------|
| **Concatenated signatures** | Simplest. Each sig independently verifiable. | Receipt size grows linearly with witness count. |
| **Coordinator-attested receipt** | Compact. Coordinator verifies all sigs, signs summary. | Reintroduces trusted party. |
| **Merkle tree of signatures** | Hash all sigs → Merkle root on-chain. Merkle proofs off-chain. | Balances cost vs. verifiability. |
| **Future: Schnorr/MuSig2** | Native multi-sig aggregation (Bitcoin Taproot uses this). | Requires firmware change. |

**DICE already has multi-witness attestation.** The VRF DiceChannel stores per-node:
```rust
device_pubkeys: Vec<[u8; 33]>   // witness identities
commit_hashes:  Vec<[u8; 32]>   // commitments
entropies:      Vec<[u8; 32]>   // revealed values
signatures:     Vec<[u8; 64]>   // ECDSA attestations
```

Each ESP32 independently generates entropy, commits (SHA-256), signs, then reveals. The coordinator verifies each witness. This IS a multi-witness attestation scheme.

---

### 2.5 On-Chain Notarization on Solana

#### PDA structure for attestations

```rust
// Attestation PDA: ["attestation", schema_id, content_hash]
pub struct Attestation {
    pub schema_id: [u8; 32],         // SHA-256 of schema definition
    pub content_hash: [u8; 32],      // SHA-256 of attested data
    pub attester: Pubkey,            // coordinator pubkey
    pub witnesses: Vec<[u8; 33]>,    // compressed device pubkeys
    pub witness_sigs: Vec<[u8; 64]>, // ECDSA signatures
    pub witness_count: u8,
    pub threshold: u8,               // minimum witnesses required
    pub timestamp: i64,              // Clock::unix_timestamp
    pub slot: u64,                   // Solana slot number
    pub status: u8,                  // 0=active, 1=revoked
}
```

#### Cost analysis (4 witnesses)

| Component | Bytes |
|-----------|-------|
| Discriminator | 8 |
| schema_id | 32 |
| content_hash | 32 |
| attester (Pubkey) | 32 |
| witnesses (4 x 33 + 4 prefix) | 136 |
| witness_sigs (4 x 64 + 4 prefix) | 260 |
| witness_count + threshold + status | 3 |
| timestamp + slot | 16 |
| **Total** | **~523 bytes** |

Rent-exempt minimum: `(128 + 523) * 6,960 = ~0.00453 SOL` (~$0.68 at $150/SOL)

#### Cost optimization

- Store only hash + metadata on-chain (~100-150 bytes → ~0.002 SOL)
- Keep full receipt with all witness sigs off-chain
- Compress witness signatures into Merkle root (32 bytes vs N*64 bytes)

#### Solana's timestamp authority

`Clock::unix_timestamp` is computed by a **validator timestamp oracle**:
- Validators include observed UTC time in Vote instructions
- System computes **stake-weighted mean** of validator timestamps
- Accurate to within seconds under normal conditions

`Clock::slot` is a **monotonically increasing counter** — strict ordering even when unix timestamps are imprecise.

**For notarization: use both.** `Clock::unix_timestamp` for human-readable time, `Clock::slot` for ordering. Slot is the more reliable ordering primitive.

---

### 2.6 Security Considerations

#### Backdating prevention

| Threat | Mitigation |
|--------|-----------|
| Fake timestamp for past data | Blockchain anchoring — hash in Solana slot is append-only, publicly verifiable |
| Tamper with witness order | Slot-based deadlines — Solana slots are consensus-determined and monotonic |
| Compromise single witness | Threshold requirement — min 4 witnesses (BFT: tolerates 1 compromised with 3f+1) |
| Extract hardware key | Secure Boot + flash encryption + NVS encryption — key never leaves device |
| Clock drift between nodes | Coordinator manages deadlines via Solana slots (consensus time), not node wall clocks |

#### Hardware-backed vs. software-only trust

| Property | Software-only (server + key file) | Hardware-backed (ESP32 + NVS) |
|----------|-----------------------------------|-------------------------------|
| Key extraction | Trivial with OS access | Difficult (flash readout protection) |
| Remote compromise | Full key compromise via exploit | Cannot extract key remotely |
| Physical tamper | No protection | Secure Boot detects modification |
| Key provenance | Could be copied anywhere | Generated on-device, never leaves |
| Attestation binding | Signature could come from any machine | Signature from specific physical device |

**The fundamental difference:** Hardware attestation proves a **specific physical device** performed the signing. Software attestation only proves someone with a copy of the key signed. For notarization, this distinction matters legally.

---

### 2.7 Receipt/Proof Format

#### What a notary receipt must contain

```json
{
  "version": "1.0",
  "type": "dice-notary-receipt",
  
  "attestation": {
    "content_hash": "sha256:abcdef1234...",
    "hash_algorithm": "sha256",
    "timestamp_unix": 1712534400,
    "timestamp_iso": "2026-04-08T00:00:00Z",
    "solana_slot": 298765432,
    "solana_signature": "5K8vT2...",
    "attestation_pda": "7xKQ3..."
  },
  
  "witnesses": [
    {
      "device_pubkey": "02abcdef...",
      "device_id": "sha256:...",
      "signature": "r_hex...s_hex...",
      "commit_hash": "sha256:...",
      "entropy": "hex..."
    }
  ],
  
  "protocol": {
    "threshold": 4,
    "witnesses_total": 5,
    "round_id": 42,
    "network": "solana-mainnet",
    "program_id": "DICE...",
    "coordinator": "CoordPubkey..."
  },
  
  "verification": {
    "instructions": "To verify: (1) SHA-256 your document, compare to content_hash. (2) For each witness, verify ECDSA(device_pubkey, signature, commit_hash). (3) Verify SHA-256(entropy) == commit_hash. (4) Query Solana for the attestation PDA.",
    "solana_rpc": "https://api.mainnet-beta.solana.com"
  }
}
```

#### Serialization recommendation

| Format | Best for |
|--------|----------|
| **CBOR** | Canonical storage (DICE already uses CBOR on wire protocol) |
| **JSON** | API responses, developer consumption, human-readable display |

**Approach:** CBOR as canonical format, JSON as display format. Follows SCITT (IETF Supply Chain Integrity) patterns — receipts are COSE_Sign1 messages.

#### Independent verifiability

The receipt MUST be self-contained:
1. Include all witness public keys (no lookup needed)
2. Include Solana transaction signature (confirmable via any RPC)
3. Include attestation PDA address (direct account lookup)
4. Include program ID (verify PDA derivation)
5. Include verification instructions (usable even if DICE goes offline)

---

### 2.8 Lessons for DICE Notary

| Lesson | Source | How DICE applies it |
|--------|--------|---------------------|
| Single TSA = single point of failure | RFC 3161 | Multi-witness hardware attestation |
| Merkle aggregation scales timestamps | OpenTimestamps | Batch multiple attestations into Merkle tree |
| Schema-driven attestations are flexible | SAS/EAS | Support attestation schemas for different use cases |
| Receipts must be independently verifiable | All systems | Self-contained receipt format with verification instructions |
| On-chain storage is expensive | Solana rent | Store hash on-chain, full receipt off-chain |
| Slot numbers are better than timestamps for ordering | Solana clock | Record both slot + timestamp, use slot for proofs |
| Hardware attestation > software attestation | Security analysis | Each sig from Secure Boot-verified device — legal-grade |
| CBOR is the right wire format | SCITT/IETF | Already using CBOR — extend to receipts |

---

## Part 3: DICE Keeper Design

Based on all research, here's how DICE Keeper should work:

### Developer flow

```
1. Developer deploys their program with a "crankable" instruction
   (any Solana instruction that needs periodic execution)

2. Developer registers a task with DICE:
   POST /keeper/tasks {
     target_program: "ProgramId...",
     instruction_data: "base64...",
     accounts: [...],
     trigger: { type: "cron", schedule: "*/10 * * * * *" },
     funding_amount: 1_000_000_000  // 1 SOL
   }

3. DICE Coordinator evaluates triggers off-chain:
   - Cron: check if schedule matches current time
   - Account: poll for state changes
   - (Like Chainlink's checkUpkeep — off-chain, gas-free)

4. When triggered:
   - Coordinator builds transaction with developer's instruction
   - Signs with coordinator keypair
   - Submits to Solana (with priority fees / Jito tips as needed)
   - Records execution receipt (tx sig, slot, timestamp, latency)

5. Fee deducted from developer's escrow
   - Per-execution: ~5,000 lamports (competitive with Tuk Tuk)
   - Dashboard shows: execution history, success rate, balance remaining
```

### Security model

```
ON-CHAIN VALIDATION:
- Keeper program validates: correct authority, sufficient funding, valid trigger
- Execution receipts logged on-chain (tx signature, slot)
- Escrow PDA holds developer funds — only coordinator can deduct per execution

COORDINATOR:
- Hardware node heartbeats prove network liveness
- Signed execution receipts from coordinator keypair
- Jito bundles for MEV-sensitive executions (liquidations)
- Retry with backoff on failed transactions

MONITORING:
- Dashboard: task status, execution history, funding balance
- Alerts: low balance, failed executions, missed triggers
- Prometheus metrics: execution latency, success rate, tx landing rate
```

### Architecture (parallel to VRF)

```
coordinator/src/
  ├── main.rs              ← spawn keeper task alongside VRF
  ├── keeper.rs            ← NEW: trigger evaluation + execution loop
  ├── state_machine.rs     ← UNTOUCHED: VRF state machine
  ├── solana_rpc.rs        ← SHARED: sign_and_send
  └── api/routes.rs        ← ADD: /keeper/* endpoints
```

The keeper loop runs as an independent `tokio::spawn` task. It shares `OnChainCtx` for Solana access but has **ZERO interaction** with the commit-reveal state machine.

---

## Part 4: DICE Notary Design

Based on all research, here's how DICE Notary should work:

### User flow

```
1. User submits document hash:
   POST /notarize {
     hash: "sha256:abc123...",
     metadata: { description: "Contract v2.1", type: "legal" }  // optional
   }

2. Coordinator dispatches hash to N connected hardware nodes
   (Reuses existing node selection + CBOR message dispatch)

3. Each ESP32 node signs the hash with its hardware-bound ECDSA key
   (Firmware doesn't know the difference — it's just signing 32 bytes)

4. Coordinator collects signatures, verifies each one
   (Reuses existing signature verification logic)

5. Optional: writes attestation PDA on-chain
   (Hash + metadata + witness count + slot + timestamp)

6. Returns receipt:
   {
     receipt_id: "uuid",
     content_hash: "sha256:abc123...",
     witnesses: [{ pubkey, signature, commit_hash }...],
     solana_slot: 298765432,
     timestamp: "2026-04-07T...",
     attestation_pda: "7xKQ3..." // if on-chain
   }

7. Anyone can verify independently:
   - Check each ECDSA signature against registered device pubkeys
   - Confirm on-chain PDA exists and matches
   - Verify Solana slot timestamp
```

### Security model

```
ATTESTATION INTEGRITY:
- Each signature from Secure Boot-verified ESP32 with hardware-bound key
- Minimum 4 witnesses (BFT: tolerates 1 compromised)
- Solana slot provides monotonic timestamp (cannot backdate)
- Receipt is self-contained — verifiable even if DICE service goes offline

ANTI-TAMPERING:
- Coordinator cannot forge witness signatures (doesn't have device keys)
- Nodes cannot collude without detection (independent entropy + signatures)
- On-chain PDA is immutable once created
- Slot ordering is consensus-determined

PRIVACY:
- Only hash is submitted — original document never leaves user
- Hash is one-way — cannot reverse to get original document
- Witness nodes see only the hash, never the document
```

### Architecture (rides on existing VRF pipeline)

```
coordinator/src/
  ├── notary.rs            ← NEW: attestation handler (~200 lines)
  ├── api/routes.rs        ← ADD: POST /notarize endpoint
  ├── main.rs              ← Wire notary handler
  ├── protocol/messages.rs ← UNTOUCHED: nodes just sign 32 bytes
  └── state_machine.rs     ← UNTOUCHED: VRF state machine
```

**Key insight:** For hackathon, notary rides on the existing VRF pipeline. Send a `JobAssignment` with the document hash as the `request_id`. The node's commit (ECDSA signature over the hash) IS the attestation. **No firmware changes. No new message types.**

---

## Part 5: Coordinator Architecture — Integration Points

Based on thorough codebase analysis, here's how both features plug in:

### Current architecture (relevant pieces)

```
AppState (shared across all handlers):
  ├── registry: Arc<RwLock<HashMap<[u8;33], NodeSession>>>  // connected nodes
  ├── rounds: Arc<Mutex<HashMap<[u8;32], RoundEntry>>>      // active VRF rounds
  ├── round_history: Arc<Mutex<Vec<CompletedRound>>>        // completed VRF rounds
  ├── request_queue: Arc<Mutex<RequestQueue>>               // burst handling
  ├── on_chain: Option<OnChainCtx>                          // Solana context
  ├── metrics: Metrics                                       // Prometheus
  └── db: Option<PgPool>                                     // PostgreSQL
```

### New shared state needed

```
AppState additions:
  ├── keeper_tasks: Arc<Mutex<Vec<KeeperTask>>>             // registered tasks
  ├── keeper_history: Arc<Mutex<Vec<KeeperExecution>>>      // execution log
  └── notary_history: Arc<Mutex<Vec<NotaryReceipt>>>        // attestation log
```

### Task spawning (main.rs)

```
Current spawned tasks:
  1. REST API (Axum on :8080)
  2. Metrics Server (Prometheus on :9090)
  3. WebSocket Server (mTLS on :8443)
  4. Round Timeout Watchdog (every 5s)
  5. Solana WebSocket Subscriber (logsSubscribe)
  6. Per-node Connection Handler (per WebSocket)

New tasks to spawn:
  7. Keeper Evaluation Loop (every N seconds — configurable)
     - Independent tokio::spawn
     - Shares OnChainCtx for Solana access
     - Zero interaction with VRF state machine
```

### Message flow (unchanged for VRF)

```
VRF:     Request → Select Nodes → JobAssignment → Commits → Reveals → Finalize
Keeper:  Timer tick → Evaluate triggers → Build tx → Submit → Log receipt
Notary:  POST /notarize → Select Nodes → Dispatch hash → Collect sigs → Return receipt
```

All three paths are **completely independent**. No shared state machine. No shared message types (for now). The only shared resources are:
- `NodeRegistry` (to know which nodes are online)
- `OnChainCtx` (to submit Solana transactions)
- `PgPool` (to persist records)

---

## Sources

### Keeper / Automation Networks
- [Clockwork Shutdown — CoinDesk](https://www.coindesk.com/business/2023/08/28/solana-based-automation-startup-clockwork-to-shut-down)
- [Clockwork Shutdown — CoinTelegraph](https://cointelegraph.com/news/solana-based-automation-protocol-clockwork-to-shutter)
- [Clockwork Architecture — QuickNode Guide](https://www.quicknode.com/guides/solana-development/3rd-party-integrations/automation-with-clockwork)
- [Clockwork Threads — Official Docs](https://docs.clockwork.xyz/reference/threads)
- [Tuk Tuk — GitHub (Helium)](https://github.com/helium/tuktuk)
- [Tuk Tuk — Official Docs](https://www.tuktuk.fun/docs)
- [Tuk Tuk — Solana Compass / Accelerate 2025](https://solanacompass.com/learn/accelerate-25/scale-or-die-at-accelerate-2025-tuk-tuk-on-chain-cron-jobs)
- [Chainlink Automation — Official Docs](https://docs.chain.link/chainlink-automation)
- [Chainlink Compatible Contracts Guide](https://docs.chain.link/chainlink-automation/guides/compatible-contracts)
- [Gelato Network — Web3 Functions](https://www.gelato.network/web3-functions)
- [Gelato — Architecture Deep Dive (BizThon)](https://medium.com/@BizthonOfficial/gelato-network-automating-web3-workflows-with-decentralized-precision-6643508b856c)
- [GEL Token Announcement](https://medium.com/gelato-network/announcing-the-gel-token-stewarding-gelato-into-a-sustainable-future-815c0ecedb8e)

### Solana Transaction Landing
- [Transaction Latency — Chorus One](https://chorus.one/reports-research/transaction-latency-on-solana-do-swqos-priority-fees-and-jito-tips-make-your-transactions-land-faster)
- [Transaction Landing Tips — QuickNode](https://blog.quicknode.com/five-tips-to-help-land-your-solana-transactions/)
- [Jito Tips — Medium](https://medium.com/@ramasheshan8/jito-tips-the-underground-highway-of-solana-transactions-d839bd74ad9d)
- [Jito Low Latency TX Docs](https://docs.jito.wtf/lowlatencytxnsend/)

### Notary / Timestamping
- [RFC 3161 — Internet X.509 PKI Time-Stamp Protocol](https://datatracker.ietf.org/doc/html/rfc3161)
- [OpenTimestamps — Official Site](https://opentimestamps.org/)
- [OpenTimestamps — Peter Todd Announcement](https://petertodd.org/2016/opentimestamps-announcement)
- [Solana Attestation Service — Range Security](https://www.range.org/blog/introducing-solana-attestation-service)
- [Solana Attestation Service](https://attest.solana.com/)
- [Ethereum Attestation Service (EAS)](https://attest.org/)
- [IETF SCITT Architecture](https://datatracker.ietf.org/doc/html/draft-ietf-scitt-architecture)
- [Solana Validator Timestamp Oracle](https://docs.anza.xyz/implemented-proposals/validator-timestamp-oracle)
- [Solana Clock — RareSkills](https://rareskills.io/post/solana-clock)
- [Solana Rent Calculation — RareSkills](https://rareskills.io/post/solana-account-rent)

---

*Research compiled 2026-04-07. Based on DICE v3 codebase, comprehensive study of Clockwork, Tuk Tuk, Chainlink Automation, Gelato Network, RFC 3161, OpenTimestamps, Solana Attestation Service, and SCITT/IETF standards.*
