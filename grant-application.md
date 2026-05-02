# DICE Network — Agentic Engineering Grant Application

Generated via Claude Code + the `apply-grant` skill from solana.new.

**Submit at**: https://superteam.fun/earn/grants/agentic-engineering
**Amount**: 200 USDG

---

## Step 1: Basics

**Project Title**: DICE Network

**One Line Description**: Hardware-backed verifiable randomness oracle on Solana — ESP32-S3 nodes run a commit-reveal protocol and deliver 32-byte randomness on-chain for 0.002 SOL per request.

**TG username**: t.me/haridluffy

**Wallet Address**: 4n9V4tTKNAJjvhJ4AeqpyEUMgLNMNsAGrmkB4c9oRAs6

---

## Step 2: Details

### Project Details

DICE (Distributed Infrastructure for Cryptographic Entropy) is a hardware-backed VRF network on Solana. Today's randomness options — Switchboard, Chainlink, ORAO — rely on off-chain compute, economic staking, or closed trust assumptions. None prove that entropy came from a physical, tamper-resistant source. DICE fixes this with ESP32-S3 secure-boot devices running immutable firmware. Each node generates hardware entropy, signs with secp256k1, and participates in a commit–reveal round; one honest node is enough for unpredictability.

The stack is production-grade: a Rust/Anchor core program (v7.7, deployed on devnet at `FMwPuCjkfZXN2MuNJQiUzZC3hnxHcD8mrTuntsqA84XD`), a Rust/Tokio coordinator with mTLS WebSocket transport, a Rust CPI SDK (`dice-vrf`), a TypeScript SDK, and a private step-ca PKI. As of v7.5, the entire commit→reveal→finalize flow lands in a single ALT-bundled transaction — measured devnet latency is **avg 3.9 s / p95 4.4 s** over 50 back-to-back rounds (down from 8 s on v7.0). Node selection is trustless via the `SlotHashes` sysvar, so the coordinator cannot pick which nodes serve a round.

Developer experience is the moat: 2 lines of Rust CPI code, one 0.002 SOL fee, no staking, no dashboard, no protocol token. The 70/20/10 fee split (nodes/treasury/reserve) hits ROI in month 1 for a $15 ESP32 operator. Four reference dApps ship in the repo (`coin-toss`, `dice-roll`, `lucky-wheel`, `prediction-market`) plus a streaming-VRF example (`pulse`).

Market positioning is strong: Colosseum Copilot scores the hardware-backed VRF on Solana ring at **1/10 crowdedness** with only one direct competitor (`infratic`). Composite score: **2.4/10 — lightly populated, zero accelerator/winner pedigree**. This grant accelerates mainnet launch: audit prep, final firmware hardening, and the first operator-hardware distribution run (Starter $89 · Pro $249 · Rack $799 pre-orders live).

### Deadline

2026-05-11 (Asia/Calcutta)

### Proof of Work

- **GitHub**: https://github.com/hariFED/DICE
- **Live frontend**: https://dice-ten-ashen.vercel.app (34 static pages, Next.js 15 App Router)
- **v7.7 on-chain program**: https://explorer.solana.com/address/FMwPuCjkfZXN2MuNJQiUzZC3hnxHcD8mrTuntsqA84XD?cluster=devnet
- **v7.5 predecessor (still live)**: https://explorer.solana.com/address/78Qv6cyKkRZN2YngiLSSBCe2iyRc6jgtCs3incCaMRcv?cluster=devnet
- **4 example dApps on devnet**:
  - `coin_toss` → `7r6UstdP6qTFK4HSqU4mFGPGyCVWd3JVjBeafQPyvspH`
  - `dice_roll` → `CLpaMPxyu5Up4fuZb1JiY2uzj4s4iYVg9RfQHNFRuzAj`
  - `lucky_wheel` → `FzUuegZpKT4BHhzms1eJX7L2f6r3NTxMRexs8uqxtnbf`
  - `prediction_market` → `EHf5YLG2p7Wca9nUqJXRB6yATZidrBzJKM4Qj4k1EUvc`
- **Build health**: `cargo check --workspace` 0 errors; `cargo test --workspace` 229 pass / 0 fail; `anchor build --no-idl` 6 programs clean; Anchor 1.0.0 migration complete
- **Hardware test report**: 545-round run on real ESP32-S3 mesh (`tests/DICE_HARDWARE_TEST_REPORT.md`)
- **Latency benchmark**: 50 rounds, avg 3.9 s / p50 3.7 s / p95 4.4 s (`test_v7_results/v77_latency_50.json`)
- **Colosseum Copilot competitive analysis**: 2.4/10 composite crowdedness across 5 hackathons (Hyperdrive → Cypherpunk)
- **Shipped in the last month** (20 commits): v7.7 editorial redesign + landing rebuild, Anchor 1.0.0 migration, pre-order flow, beginner docs, Vercel deploy config, ESP32 isometric blueprint, v7 universal-payout NodeVault architecture, hardware-signed payout binding, ALT-bundled v2 TX, CBOR protocol type 5, streaming-VRF feed lifecycle, private PKI (step-ca, Root CA air-gapped)
- **Research**: VRF streaming + multisig delivery models report in `research/`
- **Pitch deck + marketing kit**: 12-slide investor deck, 4-up product cards, operator/dev how-to cards, brand book in `marketing/` + `pitch_deck/`

### Personal X Profile

x.com/harixhilfiger

### Personal GitHub Profile

github.com/hariFED

### Colosseum Crowdedness Score

2.4 / 10 — lightly populated.

- Hardware-backed VRF on Solana: **1 / 10** (only competitor: infratic)
- VRF / randomness service supply (any tech): 3 / 10
- IoT / hardware oracle on Solana (adjacent): 5 / 10
- DePIN trust-layer umbrella: 6 / 10

Composite = 0.5·(direct) + 0.3·(VRF supply) + 0.2·(IoT oracle) ≈ 2.4

Screenshot: https://drive.google.com/file/d/1Vb95BA7Rk1o7Bzj8XGClQsKfvgDFWSs9/view?usp=sharing

### AI Session Transcript

Attached: `claude-session.jsonl` (66 KB, auto-exported from Claude Code via the `apply-grant` skill)

---

## Step 3: Milestones

### Goals and Milestones

**M1 — v7.7 devnet hardening + audit-lite (Week 1, by 2026-04-28)**
Fork-frozen v7.7 tag on devnet. sec3 automated scan pass. Internal STRIDE threat model. Final firmware freeze for 20 pilot devices. Provisioning station dry-run (step-ca Intermediate CA, LUKS-FDE, device-key issuance flow).

**M2 — First hardware cohort distributed to operators (Week 2, by 2026-05-05)**
10–20 ESP32-S3 devices provisioned with hardware-signed payout binding and shipped to pre-order operators (Starter $89 / Pro $249 / Rack $799 SKUs). Welcome card + ops manual bundled. Operators bring nodes online against the devnet coordinator.

**M3 — Real-world devnet testing + public dashboard (ship deadline, 2026-05-11)**
Live mesh of 10+ geographically-distributed nodes running 24/7 against devnet. Public dashboard showing node health, per-round latency, and finalization rate. At least 2 reference dApps (coin-toss + prediction-market) running live devnet traffic. First 1,000 randomness requests finalized end-to-end on real hardware. Latency + uptime report published in `test_v7_results/`.

**Post-grant north star — Mainnet launch**
Once devnet real-world testing proves stable over a sustained window, the path to mainnet is: audit (sec3 automated + manual with OtterSec/Neodyme/Halborn), 2-of-3 Squads multisig on program upgrade authority, mainnet program deploy, and TypeScript SDK publish to npm as `@dice-network/sdk`. Grant work de-risks this final step by validating the protocol on real devices under real conditions first.

### Primary KPI

Randomness requests finalized on devnet by real distributed hardware (not simulators) by 2026-05-11 — target: 1,000 requests across 10+ physical ESP32-S3 nodes

### Final tranche checkbox

Acknowledged: to receive the final tranche I must submit (1) the Colosseum project link, (2) the GitHub repo link, and (3) the AI subscription receipt.
