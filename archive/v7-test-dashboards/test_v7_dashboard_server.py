#!/usr/bin/env python3
"""
DICE v7 test dashboard server.

Tiny single-file HTTP server that:
  - serves test_v7_dashboard.html at /
  - proxies http://127.0.0.1:8080/nodes          at /proxy/nodes
  - proxies http://127.0.0.1:8080/api/v1/stats   at /proxy/stats
  - parses test_v7_results/A4.log (or whatever --log) and returns structured
    progress JSON at /api/progress (counts, latency stats, latest TX,
    recent event log, rolling histogram).

No external dependencies. Python 3.8+ stdlib only.

Usage:
    python test_v7_dashboard_server.py
    # then open http://localhost:9000 in your browser

    # or point at a different log file:
    python test_v7_dashboard_server.py --log test_v7_results/A4.log --port 9000
"""

import argparse
import json
import os
import re
import time
import urllib.request
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

# Line patterns produced by stress_driver.
#   [  12/1000] round_id=12  9355ms ✓ rand=6833abb3b5cbb9df...
#   [  27/1000] FAIL in 60918ms — round timed out waiting for Finalized
#   total time:   95486 ms (95.5 s)
#   required channel balance for 1000 rounds: 2000000000 lamports
RE_OK = re.compile(
    r"\[\s*(\d+)/(\d+)\]\s+round_id=(\d+)\s+(\d+)ms.+rand=([0-9a-f]+)"
)
RE_FAIL = re.compile(
    r"\[\s*(\d+)/(\d+)\]\s+FAIL in (\d+)ms\s+—\s+(.+)"
)
RE_REQUIRED = re.compile(r"required channel balance for (\d+) rounds")
RE_TX_REQ = re.compile(r"request_randomness_auto TX: ([1-9A-HJ-NP-Za-km-z]{50,})")
RE_CHANNEL_PDA = re.compile(r"channel PDA:\s+([1-9A-HJ-NP-Za-km-z]{32,})")


def percentile(sorted_vals, p):
    if not sorted_vals:
        return 0
    idx = int((len(sorted_vals) - 1) * p)
    return sorted_vals[idx]


def parse_log(log_path: Path):
    """Parse the stress-driver log and return a progress dict."""
    if not log_path.exists():
        return {
            "available": False,
            "total": 0,
            "done": 0,
            "succeeded": 0,
            "failed": 0,
            "events": [],
            "recent_durations": [],
        }

    total = 0
    rounds_ok = []   # list of (index, round_id, duration, rand, tx)
    rounds_fail = [] # list of (index, duration, reason)
    events = []      # recent events for the UI log
    latest_tx = None
    latest_rand = None
    first_ts = None
    last_ts = log_path.stat().st_mtime
    channel_pda = None
    all_durations = []

    with log_path.open("r", encoding="utf-8", errors="replace") as f:
        for line in f:
            if first_ts is None:
                first_ts = time.time()  # approximate

            m = RE_REQUIRED.search(line)
            if m:
                # stress_driver prints this once at the start
                total = int(m.group(1))
                continue

            m = RE_CHANNEL_PDA.search(line)
            if m:
                channel_pda = m.group(1)
                continue

            m = RE_OK.search(line)
            if m:
                idx = int(m.group(1))
                total = max(total, int(m.group(2)))
                round_id = int(m.group(3))
                dur = int(m.group(4))
                rand = m.group(5)
                rounds_ok.append((idx, round_id, dur, rand))
                all_durations.append(dur)
                latest_rand = rand[:16]
                events.append({
                    "time": f"r{idx}",
                    "kind": "ok",
                    "msg": f"round_id {round_id} finalized in {dur} ms — rand {rand[:16]}…",
                })
                continue

            m = RE_FAIL.search(line)
            if m:
                idx = int(m.group(1))
                total = max(total, int(m.group(2)))
                dur = int(m.group(3))
                reason = m.group(4).strip()
                rounds_fail.append((idx, dur, reason))
                events.append({
                    "time": f"r{idx}",
                    "kind": "fail",
                    "msg": f"FAIL in {dur} ms — {reason[:120]}",
                })
                continue

            m = RE_TX_REQ.search(line)
            if m:
                latest_tx = m.group(1)
                continue

    done = len(rounds_ok) + len(rounds_fail)
    succeeded = len(rounds_ok)
    failed = len(rounds_fail)
    durations_ok = sorted([r[2] for r in rounds_ok])
    avg_ms = sum(durations_ok) / len(durations_ok) if durations_ok else None

    # Elapsed from file mtime - first line (approximate)
    elapsed_ms = int((last_ts - (first_ts or last_ts)) * 1000)
    if elapsed_ms < 0:
        elapsed_ms = 0

    # Rough ETA: (remaining / done) * elapsed, if at least 3 rounds done
    eta_ms = None
    if done >= 3 and done < total:
        rate_ms_per_round = elapsed_ms / max(1, done)
        # fallback if elapsed is tiny (the mtime-based estimate is noisy)
        if avg_ms and avg_ms > rate_ms_per_round:
            rate_ms_per_round = avg_ms
        eta_ms = int(rate_ms_per_round * (total - done))

    # Most recent event first — keep last 60
    events = events[-60:]

    # Rolling duration histogram — last 60 rounds (OK + FAIL mixed)
    recent = []
    for idx, _, dur, _ in rounds_ok[-60:]:
        recent.append(dur)
    for idx, dur, _ in rounds_fail[-60:]:
        recent.append(dur)

    current_round_id = rounds_ok[-1][1] if rounds_ok else None

    return {
        "available": True,
        "channel_pda": channel_pda,
        "total": total,
        "done": done,
        "succeeded": succeeded,
        "failed": failed,
        "avg_ms": avg_ms,
        "min_ms": durations_ok[0] if durations_ok else None,
        "p50_ms": percentile(durations_ok, 0.50) if durations_ok else None,
        "p95_ms": percentile(durations_ok, 0.95) if durations_ok else None,
        "p99_ms": percentile(durations_ok, 0.99) if durations_ok else None,
        "max_ms": durations_ok[-1] if durations_ok else None,
        "elapsed_ms": elapsed_ms,
        "eta_ms": eta_ms,
        "current_round_id": current_round_id,
        "latest_tx": latest_tx,
        "latest_randomness": latest_rand,
        "events": events,
        "recent_durations": recent,
    }


class DashboardHandler(BaseHTTPRequestHandler):
    # Silence default HTTP access log spam
    def log_message(self, format, *args):
        return

    def _send(self, status, body, content_type="application/json"):
        data = body.encode("utf-8") if isinstance(body, str) else body
        self.send_response(status)
        self.send_header("Content-Type", content_type)
        self.send_header("Content-Length", str(len(data)))
        self.send_header("Cache-Control", "no-store")
        self.end_headers()
        self.wfile.write(data)

    def _proxy(self, upstream_path):
        url = f"http://127.0.0.1:8080{upstream_path}"
        try:
            req = urllib.request.Request(url, headers={"Accept": "application/json"})
            with urllib.request.urlopen(req, timeout=3) as resp:
                body = resp.read()
                self._send(resp.getcode(), body, "application/json")
        except Exception as e:
            self._send(502, json.dumps({"error": f"coordinator unreachable: {e}"}))

    def do_GET(self):
        path = self.path.split("?", 1)[0]

        if path in ("/", "/index.html"):
            html_path = Path(__file__).parent / "test_v7_dashboard.html"
            if html_path.exists():
                self._send(200, html_path.read_bytes(), "text/html; charset=utf-8")
            else:
                self._send(404, "dashboard html missing", "text/plain")
            return

        if path == "/proxy/nodes":
            self._proxy("/nodes")
            return

        if path == "/proxy/stats":
            self._proxy("/api/v1/stats")
            return

        if path == "/api/progress":
            log_path = self.server.log_path
            progress = parse_log(log_path)
            self._send(200, json.dumps(progress))
            return

        self._send(404, "not found", "text/plain")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--port", type=int, default=9000)
    ap.add_argument("--log", type=str, default="test_v7_results/A4.log",
                    help="path to stress-driver log to parse")
    args = ap.parse_args()

    log_path = Path(args.log).resolve()
    print(f"DICE dashboard server")
    print(f"  listening on http://localhost:{args.port}")
    print(f"  parsing log: {log_path}")
    print(f"  proxying coordinator at http://127.0.0.1:8080")
    print(f"  open http://localhost:{args.port} in your browser")
    print()

    httpd = ThreadingHTTPServer(("127.0.0.1", args.port), DashboardHandler)
    httpd.log_path = log_path
    try:
        httpd.serve_forever()
    except KeyboardInterrupt:
        print("shutting down")


if __name__ == "__main__":
    main()
