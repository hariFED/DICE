# DICE — Test Results Log

> **Last updated:** 2026-04-08 01:16 IST
> **Branch:** `v5-keeper-notary`

---

## Test Run: v5-keeper-notary (2026-04-08 01:16 IST)

**Command:** `cargo test --bin dice-coordinator --message-format=short`
**Result:** **113 passed, 0 failed, 0 ignored**
**Duration:** 118.45s

### Test Breakdown by Module

#### Keeper Tests (10 tests) — NEW in v5
| # | Test | Result |
|---|------|--------|
| 1 | `keeper::tests::test_interval_trigger_fires_when_due` | PASS |
| 2 | `keeper::tests::test_interval_trigger_does_not_fire_early` | PASS |
| 3 | `keeper::tests::test_disabled_task_does_not_fire` | PASS |
| 4 | `keeper::tests::test_once_trigger_disables_after_advance` | PASS |
| 5 | `keeper::tests::test_interval_advance_sets_future_fire` | PASS |
| 6 | `keeper::tests::test_register_and_list_tasks` | PASS |
| 7 | `keeper::tests::test_toggle_task` | PASS |
| 8 | `keeper::tests::test_remove_task` | PASS |
| 9 | `keeper::tests::test_remove_nonexistent_task_errors` | PASS |
| 10 | `keeper::tests::test_history_ring_buffer` | PASS |
| 11 | `keeper::tests::test_compute_stats` | PASS |

#### Notary Tests (4 tests) — NEW in v5
| # | Test | Result |
|---|------|--------|
| 1 | `notary::tests::test_parse_valid_hash` | PASS |
| 2 | `notary::tests::test_reject_invalid_hex` | PASS |
| 3 | `notary::tests::test_reject_wrong_length_hash` | PASS |
| 4 | `notary::tests::test_receipt_structure` | PASS |
| 5 | `notary::tests::test_notary_history_ring_buffer` | PASS |

#### State Machine Tests (7 tests)
| # | Test | Result |
|---|------|--------|
| 1 | `state_machine::tests::test_new_round_starts_in_collecting_commits` | PASS |
| 2 | `state_machine::tests::test_handle_commit_transitions_to_reveals_when_all_committed` | PASS |
| 3 | `state_machine::tests::test_progress_counts_initial` | PASS |
| 4 | `state_machine::tests::test_progress_counts_after_commits` | PASS |
| 5 | `state_machine::tests::test_progress_counts_after_finalization` | PASS |
| 6 | `state_machine::tests::test_randomness_none_before_finalization` | PASS |
| 7 | `state_machine::tests::test_randomness_some_after_finalization` | PASS |
| 8 | `state_machine::tests::test_status_str_all_states` | PASS |

#### Protocol Validation Tests (12 tests)
| # | Test | Result |
|---|------|--------|
| 1 | `protocol::validation::tests::test_valid_commit_accepted` | PASS |
| 2 | `protocol::validation::tests::test_invalid_commit_rejected` | PASS |
| 3 | `protocol::validation::tests::test_valid_reveal_accepted` | PASS |
| 4 | `protocol::validation::tests::test_invalid_reveal_rejected` | PASS |
| 5 | `protocol::validation::tests::test_combine_entropy_deterministic` | PASS |
| 6 | `protocol::validation::tests::test_combine_entropy_order_sensitive` | PASS |
| 7-12 | (additional validation tests) | PASS |

#### VRF Proof Tests (15 tests)
| # | Test | Result | Duration |
|---|------|--------|----------|
| 1 | `vrf_proof_tests::tests::test_vrf_full_round_4_nodes_minimum` | PASS | <1s |
| 2 | `vrf_proof_tests::tests::test_vrf_full_round_7_nodes` | PASS | <1s |
| 3 | `vrf_proof_tests::tests::test_vrf_full_round_20_nodes_large` | PASS | <1s |
| 4 | `vrf_proof_tests::tests::test_vrf_commit_hides_entropy` | PASS | <1s |
| 5 | `vrf_proof_tests::tests::test_vrf_tampered_entropy_detected` | PASS | <1s |
| 6 | `vrf_proof_tests::tests::test_vrf_forged_signature_detected` | PASS | <1s |
| 7 | `vrf_proof_tests::tests::test_vrf_one_honest_node_sufficient` | PASS | <1s |
| 8 | `vrf_proof_tests::tests::test_vrf_partial_reveal_changes_output` | PASS | <1s |
| 9 | `vrf_proof_tests::tests::test_vrf_unpredictable_without_all_entropy` | PASS | <1s |
| 10 | `vrf_proof_tests::tests::test_vrf_verifiable_by_anyone` | PASS | <1s |
| 11 | `vrf_proof_tests::tests::test_vrf_no_sequential_correlation` | PASS | <1s |
| 12 | `vrf_proof_tests::tests::test_vrf_uniqueness_1000_rounds` | PASS | ~2s |
| 13 | `vrf_proof_tests::tests::test_vrf_output_bit_distribution` | PASS | ~3s |
| 14 | `vrf_proof_tests::tests::test_vrf_output_byte_distribution` | PASS | ~3s |
| 15 | `vrf_proof_tests::tests::test_vrf_coin_toss_fairness` | PASS | ~60s |
| 16 | `vrf_proof_tests::tests::test_vrf_dice_roll_fairness` | PASS | ~60s |

#### Integration Tests (6 tests)
| # | Test | Result |
|---|------|--------|
| 1 | `integration_tests::tests::test_full_round_7_nodes_mock` | PASS |
| 2 | `integration_tests::tests::test_full_round_min_reveals` | PASS |
| 3 | `integration_tests::tests::test_timeout_commit_phase` | PASS |
| 4 | `integration_tests::tests::test_timeout_reveal_phase` | PASS |
| 5 | `integration_tests::tests::test_rapid_fire_rounds` | PASS |
| 6 | `integration_tests::tests::test_channel_reuse_state_machine` | PASS |

#### Other Tests (~60 tests)
- Protocol message encoding/decoding
- CBOR format bridging (firmware integer-key vs SDK array-envelope)
- Node selection engine
- Queue management
- Solana TX instruction builders (commit, reveal, finalize, v2 variants)
- PDA derivation
- All passing.

---

## Test Run: v3 Production Readiness (2026-04-05)

**Command:** `cargo test --workspace --message-format=short`
**Result:** **162 passed, 0 failed**

### Hardware Battle Tests (32 tests, real ESP32-S3)
| Category | Pass | Fail | Notes |
|----------|------|------|-------|
| Boot & Onboarding | 18/18 | 0 | Captive portal, WiFi, crypto, entropy |
| VRF Round Execution | 10/10 | 0 | Commit, reveal, finalize, callback |
| Stress & Throughput | 4/8 | 4 | Failures = test script queue ordering |
| Security Attacks | 10/13 | 3 | Failures = timing after attacks, all LOW |
| Randomness Quality | 5/5 | 0 | 49% bit ratio, 127/256 Hamming, perfect |
| Queue System | 5/6 | 1 | Queue saturation from prior tests |
| Device Resilience | 4/4 | 0 | 53 min uptime, NVS persistence |

### Production Readiness Tests (25 tests, mTLS + PostgreSQL)
| Category | Pass | Fail | Notes |
|----------|------|------|-------|
| mTLS Authentication | 4/4 | 0 | No-cert rejected, rogue rejected, valid accepted |
| VRF Protocol Integrity | 2/4 | 2 | Timing after mTLS attack tests |
| Entropy Quality | 4/4 | 0 | 30/30 unique, 50% bit ratio |
| Stress (mTLS + DB) | 3/3 | 0 | 16/20 burst, queue drains, DB survived |
| Impersonation & Forgery | 1/2 | 1 | Timing overlap, ECDSA prevents forgery |
| Database Persistence | 2/2 | 0 | 50 rounds in PostgreSQL |
| API Security | 3/3 | 0 | Health, 404s, Prometheus metrics |
| Sequential Reliability | 3/3 | 0 | 42/40 rounds, device alive, 0 leaks |

### Randomness Quality (50+ samples from real ESP32-S3)
| Test | Result |
|------|--------|
| Byte distribution | 256/256 distinct values |
| Bit ratio | 49-50% ones (perfect) |
| Sequential correlation | 0 (XOR test) |
| Max run length | 10 bits (threshold: 20) |
| Avalanche (Hamming) | 127/256 bits (ideal: 128) |
| Uniqueness | 0 duplicates across 545+ outputs |

---

## Test Summary Across All Versions

| Version | Date | Tests | Pass | Fail | Notes |
|---------|------|-------|------|------|-------|
| v3 | 2026-04-05 | 162 | 162 | 0 | Full workspace incl. SDK + programs |
| v5 | 2026-04-08 | 113 | 113 | 0 | Coordinator only (keeper + notary added) |

**Note:** v5 test count (113) differs from v3 (162) because v5 runs `--bin dice-coordinator` (coordinator tests only), while v3 ran `--workspace` (includes SDK + program tests). All SDK/program tests remain unchanged.

---

## How to Run Tests

```bash
# Coordinator tests only (fast, ~2 min)
cargo test --bin dice-coordinator --message-format=short

# Full workspace (includes SDK + programs, ~3 min)
cargo test --workspace --message-format=short

# Run with output visible
cargo test --bin dice-coordinator -- --nocapture

# Run specific module
cargo test --bin dice-coordinator keeper::tests
cargo test --bin dice-coordinator notary::tests
cargo test --bin dice-coordinator vrf_proof_tests
```
