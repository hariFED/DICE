# DICE — File & Folder Index

> Catalog of every top-level file and folder in the repo.
> Each entry: **what it is** and **why it's here**.
> Last updated: 2026-04-21 · Branch: v7.7

---

## Build & workspace configs (root-level)

| Entry | What | Why |
|---|---|---|
| `Cargo.toml` | Rust workspace manifest | Pins the 15 workspace crates (coordinator, 6 programs, 2 SDKs, 6 harnesses) and shared deps (Anchor 1.0.0, solana-sdk 1.18.26, tokio, k256, rustls 0.21). `load_generator` intentionally excluded due to spl-token-2022 dep conflicts. |
| `Cargo.lock` | Rust dep lockfile | Reproducible builds across dev + CI. |
| `Anchor.toml` | Anchor CLI config | Pins `anchor_version = "1.0.0"`, maps 5 program IDs for localnet + devnet (v7.7 `FMwPuC…` plus 4 example dApps). |
| `package.json` | Root npm manifest | Pulls `@coral-xyz/anchor`, `ts-mocha`, `@types/mocha` for the top-level TypeScript test suites (`tests/dice.ts`, `tests/dice_v2.ts`). |
| `package-lock.json` | npm lockfile for the root suite | Reproducible TS test deps. |
| `tsconfig.json` | Top-level TS config | Used by `ts-mocha` when running integration tests from the repo root. |
| `.gitignore` | Ignore list | Excludes `target/`, `.anchor/`, `node_modules/`, `.next/`, `build/v7_nvs*` (device secrets), `*.key`, `*.pem`, `coordinator-keypair.json`, `.env`. |
| `.dockerignore` | Docker build context filter | Strips `target/`, `node_modules/`, `.next/`, `.git/`, test output before docker build — makes the coordinator image build fast. |
| `.vercelignore` | Vercel deploy filter | Strips non-frontend roots from the deploy context. |
| `.env.example` | Template env file | Documents `DATABASE_URL`, `SOLANA_RPC_URL`, TLS cert paths, port flags for the coordinator. |
| `.env` | Local secrets (ignored) | Real DB URL + keys for local runs. **Do not commit.** |

---

## Deploy configs (root-level)

| Entry | What | Why |
|---|---|---|
| `DEPLOY.md` | Deploy runbook | Step-by-step coordinator + program deploy notes. Currently being rewritten from Fly-first to DigitalOcean-first (see `docs/TODO.md`). |
| `fly.toml` | Fly.io app config | Legacy — drafted before the Fly card rejection. Kept until DO migration is finalized. |
| `coordinator-keypair.json` | Solana keypair for the coordinator (ignored) | Signs on-chain TXs when `--simulation` runs with devnet enabled. **Never commit.** |
| `start_production.bat` | *(moved to `archive/legacy/`)* | — |
| `build_dice_wsl.sh` | *(moved to `archive/legacy/`)* | — |

---

## Source: core programs

| Entry | What | Why |
|---|---|---|
| `programs/dice/` | Core VRF Anchor program (v7.7, `FMwPuC…`) | The on-chain randomness oracle: `register_device`, `request_randomness_auto`, `submit_round_v2`, `finalize_randomness`, `claim_rewards_v2`, NodeVault, streaming-feed, `select_nodes`. Folder is intentionally single-program so the core stays isolated from examples. |

## Source: example dApps (`dapp-examples/`)

Reference integrations. Each is a self-contained Anchor program that CPIs into `dice` and implements its own callback handler. Moved out of `programs/` 2026-04-21 so the core program is easy to find and the examples are easy to copy.

| Entry | What | Why |
|---|---|---|
| `dapp-examples/coin-toss/` | 50/50 coin flip | Minimal reference — the simplest possible integration that takes one bit from the 32-byte random value. |
| `dapp-examples/dice-roll/` | Classic dice roll (1–6) | Shows how to reduce a 32-byte value to a bounded range without modulo bias. |
| `dapp-examples/lucky-wheel/` | Weighted spinning wheel | Demonstrates weighted draws on top of a uniform 32-byte output. |
| `dapp-examples/prediction-market/` | Binary outcome settlement | Multi-round randomness used to settle markets; shows the auto-request path. |
| `dapp-examples/pulse/` | Streaming-VRF consumer | Reads `RandomnessFeed` as a passive account input — no commit-reveal roundtrip. Pattern for latency-sensitive dApps. |

---

## Source: coordinator

| Entry | What | Why |
|---|---|---|
| `coordinator/` | Rust coordinator server | WebSocket endpoint for nodes, REST API + dashboard on 8080, Prometheus on 9090, Solana RPC client. Owns round orchestration, ALT building, priority-fee tuning, and the DB-backed audit log. |
| `coordinator/src/` | Coordinator source | Modules for WS, REST, round state machine, Solana submission, metrics. |
| `coordinator/tests/` | Coordinator integration tests | Round finalization flows, ECDSA verification, ALT construction. |
| `coordinator/Cargo.toml` | Coordinator crate manifest | Pulls tokio-tungstenite 0.21, rustls 0.21, sqlx 0.8 Postgres, axum 0.7. |

---

## Source: SDKs

| Entry | What | Why |
|---|---|---|
| `sdk/dice-vrf/` | Rust CPI SDK | `request_randomness_ix` + PDA helpers — what dApps import to call into `dice` on-chain. |
| `sdk/dice-vrf-macros/` | Proc-macro crate | Generates the `#[dice_callback]` attribute and callback-discriminator constants used by the SDK. |
| `sdk/dice-vrf-ts/` | TypeScript SDK (`@dice-network/sdk`) | Typed client — `requestRandomness`, `awaitResult`, PDA helpers, streaming-feed subscriber. Not yet on npm. |

---

## Source: firmware

| Entry | What | Why |
|---|---|---|
| `firmware/` | ESP32-S3 ESP-IDF v5.x project | Node firmware — entropy harvest, commit-reveal, mTLS, NVS provisioning, LED status, captive-portal onboarding. |
| `firmware/main/` | Firmware entry + round loop | App logic that connects to the coordinator, signs with secp256k1, and reports health. |
| `firmware/components/` | Custom ESP-IDF components | Shared crypto + CBOR framing used across the round loop. |
| `firmware/managed_components/` | Pulled-in ESP-IDF components | Tracked by `dependencies.lock`. |
| `firmware/partitions.csv` | Flash partition table | Reserves NVS + app + OTA regions. |
| `firmware/sdkconfig` + `sdkconfig.defaults` | ESP-IDF build config | Pin chip target, Wi-Fi, TLS stack. Runtime `sdkconfig` is gitignored. |
| `firmware/HARDWARE.md` | Board wiring + flashing notes | How to flash and provision a new ESP32-S3 unit. |
| `firmware/keys/` | Per-device key material (ignored) | secp256k1 node keys generated during provisioning. |
| `firmware/build_firmware.bat`, `flash_firmware.bat` | Build/flash helpers for Windows | Convenience wrappers around `idf.py build` / `flash`. |
| `firmware/bounded_monitor.py` | Serial monitor with bounded buffer | Used during hardware testing so logs don't blow up memory. |
| `firmware/test/` | On-device smoke tests | Unity-based tests that run on hardware. |
| `firmware/tools/` | Provisioning scripts | NVS image builders, cert injectors. |

---

## Source: frontend

| Entry | What | Why |
|---|---|---|
| `frontend/` | Next.js 15 App Router site | Live at https://dice-ten-ashen.vercel.app. v5 editorial redesign shipped 2026-04-21. |
| `frontend/app/` | App Router routes | Landing, `/docs/getting-started/*`, `/explorer`, `/preorder`, `/manifesto`, `/roadmap`. |
| `frontend/components/` | Shared UI | `Logo`, `ProtocolFlow`, `Esp32Exploded`, `EntropyHeatmap`, `LatencySparkline`, `NodeMapStrip`, pre-order 4-step form. |
| `frontend/lib/` | Client + server helpers | Stats fetchers, BFF callers, formatting utils. |
| `frontend/public/` | Static assets | `logo.svg` (isometric cube glyph), OG images, favicon. |
| `frontend/vercel.json` | Vercel config | `rootDirectory=frontend`, pnpm@10 pin, Next.js framework preset. |
| `frontend/next.config.ts` | Next.js config | Image domains, experimental flags. |
| `frontend/eslint.config.mjs`, `postcss.config.mjs`, `components.json` | Tooling config | shadcn/ui registry, Tailwind postcss, ESLint flat config. |
| `frontend/pnpm-lock.yaml` + `pnpm-workspace.yaml` | pnpm lockfile + workspace | Locks frontend deps; workspace is scoped to `frontend/` only. |
| `frontend/CLAUDE.md`, `AGENTS.md`, `LANDING_PAGE_PROMPT.md`, `PLAN.md` | Agent briefs | Instructions used when the frontend was generated/iterated by agents. |
| `frontend/README.md` | Frontend-specific readme | Local dev + deploy notes for the site. |
| `frontend/refreence/` | Design references *(sic, typo preserved)* | Snapshots/screenshots informing the editorial redesign. |
| `frontend/data/` | Static data for the site | Node locations (for cobe globe markers), roadmap items, FAQ. |

---

## Source: tests

| Entry | What | Why |
|---|---|---|
| `tests/dice.ts` | Core Anchor TS integration suite | Exercises `register_device`, `request_randomness`, commit/reveal PDAs on devnet. |
| `tests/dice_v2.ts` | v2/v7 channel + streaming suite | Covers reusable DiceChannel, streaming feeds, NodeVault. |
| `tests/devnet_setup.ts` | Bootstrap fixtures | One-shot script that creates the escrow + channel PDAs tests depend on. |
| `tests/onchain_setup_test.py` | Python bootstrap | Alternate setup path used by the Python dashboard. |
| `tests/harness/mock_firmware_node/` | Simulated ESP32 node (Rust) | k256 ECDSA + CBOR + WebSocket client that passes as a real node to the coordinator. |
| `tests/harness/smoke_v7/` | End-to-end smoke runner | One-shot driver that spins coord + 7 mocks + triggers a round. |
| `tests/harness/stress_driver/` | v2-channel load tester | Sequential rounds on a single channel, records latency + wedges. Used for `archive/v7-test-dashboards/test_v7.md` categories A + J. |
| `tests/harness/adversarial_driver/` | Adversarial injection runner | Malformed commits, wrong-entropy reveals, byzantine nodes. |
| `tests/harness/coin_toss_driver/` | Coin-toss dApp driver | Round-trips the coin-toss example through `dice`. |
| `tests/harness/pulse_driver/` | Streaming-feed driver | Exercises publish/subscribe on the Pulse feed. |
| `tests/harness/v73_driver/` | On-chain selection validator | Calls `request_randomness_auto` to verify `select_nodes` CPI. |
| `tests/harness/load_generator/` | Heavy-load generator (excluded from workspace) | Build separately: `cargo check --manifest-path tests/harness/load_generator/Cargo.toml`. Excluded because spl-token-2022 pulls solana-program =1.17.6 which conflicts with workspace solana-sdk =1.18.26. |
| `tests/e2e/` | End-to-end scripts | Shell + TS runners that assemble full scenarios. |
| `tests/fixtures/` | Shared test inputs | Deterministic entropy samples, canned payloads. |
| `tests/DICE_HARDWARE_TEST_REPORT.md` | 545-round real-hardware run report | Artefact from the v3 hardware milestone. |
| `tests/ONCHAIN_VRF_TEST_RESULTS.md` | On-chain correctness run report | Randomness verified against finalized TX data. |
| `tests/STORAGE_AND_SECURITY_ANALYSIS.md` | Data-at-rest + mTLS review | Captures how device keys, DB rows, and certs are protected. |
| `tests/production_readiness_test.sh` | Pre-deploy check script | Health/endpoint/perf smoke before promoting. |
| `tests/security_attacks.sh` | Attack-suite runner | Fires the adversarial driver with a fixed matrix. |
| `tests/run_battle_tests.sh` | Full stress + adversarial runner | Bundle used for v7 battle-testing (task #22). |

---

## Infra

| Entry | What | Why |
|---|---|---|
| `docker/` | Dockerfiles + compose | Production + dev + test stacks for coordinator, mock nodes, Postgres, Prometheus. |
| `docker/docker-compose.yml` | Default compose | `pnpm dev`-style local bring-up. |
| `docker/docker-compose.prod.yml` | Production overlay | Resource limits, TLS, structured logs. |
| `docker/docker-compose.test.yml` | CI overlay | Ephemeral Postgres, test-only ports. |
| `docker/Dockerfile.coordinator[.prod]` | Coordinator images | Dev (debug build) + prod (release, musl, no shell). |
| `docker/Dockerfile.mock-node` | Mock node image | Used in the compose stack to stand up simulated nodes. |
| `docker/prometheus/` | Prometheus scrape config | Scrapes coordinator:9090 and labels series. |
| `deploy/coord-do/` | DigitalOcean Droplet deploy kit | `provision.sh` + `push.sh` + compose — ready to bring up the coordinator on a $6/mo DO Droplet. Not yet executed (task #19, blocked on user card). |
| `pki/` | Private PKI (step-ca) | Issues device mTLS certs, coordinator TLS cert, intermediate + root CAs. Private keys are gitignored. |
| `certs/` | Dev cert scratch dir | Generated by the PKI during local testing. |
| `.github/workflows/` | CI pipeline | `cargo check`, `cargo test`, clippy, cargo-audit. |

---

## Support + ops code

| Entry | What | Why |
|---|---|---|
| `scripts/verify_idl_types.py` | IDL/type-schema verifier | Catches drift between `programs/dice` Anchor types and the manually-maintained `target/idl/dice.json`. |
| `scripts/verify_pda_compat.py` | PDA derivation cross-check | Ensures Rust + TS + Python all derive PDAs identically. |
| `scripts/verify_protocol_compat.py` | Wire-protocol sanity check | CBOR framing + discriminator bytes match between firmware + coordinator. |

---

## Output / artifacts (ignored or live-updating)

| Entry | What | Why |
|---|---|---|
| `target/` | Rust build output (ignored) | Regenerated by cargo. |
| `build/` | Firmware + reports build output (ignored) | Contains `build/v7_nvs*/` with **device secp256k1 private keys + mTLS certs** from provisioning runs. Never commit. |
| `.next/` | Next.js build output (ignored) | Regenerated by `pnpm build`. |
| `.anchor/` | Anchor-test ledger + artifacts (ignored) | Temporary local-validator state. |
| `.pytest_cache/` | pytest cache (ignored) | |
| `.vercel/` | Vercel CLI link metadata (ignored) | |
| `node_modules/` | Root npm install (ignored) | |
| `test_v7_results/` | Latency + stress run outputs | **Current** — v7.3 → v7.7 runs land here. Latest: `v77_latency_50.json` (avg 3.9 s), `v77_streaming_50.log`. Referenced in `docs/PROGRESS.md`. |

---

## Docs, narrative, marketing

| Entry | What | Why |
|---|---|---|
| `docs/PROGRESS.md` | Living version history + v7.7 changelog | Single source of truth for what shipped when. |
| `docs/TODO.md` | Open task list | What's shipped, what's blocked (coord-DO deploy, NodeVault rebind, stress+adversarial). |
| `docs/SIMULATION.md` | CLI reference for simulation mode | Every flag on the coordinator + mock node. |
| `docs/TEST_REPORT.md` | Full test results + on-chain account addresses | Auditable link from tests back to explorer. |
| `docs/CHANNEL_DESIGN.md` | v2 DiceChannel PDA design doc | Why reusable channels exist; 18× cheaper than per-round PDAs. |
| `docs/V2_CHANGELOG.md` | v1→v2 instruction map | Migration cheatsheet for integrators. |
| `docs/ANCHOR_1_0_MIGRATION.md` | Anchor 0.31 → 1.0.0 notes | The two API breakages (re-exports, Context lifetime) + per-program changes. |
| `docs/v7-universal-payout.md` | NodeVault + streaming-VRF design | Why payout binding is hardware-signed. |
| `docs/DICE_Complete_Architecture.docx` | Full architecture spec (Word) | Shared with external reviewers. |
| `docs/DICE_Tech_Stack_OpSec.docx` | Tech stack + opsec writeup (Word) | Used in vendor / investor diligence. |
| `docs/Hariharan_DICE_Beginner.pdf` | Beginner-track explainer PDF | Linked from the frontend's `/docs/getting-started`. |
| `docs/PhysicalVRF_Market_Analysis.docx`, `docs/PhysicalVRF_PitchDeck.pdf` | Early market + pitch docs | Kept for narrative continuity. |
| `marketing/` | HTML→PDF build kit (Playwright) | 12-slide deck, 4-up product cards, operator/dev how-to cards, 6-chapter brandbook. `pnpm pdf` renders to `dist/`. |
| `marketing/build-pdfs.mjs` | Chromium PDF renderer | Single-step build script. |
| `marketing/video-scripts/` | Working scripts for narrative videos | Drafts for the weekly-update shoots. |
| `marketing/src/` | HTML source for all marketing assets | Slides, cards, branding. |
| `pitch_deck/` | Investor pitch (HTML + PDF) | `dice_pitch.html`, `dice_pitch.pdf`, `dice_dev_docs.html`. |
| `research/` | Market + competitor + novel-delivery reports | VRF/DePIN ecosystem, Switchboard weaknesses, AI-provenance market, trusted-time market, web3 mentions. Published as both `.md` and `.html`. |
| `shoot/` | Weekly-update HTML scripts | `week-1-update.html`, `week-2-update.html`, 7 narrative shorts (the-discovery, the-eight-dollar-chip, the-trust-problem, the-builder, the-missed-flight, the-platform, launch). |
| `how-it-works/` | Long-form explainer pages | 6 numbered HTML chapters (architecture, VRF flow, developer integration, payment, security, honest assessment) + index. |
| `packaging/` | Product packaging collateral | `welcome-card.html` (ships with every unit), `manual.html` (operator setup), `enclosure-ideas.md` (3D-print notes). |

---

## Archive (moved here 2026-04-21)

| Entry | What | Why archived |
|---|---|---|
| `archive/legacy/orchestrator.py` | Phase-0-to-5 build orchestrator (v1/v2 era) | Superseded by direct `cargo` / `anchor` / `pnpm` invocation. Kept for history. |
| `archive/legacy/Makefile` | Phase targets that drove `orchestrator.py` | Same — flow is obsolete. |
| `archive/legacy/start_production.bat` | Old Windows prod launcher | **Contains a hardcoded Neon Postgres password committed in git history — rotate that credential.** Superseded by `deploy/coord-do/`. |
| `archive/legacy/build_dice_wsl.sh` | WSL-only build helper | Functionality absorbed into `anchor build --no-idl` + `cargo check`. |
| `archive/v7-test-dashboards/test_v7.md` | v7 (pre-7.7) test plan + narrative | Superseded by `docs/PROGRESS.md` + run files in `test_v7_results/`. |
| `archive/v7-test-dashboards/test_v7_dashboard.html` + `_server.py` | Local HTTP dashboard for v7 runs | Dashboard now lives inside the coordinator (`http://localhost:8080/`). |
| `archive/v7-test-dashboards/test_v7_report.html` | v7 run report render | Superseded by per-run files in `test_v7_results/`. |
| `archive/v7-test-dashboards/test_v7_latency_report.html` | v7 latency report render | Superseded by `test_v7_results/v77_latency_50.json`. |
| `archive/v7-test-dashboards/test_v7_expense_report.html` | v7 cost-per-round report | Still useful as a template; not part of the v7.7 flow. |
