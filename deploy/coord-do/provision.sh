#!/usr/bin/env bash
# One-time Droplet bootstrap. Run as root ON THE DROPLET.
#
#   scp deploy/coord-do/provision.sh root@$IP:/root/
#   ssh root@$IP "bash /root/provision.sh"
#
# Idempotent: safe to re-run. Installs Docker, tunes limits, opens ports,
# creates the deploy directory skeleton.

set -euo pipefail

echo "[1/6] apt update + upgrade"
export DEBIAN_FRONTEND=noninteractive
apt-get update -qq
apt-get upgrade -y -qq

echo "[2/6] install docker + plugins"
if ! command -v docker >/dev/null 2>&1; then
  curl -fsSL https://get.docker.com | sh
fi
# compose plugin ships with the above installer on 20.04+; verify:
docker compose version >/dev/null 2>&1 || apt-get install -y docker-compose-plugin

echo "[3/6] kernel / file-descriptor limits (needed for a WS-heavy server)"
cat >/etc/security/limits.d/dice.conf <<'EOF'
*  soft  nofile  65536
*  hard  nofile  65536
root soft nofile 65536
root hard nofile 65536
EOF
# persist on next boot
grep -q 'fs.file-max' /etc/sysctl.conf || echo 'fs.file-max = 131072' >> /etc/sysctl.conf
sysctl -p >/dev/null

echo "[4/6] firewall — allow 22, 8080, 8443"
ufw --force reset >/dev/null
ufw default deny incoming
ufw default allow outgoing
ufw allow 22/tcp        # SSH
ufw allow 8080/tcp      # Coord HTTP API (consumed by Vercel BFF)
ufw allow 8443/tcp      # Coord mTLS WebSocket (ESP32 devices)
# 9090 (prometheus) intentionally NOT exposed — scrape only via SSH tunnel
ufw --force enable
ufw status

echo "[5/6] create /opt/dice-coord layout"
mkdir -p /opt/dice-coord/src
mkdir -p /opt/dice-coord/secrets
chmod 700 /opt/dice-coord/secrets

echo "[6/6] done"
echo "------------------------------------------------------------------"
echo " Droplet bootstrap complete."
echo " Next: from your local machine, run:"
echo "   export DROPLET_IP=<this droplet's IP>"
echo "   bash deploy/coord-do/push.sh"
echo "------------------------------------------------------------------"
