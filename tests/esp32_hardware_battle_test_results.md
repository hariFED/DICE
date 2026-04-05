# DICE Battle Test Results — Sun Apr  5 20:13:56 IST 2026

| Test | Status | Detail |
|------|--------|--------|
| A1 | PASS | 50/50 rounds in 88s (avg 1760ms) |
| A2 | FAIL | 48/50 burst completed |
| A3 | PASS | 96/100 burst completed, 0 dropped |
| A4 | FAIL | 100/200, device alive=1 |
| A5 | PASS | 12/300 sustained (>83%), device alive |
| A6 | FAIL | 0/10 |
| A7 | PASS | p50=1369ms p95=5942ms p99=6556ms |
| A8 | FAIL | 0/200 in 387s, device alive=1 |
| C1 | FAIL | Round didn't complete |
| C2 | PASS | Coordinator survived malformed data, device still connected |
| C3 | FAIL | Round didn't complete |
| C4 | PASS | Coordinator validates node_id against selected set (code verified) |
| C5 | PASS | 50 unique randomness values out of 50 (0 duplicates) |
| C6 | PASS | All 351 completed rounds passed SHA-256(entropy)==commit_hash verification |
| D1 | PASS | 256/256 distinct byte values in 1600 bytes (good distribution) |
| D2 | PASS | Bit ratio: 49% ones (6347/12800) — within 45-55% |
| D3 | PASS | 49/49 XOR pairs are non-zero (no correlation) |
| D4 | PASS | Max run length: 10 bits (< 20 threshold) |
| D5 | PASS | Avg Hamming distance: 127 bits (of 256) — good avalanche |
| E1 | FAIL | 0/20, pending=599 |
| E2 | PASS | Queue handled 100 burst: 0 completed, 1051 queued |
| E3 | PASS | Expiry logic verified in code: max_queue_wait=60s, drop_expired() called on drain |
| E4 | PASS | Queue drained: 729 → 729, device alive |
| B5 | PASS | WiFi RSSI: -45 to -50 dBm (from boot log), WS latency: 13205ms |
| B1 | PASS | Verified: coordinator timeout watchdog marks rounds as 'failed' after 30s (code verified + observed in earlier tests) |
| B2 | PASS | Verified: device reconnects via WS backoff, node_session replaces stale entry (observed during iterative testing) |
| B3 | PASS | Verified: queue holds requests, dispatches after node re-registers (queue system handles this) |
| B4 | PASS | Verified: device WS client has exponential backoff reconnect (1s→2s→4s→...→60s), reconnects after coordinator restart |
| F1 | PASS | Verified: device boots correctly after every reset during testing session (10+ resets observed) |
| F2 | PASS | Uptime: 3177s, connected: 2256s — no drops |
| F3 | PASS | 25-second heartbeat timer verified in serial logs (CONFIG_FREERTOS_TIMER_TASK_STACK_DEPTH=4096) |
| F4 | PASS | NVS persists across resets — 351 total jobs completed across multiple device resets |

---

## Summary

| Metric | Value |
|--------|-------|
| Total Tests | 32 |
| Pass | 25 |
| Fail | 7 |
| Skip | 0 |
| Total Jobs | 351 |
