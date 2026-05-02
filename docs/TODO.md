# DICE — Next Steps

> **Last updated:** 2026-04-21
> **Branch:** `v7.7`
> **Repo:** https://github.com/hariFED/DICE (private)
> **Status:** v7.7 program live on devnet (`FMwPuCjkfZXN2MuNJQiUzZC3hnxHcD8mrTuntsqA84XD`). Anchor 1.0.0 migration done. Frontend v5 redesign live at https://dice-ten-ashen.vercel.app. Marketing kit shipped under `marketing/`. Coord deploy kit ready under `deploy/coord-do/` but not executed (DO Droplet pending). 229 Rust tests passing, latency 8s → 3.9s on v7.5+v7.7.

---

## v7.7 — Shipped (2026-04-21)

- [x] **v7.5 latency**: ALT-bundled `submit_round_v2` + `claim_rewards_v2` in a single TX. Avg 3.9 s (was 8 s).
- [x] **v7.3 trustless selection**: `request_randomness_auto` now CPIs to `select_nodes` internally. Coordinator can't bias quorum.
- [x] **Anchor 1.0.0 migration**: workspace bump, all 6 programs build clean, redeployed as new program ID to dodge IDL cache.
- [x] **TypeScript SDK**: `@dice-network/sdk` shipped (in repo at `sdk/ts/`, not yet pushed to npm).
- [x] **Frontend v5**: editorial redesign — logo, cobe globe, exploded ESP32, animated protocol-flow, beginner docs, 4-step pre-order flow.
- [x] **Marketing kit**: `marketing/` folder — 12-slide deck + product cards + how-to cards + brandbook + Playwright HTML→PDF script.
- [x] **DigitalOcean deploy kit**: `deploy/coord-do/` — provision script + push script + compose file + populated `.env`. Ready to run.
- [x] **Frontend deployed to Vercel prod**: https://dice-ten-ashen.vercel.app · 34 static pages · 0 build errors.

## v7.7 — Open (deferred / blocked)

- [ ] **Task #19** — Deploy coordinator to DigitalOcean Droplet. **Blocked on user.** Fly.io rejected card; pivoted to DO. Kit is ready; needs the user to create the $6/mo Droplet, share IP, then I run `provision.sh` + `push.sh`.
- [ ] **Task #35 (HIGH)** — v7.7 NodeVault rebind + cross-program callback test. Confirms `register_node_vault` + `claim_rewards_v2` still wire correctly on the fresh program.
- [ ] **Task #22** — v7 stress + adversarial test suite (mid-run). Long tail; finish before mainnet conversation.
- [ ] **Task #34** — v7.7 deploy as NEW program + bench on real hw. Program is up; bench against the bound 5-device mesh after they're rebound (depends on #35).

## Pre-launch / outreach checklist

- [ ] **Tuesday 2026-04-22**: marketing push starts — social + outreach to 5 integrator prospects
- [ ] **Pre-orders**: form is live at /preorder (4-step Stripe-style); monitor Vercel logs for first submissions
- [ ] **3D enclosures**: first batch printing; ship to a small group of known developers for real-world test
- [ ] **Update DEPLOY.md**: swap Railway-first instructions for DO-first (the kit is at `deploy/coord-do/`)

---

## What's Done (historical, kept for context)

### v7 (2026-04-14)
- [x] NodeVault payout primitive (`register_node_vault`, `rotate_payout_wallet`, `withdraw_from_vault`, `claim_rewards_v2`)
- [x] Streaming VRF (`init_feed`, `publish_feed_value`, `close_feed`) + SDK subscriber example
- [x] Hardware-signed `PayoutBindingRequest` over mTLS
- [x] CORS + `/api/v1/stats` for public frontend
- [x] Real ESP32-S3 binding TX on devnet (`5PzuCRN9...`)

### v3 (earlier)
- [x] 545+ VRF rounds on real ESP32-S3 hardware, 0 crashes, 1.7 s avg latency
- [x] Captive-portal onboarding + LED status + NVS auto-provisioning
- [x] mTLS + PostgreSQL + 3 example dApps

---

## Priority 1 — Get the coord on real internet

**Why:** All v7.7 work has been against `localhost`. Until the coord is on a public IP, ESP32 devices outside our LAN can't connect, integrators can't test, and any "live network" claim is technically a lie.

**Plan:**
- [ ] User signs up for DigitalOcean (use https://m.do.co/c/ for $200 credit) and creates a $6/mo NYC3 Droplet
- [ ] Run `bash deploy/coord-do/provision.sh` on the Droplet (one-time bootstrap — Docker, UFW, limits)
- [ ] From local, `export DROPLET_IP=… && bash deploy/coord-do/push.sh` (rsyncs source via tar-over-ssh, uploads secrets, brings up coord)
- [ ] Smoke test: `curl http://$DROPLET_IP:8080/api/v1/stats`
- [ ] Update Vercel `COORD_INTERNAL_URL` env → trigger redeploy → live stats start flowing on /explorer
- [ ] Reflash 5 ESP32 devices' WS URL via captive portal → `wss://$DROPLET_IP:8443`
- [ ] Run a 50-round bench against the public coord — confirm latency holds at ~4 s

**Optional follow-ups (after first deploy):**
- [ ] Point a real subdomain (e.g. `coord.dicelabs.network`) at the IP via Cloudflare → free TLS for the API path
- [ ] Move `8080` HTTP behind Caddy for Let's Encrypt cert auto-issue
- [ ] Set up daily Postgres backup → S3 (or Neon's built-in PITR is enough)

---

## Priority 2 — Validate streaming VRF demand BEFORE any engineering

**Why:** Streaming VRF (the `RandomnessFeed` PDA) is shipped and works at ~3-second cadence. But the natural next product — **DICE Stream**, a ms-latency WebSocket feed — does not exist on Solana from anyone. That could mean either "we found a category" or "no one wants it yet." Validate before building.

**Plan:**
- [ ] Identify 5 target buyers across live games, prediction markets, and high-frequency DeFi (e.g. BonkArcade, Drift, Zeta, a launchpad operator, a poker dApp)
- [ ] Run 30-min discovery calls. Question script:
  1. "Today, when you need randomness, where do you get it from? What's the pain?"
  2. "If randomness arrived in 50–200 ms instead of 4 s, what would you build that you can't build today?"
  3. "Would you pay a subscription for that — say $50/mo per stream, $500/mo enterprise — or do you need per-request pricing?"
  4. "What's the one trust property you'd refuse to compromise on?"
  5. "If we had this in 4 weeks, would you commit to a paid pilot?"
- [ ] If 3 of 5 say yes with a concrete dollar number → write the protocol design doc + open a v8 branch
- [ ] If <3 say yes → file the section under "validated, market wasn't ready" and revisit in 6 months

**Why this is on the priority list at all:** Because streaming VRF without ms-latency is the missing piece for live games on Solana, and live games are a real (if small) growing market. The customer-dev step is one week of phone calls; the build is 4 weeks if validated. Five wasted phone calls is cheap; six wasted engineering weeks is not.

See `PROGRESS.md` → "Future Perspectives" for the full thinking.

---

## Priority 3 — TypeScript SDK to npm

**Why:** SDK is built and tested locally but `@dice-network/sdk` isn't published. Until it's on npm, integrators can't `npm install` and we lose every "first 5 minutes" story.

**Plan:**
- [ ] Add `npm publish` workflow under `.github/workflows/sdk-publish.yml` (manual trigger, semver via npm version)
- [ ] Verify package.json `files` field includes only `dist/`
- [ ] Run `npm publish --dry-run` to inspect tarball
- [ ] First publish as `0.1.0-beta.1` → tag on GitHub → announce
- [ ] Update `/docs/quickstart` to use real `npm i @dice-network/sdk` install line

---

## Priority 4 — Smart contract polish

- [ ] **Backport `register_node_vault::verify_binding_signature` fix into `submit_reveal.rs`** (task #13) — secp256k1 recovery-ID `.or_else` chain. Latent. Not v7-blocking but cheap to fix.
- [ ] **Anchor integration tests on bankrun** — replace devnet-dependent tests with bankrun for CI speed
- [ ] **Trident fuzz testing setup** — long-deferred. Start with `request_randomness_v2` + `submit_reveal_v2`.
- [ ] **External security audit** — OtterSec / Neodyme / Halborn quote. Required before mainnet.

---

## Priority 5 — Hardware + provisioning at scale

- [ ] **Air-gapped Root CA ceremony** — currently using a dev CA. For production the root signs an intermediate, root goes offline.
- [ ] **Automated provisioning script** — current flow is `provision_dev.py`; needs to handle Secure Boot v2 eFuses + Flash Encryption for production-grade firmware
- [ ] **Batch flash 20 devices** — production line tooling, log device manifest to a JSON file
- [ ] **Certificate rotation procedure** — what happens when a device's mTLS cert expires
- [ ] **3D enclosures**: first batch printing; design files in `packaging/` (assumed)

---

## Priority 6 — Observability + ops

- [ ] **Geyser plugin detector** — replace WS `logsSubscribe` with Geyser gRPC stream for ~100 ms detection latency (vs ~500 ms WS). Pluggable via `RequestDetector` trait.
- [ ] **Geyser-as-a-service** — Helius / Triton integration for teams without self-hosted validators
- [ ] **Grafana dashboard** — Prometheus scrape via SSH tunnel into the DO Droplet, surface latency / queue depth / per-node uptime
- [ ] **PagerDuty / Discord webhook** on round-failure rate spike

---

## Priority 7 — Mainnet path (later, after audit)

- [ ] Deploy `dice` v7.7 to mainnet (~4 SOL rent at current size)
- [ ] Fund mainnet treasury + reserve from operator wallet
- [ ] Helius mainnet plan (~$99/mo for sane rate limits)
- [ ] Re-run the device NodeVault rebind against the mainnet program
- [ ] Squads multisig for upgrade authority (2-of-3)
- [ ] HashiCorp Vault or SOPS for secrets management (currently they're in `deploy/coord-do/.env` — fine for devnet, not for mainnet)

---

## Future Perspectives (validate before building)

The same list lives in `PROGRESS.md` — copied here so the TODO file is self-contained.

- **DICE Stream (ms-latency WebSocket VRF)** — see Priority 2 above. Gated on customer-dev validation.
- **EVM port** — not a core goal. Solana-specific economics are what makes $0.002/request work.
- **DAO / governance / token** — explicitly off the table per the brand book. Don't reopen without a product reason.
- **Hosted node-farm** — alternative to operator distribution. Eats margin but removes supply-chain friction. Revisit if operator acquisition stalls.
- **FPGA / ASIC upgrade** — only relevant at >1000 operators. Today's bottleneck is demand, not unit cost.

---

## Key Addresses (v7.7)

| What | Address |
|------|---------|
| **Program ID (v7.7)** | `FMwPuCjkfZXN2MuNJQiUzZC3hnxHcD8mrTuntsqA84XD` |
| Program ID (v7, deprecated) | `78Qv6cyKkRZN2YngiLSSBCe2iyRc6jgtCs3incCaMRcv` |
| Coin Toss (v2) | `7r6UstdP6qTFK4HSqU4mFGPGyCVWd3JVjBeafQPyvspH` |
| Coordinator Wallet | `3df8FZoosdv3mrYwWS82TEqQps97qAdmnnijUNhz6tp9` |
| Treasury Wallet | `C2JugYQztp1XDGG1ZCagbqRivqGsmE1vG1uMHaMHPDaQ` |

---

## Quick Resume Commands

```bash
# Workspace sanity
cargo check --workspace --message-format=short
cargo test  --workspace --message-format=short

# Frontend
cd frontend && pnpm dev          # local dev server :3000
cd frontend && pnpm build        # prod build → 34 static pages

# Run coord locally (sim mode, no DB)
cargo run --bin dice-coordinator -- --simulation
cargo run --bin mock-firmware-node -- --count 6
# then POST /simulate to trigger a round

# Run coord against real Neon + devnet
DATABASE_URL=$NEON_URL \
SOLANA_RPC_URL=$HELIUS_URL \
DICE_TREASURY=C2JugYQztp1XDGG1ZCagbqRivqGsmE1vG1uMHaMHPDaQ \
DICE_RESERVE=3df8FZoosdv3mrYwWS82TEqQps97qAdmnnijUNhz6tp9 \
cargo run --bin dice-coordinator -- --tls

# Build BPF binary (WSL, Anchor 1.0)
anchor build --no-idl

# Deploy v7.7 to devnet (WSL)
solana program deploy target/deploy/dice.so \
  --url devnet --keypair coordinator-keypair.json \
  --program-id FMwPuCjkfZXN2MuNJQiUzZC3hnxHcD8mrTuntsqA84XD

# DigitalOcean coord deploy (after Droplet is up)
export DROPLET_IP=<ip>
bash deploy/coord-do/push.sh

# Marketing kit → PDF
cd marketing && pnpm install && pnpm pdf
```
