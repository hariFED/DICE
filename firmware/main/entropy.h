#pragma once
#include <stdint.h>
#include <stdbool.h>

/**
 * @file entropy.h
 * @brief Hardware entropy generation for DICE nodes.
 *
 * Mixes three independent sources and finalises with SHA-256:
 *   1. esp_fill_random() — hardware TRNG (primary source, SAR ADC + thermal noise)
 *   2. ADC1 noise       — 8 raw samples from a floating pin (additional analogue entropy)
 *   3. FreeRTOS tick    — timing jitter from the kernel scheduler
 */

/**
 * Generate 32 bytes of hardware entropy.
 *
 * All three sources are XOR-mixed into a 32-byte buffer then SHA-256
 * hashed so the output is uniformly distributed regardless of any
 * single-source bias.
 *
 * @param entropy_out  Output buffer, exactly 32 bytes.
 * @return true on success, false if SHA-256 computation fails.
 */
bool dice_entropy_generate(uint8_t entropy_out[32]);

/**
 * Self-test: generate 10 samples and verify no two are identical.
 *
 * Should be called once at boot before accepting job assignments.
 * Returns false (and logs an error) if the TRNG appears stuck.
 *
 * @return true if all samples are distinct, false otherwise.
 */
bool dice_entropy_selftest(void);
