# DICE Coordinator · DigitalOcean deployment

Ship the `dice-coordinator` binary to a $6/mo DigitalOcean Droplet. Same
behavior as Fly.io (native raw-TCP mTLS on 8443, HTTP REST on 8080), just
without the payment headaches.

## What you need before starting

- DigitalOcean account (use https://m.do.co/c/ for $200 credit)
- A public SSH key at `~/.ssh/id_ed25519.pub` (or any other pubkey you want)
- Your local secrets already in place:
  - `certs/{ca,coordinator}.{crt,key}`
  - `coordinator-keypair.json` (at repo root)

## Step 1 · Create the Droplet (web UI — 3 minutes)

1. Log in → **Create → Droplets**.
2. Image: **Ubuntu 24.04 (LTS) x64**.
3. Size: **Basic · Regular SSD · $6/mo** (1 GB RAM / 1 CPU / 25 GB disk).
4. Region: **NYC3** or **NYC1** (close to the primary Neon in us-east-1).
5. Authentication: **SSH Key** → add your key (or upload `id_ed25519.pub`).
   Do **not** use password auth.
6. Hostname: `dice-coord`.
7. **Create Droplet**. Note the IPv4 address it assigns you.

## Step 2 · One-time Droplet bootstrap

Replace `<DROPLET_IP>` with the address from step 1. From this repo root:

```bash
# Upload the bootstrap script + run it as root
scp deploy/coord-do/provision.sh root@<DROPLET_IP>:/root/provision.sh
ssh root@<DROPLET_IP> "bash /root/provision.sh"
```

This installs Docker + docker compose, tunes kernel limits, configures UFW
(open 22, 8080, 8443), and creates the `/opt/dice-coord/` directory.

Takes ~90 seconds.

## Step 3 · Push the coord + secrets + start

Fill in `deploy/coord-do/.env` (see `.env.template`). Then:

```bash
export DROPLET_IP=<DROPLET_IP>
bash deploy/coord-do/push.sh
```

`push.sh` does three things:

1. rsyncs the full repo source to `/opt/dice-coord/src/` on the Droplet
   (excludes from `.dockerignore`).
2. scps the cert files + Solana keypair to `/opt/dice-coord/secrets/`.
3. SSHes in and runs `docker compose up -d --build`.

First build takes ~5 min (cargo compiles the release binary). Subsequent
deploys reuse layers and take ~60 s.

## Step 4 · Smoke test

```bash
curl http://<DROPLET_IP>:8080/api/v1/stats
```

Should return JSON with `rounds_total`, `active_nodes`, etc. If yes, you
are live.

## Step 5 · Wire Vercel to the new coord

Set the Vercel env var (Project Settings → Environment Variables):

```
COORD_INTERNAL_URL = http://<DROPLET_IP>:8080
```

Then redeploy the frontend (Vercel → Deployments → ⋯ → Redeploy).

Verify from https://dice-ten-ashen.vercel.app/explorer — live stats should
populate within 5 s.

## Step 6 · Flash ESP32 devices to point at the new mTLS endpoint

On each device: BOOT-button hold 5 s → captive portal → set WebSocket URL
to `wss://<DROPLET_IP>:8443`. Device reboots and joins the mesh.

## Upgrading to a real domain (optional, later)

Once you have a domain (e.g. `dicelabs.network`):

1. Point `api.dicelabs.network` (A record) at the Droplet IP.
2. Add Caddy as a reverse proxy for HTTPS on the API — see
   `deploy/coord-do/Caddyfile.example` (auto-issues Let's Encrypt cert).
3. Keep `ws.dicelabs.network` pointing directly at the Droplet IP — mTLS
   does its own TLS, no proxy needed.

## Logs / troubleshooting

```bash
ssh root@<DROPLET_IP>
cd /opt/dice-coord
docker compose logs -f coord          # tail live logs
docker compose ps                      # container health
docker compose restart coord           # restart after config tweak
docker compose down && docker compose up -d --build   # rebuild from scratch
```

## Costs

- Droplet: **$6/mo** (hourly billing — destroy any time)
- Neon Postgres: $0 (free tier)
- Helius devnet RPC: $0
- **Total: $6/mo**
