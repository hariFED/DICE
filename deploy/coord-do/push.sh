#!/usr/bin/env bash
# Sync repo + secrets to the Droplet, then bring up the coord.
# Run FROM the repo root after provision.sh has succeeded.
#
#   export DROPLET_IP=1.2.3.4
#   bash deploy/coord-do/push.sh
#
# Uses tar-over-ssh instead of rsync so it runs unchanged on Git Bash
# for Windows. Same net effect — wipes /opt/dice-coord/src on the remote
# and extracts a fresh tree each push. Docker's build cache still reuses
# layers across runs.
#
# Env vars:
#   DROPLET_IP   — required. Droplet public IPv4.
#   DROPLET_USER — optional, default: root
#   SSH_KEY      — optional; path to private key (ssh default otherwise)

set -euo pipefail

: "${DROPLET_IP:?DROPLET_IP not set — export DROPLET_IP=<ip> first}"
DROPLET_USER="${DROPLET_USER:-root}"
SSH_CMD=(ssh -o StrictHostKeyChecking=accept-new)
SCP_CMD=(scp -o StrictHostKeyChecking=accept-new)
if [[ -n "${SSH_KEY:-}" ]]; then
  SSH_CMD+=(-i "$SSH_KEY")
  SCP_CMD+=(-i "$SSH_KEY")
fi

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$REPO_ROOT"

# ----------------------------------------------------------------- preflight
if [[ ! -f deploy/coord-do/.env ]]; then
  echo "ERROR: deploy/coord-do/.env not found."
  echo "       cp deploy/coord-do/.env.template deploy/coord-do/.env"
  exit 1
fi
for f in certs/ca.crt certs/coordinator.crt certs/coordinator.key coordinator-keypair.json; do
  [[ -f "$f" ]] || { echo "ERROR: missing $f"; exit 1; }
done

REMOTE="$DROPLET_USER@$DROPLET_IP"

echo "[1/4] wipe + re-seed /opt/dice-coord/src on $DROPLET_IP (via tar-over-ssh)"
"${SSH_CMD[@]}" "$REMOTE" "rm -rf /opt/dice-coord/src && mkdir -p /opt/dice-coord/src"

# Build a tarball on the fly and pipe it over ssh. The exclude list mirrors
# .dockerignore so we don't ship target/, node_modules/, etc. across the wire.
tar -czf - \
  --exclude='.git' \
  --exclude='target' \
  --exclude='node_modules' \
  --exclude='frontend/node_modules' \
  --exclude='frontend/.next' \
  --exclude='marketing' \
  --exclude='shoot' \
  --exclude='research' \
  --exclude='test_v7_results' \
  --exclude='*.log' \
  --exclude='.claude' \
  --exclude='.hypothesis' \
  --exclude='__pycache__' \
  --exclude='certs' \
  --exclude='coordinator-keypair.json' \
  --exclude='deploy/coord-do/.env' \
  -C "$REPO_ROOT" . \
  | "${SSH_CMD[@]}" "$REMOTE" "tar -xzf - -C /opt/dice-coord/src"

echo "[2/4] upload secrets → /opt/dice-coord/secrets/  (mode 0600)"
"${SCP_CMD[@]}" -q \
  certs/ca.crt \
  certs/coordinator.crt \
  certs/coordinator.key \
  coordinator-keypair.json \
  deploy/coord-do/.env \
  "$REMOTE:/opt/dice-coord/secrets/"
"${SSH_CMD[@]}" "$REMOTE" "chmod 600 /opt/dice-coord/secrets/*"

echo "[3/4] copy compose file into place"
"${SCP_CMD[@]}" -q \
  deploy/coord-do/docker-compose.yml \
  "$REMOTE:/opt/dice-coord/docker-compose.yml"

echo "[4/4] docker compose up -d --build  (first build ~5 min — stream with logs -f)"
"${SSH_CMD[@]}" "$REMOTE" "cd /opt/dice-coord && docker compose up -d --build"

echo ""
echo "------------------------------------------------------------------"
echo " Deploy complete. Verify:"
echo "   curl http://$DROPLET_IP:8080/api/v1/stats"
echo ""
echo " Tail logs:"
echo "   ssh $REMOTE 'cd /opt/dice-coord && docker compose logs -f coord'"
echo "------------------------------------------------------------------"
