# Deploying DICE to the cloud

Two services ship together:

1. **Frontend + BFF** on **Vercel** (Next.js 16) — pulls from this monorepo, builds only from `frontend/`.
2. **Coordinator** on **Railway** (or Fly.io) — long-running Rust binary that holds the mTLS WebSocket for ESP32 devices and submits on-chain TXs.

Devnet launch target: free-tier Vercel + ~$5/mo Railway + existing Neon Postgres + existing Helius RPC key.

---

## 1 · Frontend (Vercel)

### One-time Vercel setup

1. Log in at <https://vercel.com> (GitHub OAuth).
2. **Add New Project** → import `hariFED/DICE`.
3. In the import screen:
   - **Root Directory**: **`frontend`** (click "Edit" and pick the folder)
   - **Framework Preset**: `Next.js` (auto-detected)
   - **Build Command**: leave as framework default (`next build`) — `vercel.json` at the repo root overrides this if needed.
   - **Install Command**: `pnpm install --frozen-lockfile`
4. Environment variables (Project Settings → Environment Variables):

   | Name | Value | Scope |
   |---|---|---|
   | `COORD_INTERNAL_URL` | `https://<your-coord>.up.railway.app` | Production, Preview |
   | `PREORDER_FORWARD_URL` | Formspree / Supabase / Google Apps Script URL (optional) | Production |
   | `NEXT_PUBLIC_API_URL` | same as `COORD_INTERNAL_URL` for now (legacy) | Production, Preview |

5. Click **Deploy**. First build ≈ 2 min.
6. After it's live, **Settings → Domains** → add your custom domain (e.g. `dice.fm`). Vercel issues the TLS cert.

### Path-based build skipping

Root-level `vercel.json` is already wired:

- `ignoreCommand` uses `git diff` to skip a build when nothing under `frontend/` or `vercel.json` changed on the push. So a commit that only touches `coordinator/src/*.rs` won't burn your Vercel build quota.

### Preview deploys

Every PR against any branch gets a `https://<project>-<branch>.vercel.app` preview. BFF routes hit the same `COORD_INTERNAL_URL` you set as Production — fine for devnet; switch to a staging coord for mainnet.

---

## 2 · Coordinator (Railway)

### One-time Railway setup

1. Log in at <https://railway.app>.
2. **New Project** → Deploy from GitHub → pick `hariFED/DICE`.
3. **Add a service** with the following custom settings:
   - **Root Directory**: `/` (the coord Cargo build needs the workspace)
   - **Build Command**: `cargo build --release --bin dice-coordinator`
   - **Start Command**: `./target/release/dice-coordinator`
   - **Watch Paths**: `coordinator/**, programs/dice/**, sdk/dice-vrf/**, Cargo.toml, Cargo.lock` (so frontend commits don't redeploy coord)
4. Environment variables (Service Settings → Variables):

   | Name | Value |
   |---|---|
   | `DATABASE_URL` | Neon postgres URL (`?sslmode=require&channel_binding=require`) |
   | `SOLANA_RPC_URL` | `https://devnet.helius-rpc.com/?api-key=…` |
   | `DICE_TREASURY` | `C2JugYQztp1XDGG1ZCagbqRivqGsmE1vG1uMHaMHPDaQ` |
   | `DICE_RESERVE`  | `3df8FZoosdv3mrYwWS82TEqQps97qAdmnnijUNhz6tp9` |
   | `DICE_WS_PORT`  | `8443` |
   | `DICE_API_PORT` | `8080` |
   | `DICE_TLS`      | `true` |
   | `DICE_TLS_CERT_PATH` | `/app/certs/coordinator.crt` |
   | `DICE_TLS_KEY_PATH`  | `/app/certs/coordinator.key` |
   | `DICE_CA_CERT_PATH`  | `/app/certs/ca.crt` |
   | `DICE_KEYPAIR_PATH`  | `/app/coordinator-keypair.json` |

5. **Secret volumes** — attach the following files as Railway secret files (not env vars):
   - `coordinator-keypair.json` (the Solana upgrade-authority keypair)
   - `certs/coordinator.crt`, `certs/coordinator.key`, `certs/ca.crt`

6. **Expose ports**:
   - Internal: `8080` (HTTP API) → Railway-generated public URL → this is what goes in Vercel's `COORD_INTERNAL_URL`.
   - Internal: `8443` (mTLS WebSocket) → needs a **TCP proxy** (Railway paid plan supports this). Devices connect here.

### DNS (optional but recommended)

- Point `api.dice.fm` at Railway's `:8080` service → `COORD_INTERNAL_URL = https://api.dice.fm`
- Point `ws.dice.fm` at the TCP-proxy service → firmware connects to `wss://ws.dice.fm:8443`

---

## 3 · Device (ESP32-S3) coord URL update

Each device's firmware has the coord's WebSocket URL saved in NVS. After the Railway coord is live:

1. Hold **BOOT** button for ~5 s → captive portal comes up on the device's WiFi AP (`DICE-<mac>`).
2. Connect to the AP from your phone. Browser auto-opens the portal.
3. Fill in:
   - WiFi SSID + password (home network)
   - Coordinator WebSocket URL: `wss://ws.dice.fm:8443` (or the Railway URL)
4. Device reboots, connects, re-sends PayoutBindingRequest → coord registers NodeVault on v7.7 program.

Repeat for all 5 devices. ~5 min each.

---

## 4 · Pre-launch checklist

Before hitting Deploy:

- [ ] `git status` clean on `v7.7`
- [ ] `pnpm build` from `frontend/` exits 0 locally — **already verified ✓**
- [ ] Coord `cargo build --release` exits 0 locally — **already verified ✓**
- [ ] No secrets in git (`git ls-files | grep -E 'keypair|\\.env$|\\.key$'` is empty) — **verified ✓**
- [ ] `DICE_KEYPAIR_PATH` file is NOT in git (gitignored `*-keypair.json`) — **verified ✓**
- [ ] Vercel env vars set: `COORD_INTERNAL_URL`
- [ ] Railway secret files uploaded: coordinator keypair + 3 cert files
- [ ] At least one ESP32 device reflashed to point at the Railway WS URL
- [ ] Devnet wallet funded ≥ 0.5 SOL for a couple hundred VRF rounds

---

## 5 · Smoke test after deploy

1. Visit the Vercel URL → `/explorer` should load without console errors.
2. `GET https://<vercel-url>/api/v1/stats` → returns `{rounds_total, active_nodes, …}` from Railway coord.
3. Hit `/preorder` → submit the form → check Vercel logs for `[preorder]` line (or Formspree inbox if wired).
4. From a laptop, run the stress driver against the same program-id pointing at the Railway coord RPC:
   ```
   ./target/release/stress-driver.exe \
     --rpc-url https://devnet.helius-rpc.com/?api-key=YOUR_KEY \
     --channel-index 1100 --rounds 10 --node-count 4 --prefund-sol 0.03
   ```
5. `/explorer` should show the rounds live.

---

## 6 · Costs (devnet launch)

| Service | Tier | Cost |
|---|---|---|
| Vercel | Hobby (personal, non-commercial) → Pro for custom domain | $0 → $20/mo |
| Railway | Hobby — small Rust service, ~256 MB RAM | ~$5/mo |
| Neon | Free tier (3 GB) | $0 |
| Helius | Devnet free tier (fine for now) | $0 |
| Domain | Namecheap / Porkbun | $10–15/yr |

Total: **$0–35/mo** before any revenue.

---

## 7 · Going mainnet (later)

When you're ready for mainnet, the only code delta is the `SOLANA_RPC_URL` env var. Everything else is config-level:

1. Fresh `solana program deploy target/deploy/dice.so` against `solana config set -u mainnet` (~4 SOL on current rent).
2. Fund the new mainnet treasury + reserve wallets.
3. Upgrade Helius plan to something with decent rate limits (~$99/mo).
4. Re-run the 4-device NodeVault rebind against the mainnet program (same BOOT-button flow).

The program source, frontend, and docs are version-less — mainnet and devnet dicenetwork stacks can coexist via two Railway services + two Vercel previews.
