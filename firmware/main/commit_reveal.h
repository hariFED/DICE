#pragma once
#include <stdint.h>
#include <stdbool.h>

/**
 * @file commit_reveal.h
 * @brief Commit-reveal protocol implementation for DICE nodes.
 *
 * Flow per job:
 *   1. Coordinator sends JobAssignment  →  dice_cr_handle_job()
 *      - Generates hardware entropy
 *      - Computes commit_hash = SHA-256(entropy)
 *      - Signs commit_hash with device ECDSA key
 *      - Sends CommitSubmission to coordinator
 *      - Stores entropy for later reveal
 *
 *   2. Coordinator sends signal (or deadline triggers reveal)  →  dice_cr_do_reveal()
 *      - Signs entropy with device ECDSA key
 *      - Sends RevealSubmission with stored entropy + signature
 *      - Zeroes stored entropy buffer
 *
 * Only one job is processed at a time (static storage).
 * If a new job arrives before reveal is complete, the old job is discarded.
 */

/**
 * Handle a received JobAssignment.
 *
 * Generates entropy, forms the commit, and sends CommitSubmission.
 * The entropy is stored internally until dice_cr_do_reveal() is called.
 *
 * @param request_id   32-byte request identifier from the coordinator.
 * @param round_seq    Round sequence number.
 * @param deadline_ts  Unix timestamp by which commit must be sent.
 */
void dice_cr_handle_job(const uint8_t request_id[32],
                         uint32_t      round_seq,
                         uint64_t      deadline_ts);

/**
 * Perform the reveal phase for the given request.
 *
 * Must be called after dice_cr_handle_job() for the same request_id.
 * Sends RevealSubmission and zeroes the stored entropy.
 *
 * @param request_id  32-byte request identifier (must match the pending commit).
 * @return true if reveal was sent successfully.
 */
bool dice_cr_do_reveal(const uint8_t request_id[32]);
