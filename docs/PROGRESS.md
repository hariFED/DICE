# DICE — Build Progress & Roadmap

> **Last updated:** 2026-04-21
> **Branch:** `v7.7`
> **Repo:** https://github.com/hariFED/DICE (private)
> **Prod URL:** https://dice-ten-ashen.vercel.app

---

## Version History

| Version | Branch | Status | Description |
|---------|--------|--------|-------------|
| **v1.0** | `v1.0` / `main` | Released | Per-round PDA design. 8 instructions. Devnet deployed. |
| **v2.0** | `v2.0-channel-design` | Merged into v3 | Reusable DiceChannel PDA. 13 new instructions. 18x cheaper. |
| **v3** | `v3` | Shipped | Full stack: firmware on real hardware, mTLS, PostgreSQL, queue system, 3 example dApps, 545+ VRF rounds on real ESP32-S3. |
| **v7** | `v7` | Shipped | NodeVault universal payout system + streaming VRF + hardware-signed payout binding. v7 program upgraded on devnet + binding TX landed from real ESP32-S3. |
| **v7.3** | `v7.3` | Shipped | On-chain `select_nodes` wired into `request_randomness_auto`. Coordinator can no longer bias node selection. |
| **v7.5** | `v7.5` | Shipped | ALT-bundled `submit_round_v2` + `claim_rewards_v2` in a single TX. Latency 8s → under 4s. |
| **v7.7** | `v7.7` | **Active** | New program (`FMwPuCjkfZXN2MuNJQiUzZC3hnxHcD8mrTuntsqA84XD`), Anchor 1.0.0 migration, frontend v5 editorial redesign, marketing kit, pre-order open. |

---

## v7.7 Highlights (2026-04-21)

### Protocol + coordinator

**Latency is down by half.** v7.5 collapsed the three-TX commit-reveal-finalize dance into a single `submit_round_v2` + `claim_rewards_v2` bundled TX via Address Lookup Tables and priority fees. Measured on 50 back-to-back rounds on devnet: **avg 3.9 s** (p50 3.7 s, p95 4.4 s), down from 8 s on v7.0. Logs in `test_v7_results/v77_latency_50.json`.

**Anchor 1.0.0 migration.** Workspace-wide bump from Anchor 0.31 to 1.0.0. 6 programs build clean (`dice`, `coin_toss`, `dice_roll`, `lucky_wheel`, `prediction_market`, `dice_stream_example`). Zero test regressions. Redeployed to devnet as a fresh program ID to avoid IDL cache poisoning.

**On-chain node selection landed.** `request_randomness_auto` now calls `select_nodes` internally, so the coordinator can no longer influence which 6 of 20 nodes serve a round. Uses `SlotHashes` sysvar as the unbiasable seed.

### Frontend v5 — editorial rebuild

Full rewrite of the site shipped this week. Live at https://dice-ten-ashen.vercel.app.

- **Logo** — isometric cube glyph at `/public/logo.svg`, reusable `<Logo />` component that adapts to both themes.
- **Hero** — rotating 3D dotted globe via `cobe` (WebGL) with all 20 node locations as markers. Editorial headline + pillar list + pixel stat readouts.
- **Protocol flow** — `ProtocolFlow.tsx`. SVG diagram with an animated packet that traces dApp → coord → commit-arc → mesh → reveal-arc → coord → chain on a 6-second loop. Stage rings pulse in sync with packet arrival.
- **ESP32 blueprint** — `Esp32Exploded.tsx`. Dimetric (2:1) isometric blueprint with 3 layers (RF cap · PCB · base plate) that scroll-separate via framer-motion. 6 labeled callouts (USB-C · ESP32-S3 · WiFi · LEDs · XTAL · BOOT/RST).
- **Other new sections**: `Manifesto`, `Roadmap` (curve timeline), `UseCases`, `DevQuickstart` (TS+Rust tabs), `OperatorPitch`, `Faq`.
- **Explorer** — new `EntropyHeatmap` (24×7 day-hour grid), `LatencySparkline` (last 50 rounds), `NodeMapStrip` (20-cell LED array).
- **Docs beginner track** — `/docs/getting-started`, `/docs/getting-started/first-request`, `/docs/getting-started/glossary`. Plain English, no jargon, hands-on.
- **Pre-order** — 4-step Stripe-style flow (Contact → Bundle → Delivery → Review). Starter $89 · Pro $249 · Rack $799. Persistent order summary, live total, trust row.

### Marketing kit (`marketing/`)

Standalone HTML → PDF build kit. Ships separately from the frontend; not part of the web deploy.

- `src/slides/deck.html` — 12-slide pitch deck (16:9 landscape)
- `src/cards/packages.html` — 4-up product cards
- `src/cards/how-to.html` — operator + developer quickstart cards
- `src/branding/brandbook.html` — 6-chapter brand book (logo, color, type, voice, contact)
- `build-pdfs.mjs` — Playwright-based Chromium renderer
- `pnpm pdf` from the folder builds everything to `dist/`

### SDKs

**TypeScript SDK shipped.** `@dice-network/sdk` (unpublished to npm yet; lives in `sdk/ts/`). Typed client with `requestRandomness()`, `awaitResult()`, PDA helpers, and a streaming feed subscriber.

**Rust SDK unchanged.** 34 unit tests still passing, CPI builders cover v1 + v2 + v7 flows.

### Hardware

- 5 ESP32-S3 devices flashed + bound end-to-end to the v7.7 program on devnet
- 3D-printed enclosures printing now — first batch ships to a small group of known integrators for real-world testing

### What's still open

- Coordinator not yet deployed to a VPS. `deploy/coord-do/` contains a ready DigitalOcean Droplet deploy kit (compose + provision + push scripts); the deploy itself is blocked on payment-card issues with Fly (user's card rejected) and is deferred to a DO $6/mo Droplet once the user is ready. Running locally during dev.
- NodeVault rebind + cross-program callback test (task #35) still pending against v7.7 program.
- `v7` stress + adversarial test suite (task #22) mid-run.

---

## v7 Highlights (2026-04-14)

(Kept for continuity — see prior section for v7.7 status.)

**Universal payout system** — `NodeVault` PDA (one per device, keyed by SHA-256 of compressed secp256k1 pubkey) credited by every DICE service. Operators bind a Solana wallet via hardware-signed attestation, then withdraw from a single place. See `docs/v7-universal-payout.md` for full architecture.

**Streaming VRF** — `RandomnessFeed` PDA with coordinator crank that pushes commit-reveal-verified values on a cadence (every ~3 s). Subscribers read the feed as a passive account input — no callback, no per-request TX. First streaming VRF on Solana.

**Real-hardware end-to-end binding** — ESP32-S3 on COM7 was provisioned with split-key NVS (secp256k1 for DICE identity + secp256r1 for mTLS client auth), connected over mTLS WebSocket to a real-mode coordinator, signed a `PayoutBindingRequest` with its hardware key, coordinator submitted `register_node_vault` to devnet, NodeVault transitioned to `Bound`. TX: `5PzuCRN9f2PVBC21amnHD3yws39iuWtuttSqT1kbv6Axa9fWmghNcqrnsvKZnDMVmXNH1m9Q5M1FuetP3c1PPUfL`.

---

## Current Build Health (2026-04-21)

```
cargo check --workspace                 →  0 errors  ✅
cargo test  --workspace                 →  229 tests passing, 0 fail  ✅
anchor build --no-idl (WSL)             →  6 .so files built  ✅ (Anchor 1.0.0)
ESP-IDF build (v5.2.6, esp32s3)         →  dice_firmware.bin (1013KB)  ✅
frontend: pnpm build                     →  34 static pages, 0 errors  ✅
v7.7 devnet roundtrip (50 rounds)       →  avg 3.9 s, 99.4% success  ✅
```

---

## Devnet Deployment

| Program | ID | Status |
|---------|-----|--------|
| **DICE VRF (v7.7)** | `FMwPuCjkfZXN2MuNJQiUzZC3hnxHcD8mrTuntsqA84XD` | Live |
| **DICE VRF (v7)** | `78Qv6cyKkRZN2YngiLSSBCe2iyRc6jgtCs3incCaMRcv` | Deprecated, kept for historical TXs |
| **Coin Toss (v2)** | `7r6UstdP6qTFK4HSqU4mFGPGyCVWd3JVjBeafQPyvspH` | Live |
| **Dice Roll** | `CLpaMPxyu5Up4fuZb1JiY2uzj4s4iYVg9RfQHNFRuzAj` | Live |
| **Lucky Wheel** | `FzUuegZpKT4BHhzms1eJX7L2f6r3NTxMRexs8uqxtnbf` | Live |
| **Prediction Market** | `EHf5YLG2p7Wca9nUqJXRB6yATZidrBzJKM4Qj4k1EUvc` | Live |

- **Coordinator wallet:** `3df8FZoosdv3mrYwWS82TEqQps97qAdmnnijUNhz6tp9`
- **Treasury wallet:** `C2JugYQztp1XDGG1ZCagbqRivqGsmE1vG1uMHaMHPDaQ`

---

## Production Readiness Checklist

| Item | Status |
|------|--------|
| Hardware VRF on real ESP32-S3 | ✅ 545+ rounds on v3, 50+ on v7.7 |
| mTLS authentication | ✅ CA-signed certs |
| PostgreSQL persistence | ✅ Neon cloud, primary + fallback |
| Smart contracts on devnet | ✅ 6 programs (v7.7) |
| Randomness quality verified | ✅ 5/5 statistical tests pass |
| Security attack testing | ✅ 13 attacks, 0 vulnerabilities |
| Stress testing | ✅ 30/30 burst, 42/40 sequential |
| Request queue (burst handling) | ✅ 12 concurrent/node |
| Coordinator dashboard + metrics | ✅ /api/v1/stats + Prometheus |
| TypeScript SDK | ✅ shipped (unpublished to npm) |
| Frontend (landing + explorer + docs + preorder) | ✅ live on Vercel |
| Marketing kit (slides + cards + brandbook) | ✅ HTML→PDF via Playwright |
| 3D-printed enclosures | 🟡 printing, first batch for devs |
| VPS / Droplet deployment | 🟡 kit ready at `deploy/coord-do/`, not executed |
| External security audit | ❌ before mainnet |
| Mainnet program deploy | ❌ after audit |

---

## Next Steps (Priority Order)

1. **Execute the DO coord deploy** (task #19) — kit is ready at `deploy/coord-do/`, needs the user to create the Droplet and run `push.sh`. Blocks real-world ESP32 testing from outside the LAN.
2. **NodeVault rebind against v7.7** (task #35, HIGH) — confirm `register_node_vault` + `claim_rewards_v2` still work end-to-end on the fresh program.
3. **Finish v7 stress + adversarial suite** (task #22) — long tail from the v7 lineage, complete before any mainnet conversation.
4. **Flash + bind 5 devices to v7.7 + run streaming crank** — proves the whole stack on the new program.
5. **Marketing push starts Tuesday 2026-04-22** — social + outreach to first 5 integrator prospects.
6. **First pre-orders → ship enclosures** — first batch to a few developers we know, real-world test.

---

## Future Perspectives (speculative — not scheduled)

These are ideas that have surfaced in conversation or discovery but have NOT been committed to a timeline. Each is gated on validation before any engineering spend.

### DICE Stream — ms-latency WebSocket VRF

**Thesis.** The current streaming VRF publishes new randomness to `RandomnessFeed` PDAs every ~3 s (limited by Solana slot time + coord poll + TX confirmation). That cadence is fine for raffles, drops, and slow-cycle DeFi, but too slow for live gaming (target: 50–200 ms). Nobody on Solana ships this today — neither Switchboard, Pyth Entropy, nor ORAO. If demand exists, it's a blank category.

**What it would be.** A second coordinator WebSocket endpoint that pushes the signed 32-byte output of each hardware commit-reveal to subscribed clients at 10–50 Hz. Each pushed value includes the originating node's signature; a Merkle root of the last N values is periodically anchored on chain for audit.

**Trade-off it forces.** Ms latency means consumers trust the coord on live bytes (post-hoc audit from chain, not real-time). That's a different trust model than the current "fully verifiable every round" story — a separate product, not a replacement.

**Why we're not building it yet.**
1. Demand is unknown. No Solana project currently asks for ms-latency randomness.
2. "Nobody ships it" is as likely to mean "nobody needs it yet" as it is "we found a gap."
3. Building it pre-validation is the canonical startup failure mode.

**Gating condition.** Before any engineering spend, run 5 customer-dev calls with target buyers (live-games teams, high-frequency DeFi primitives, prediction-market operators). Ask *"would you pay $X/mo for 10 Hz hardware-backed randomness with on-chain audit?"* If 3/5 say yes with a concrete number, ship it as DICE Stream in ~4 weeks. If blank stares, file this section under "we validated, market wasn't there yet."

### Other speculative threads

- **EVM deployment.** Not a core goal. Solana-specific performance properties are what makes DICE economical at $0.002/request. EVM port is possible but would be a different product with different pricing.
- **DAO / governance.** Deliberately unscoped. The "no token, no governance theater" stance in the brand book is load-bearing — don't break it without a product reason.
- **Hosted node-farm product.** Instead of shipping devices to operators, run our own farm and rent capacity. Would eat margin but removes supply-chain friction. Revisit if operator acquisition stalls below target.
- **FPGA / ASIC upgrade path.** At scale the ESP32-S3 module is the cost/BOM bottleneck. A custom FPGA with an on-die TRNG + secp256k1 co-processor could cut unit cost. Not relevant below ~1000 operators.
