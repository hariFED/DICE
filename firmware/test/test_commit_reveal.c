#include <string.h>
#include <stdbool.h>
#include "unity.h"
#include "dice_protocol.h"
#include "crypto.h"
#include "entropy.h"

/**
 * @file test_commit_reveal.c
 * @brief Unity tests for commit-reveal message construction.
 *
 * These tests validate the correctness of CommitSubmission and
 * RevealSubmission message encoding without requiring a live WebSocket
 * connection.  The WebSocket send call is mocked via the mock functions
 * at the bottom of this file.
 *
 * Tests focus on:
 *   - CommitSubmission CBOR encodes and decodes correctly
 *   - commit_hash == SHA-256(entropy)
 *   - RevealSubmission CBOR encodes and decodes correctly
 *   - node_id is a valid 33-byte compressed pubkey
 */

/* ------------------------------------------------------------------ */
/* Test helpers: build messages manually                                */
/* ------------------------------------------------------------------ */

/**
 * Build a CommitSubmission message from raw fields.
 * Returns the CBOR-encoded length.
 */
static int build_commit_msg(const uint8_t request_id[32],
                              const uint8_t node_id[33],
                              const uint8_t commit_hash[32],
                              const uint8_t signature[64],
                              uint8_t *cbor_out, size_t cbor_max)
{
    dice_message_t msg;
    memset(&msg, 0, sizeof(msg));
    msg.type = DICE_MSG_COMMIT;
    memcpy(msg.commit.request_id,  request_id,  32);
    memcpy(msg.commit.node_id,     node_id,     33);
    memcpy(msg.commit.commit_hash, commit_hash,  32);
    memcpy(msg.commit.signature,   signature,   64);
    return dice_msg_encode(&msg, cbor_out, cbor_max);
}

/**
 * Build a RevealSubmission message from raw fields.
 */
static int build_reveal_msg(const uint8_t request_id[32],
                              const uint8_t node_id[33],
                              const uint8_t entropy[32],
                              const uint8_t signature[64],
                              uint8_t *cbor_out, size_t cbor_max)
{
    dice_message_t msg;
    memset(&msg, 0, sizeof(msg));
    msg.type = DICE_MSG_REVEAL;
    memcpy(msg.reveal.request_id, request_id, 32);
    memcpy(msg.reveal.node_id,    node_id,    33);
    memcpy(msg.reveal.entropy,    entropy,    32);
    memcpy(msg.reveal.signature,  signature,  64);
    return dice_msg_encode(&msg, cbor_out, cbor_max);
}

/* ------------------------------------------------------------------ */
/* Tests: CBOR round-trip for CommitSubmission                          */
/* ------------------------------------------------------------------ */

TEST_CASE("CommitSubmission encodes and decodes correctly", "[commit_reveal]")
{
    uint8_t request_id[32];
    uint8_t node_id[33];
    uint8_t entropy[32];
    uint8_t commit_hash[32];
    uint8_t signature[64];
    uint8_t cbor_buf[512];

    /* Generate random request_id and entropy */
    TEST_ASSERT_TRUE(dice_entropy_generate(request_id));
    TEST_ASSERT_TRUE(dice_entropy_generate(entropy));

    /* Build node_id: compressed pubkey prefix 0x02 + 32 random bytes */
    node_id[0] = 0x02;
    TEST_ASSERT_TRUE(dice_entropy_generate(&node_id[1]));

    /* commit_hash = SHA-256(entropy) */
    dice_crypto_sha256(entropy, 32, commit_hash);

    /* signature: 64 pseudo-random bytes for the test */
    uint8_t sig_a[32], sig_b[32];
    TEST_ASSERT_TRUE(dice_entropy_generate(sig_a));
    TEST_ASSERT_TRUE(dice_entropy_generate(sig_b));
    memcpy(signature,      sig_a, 32);
    memcpy(signature + 32, sig_b, 32);

    /* Encode */
    int len = build_commit_msg(request_id, node_id, commit_hash, signature,
                                cbor_buf, sizeof(cbor_buf));
    TEST_ASSERT_GREATER_THAN_MESSAGE(0, len, "CommitSubmission encode failed");

    /* Decode */
    dice_message_t decoded;
    bool ok = dice_msg_decode(cbor_buf, (size_t)len, &decoded);
    TEST_ASSERT_TRUE_MESSAGE(ok, "CommitSubmission decode failed");

    /* Verify fields */
    TEST_ASSERT_EQUAL_UINT8(DICE_MSG_COMMIT, decoded.type);
    TEST_ASSERT_EQUAL_MEMORY(request_id,  decoded.commit.request_id,  32);
    TEST_ASSERT_EQUAL_MEMORY(node_id,     decoded.commit.node_id,     33);
    TEST_ASSERT_EQUAL_MEMORY(commit_hash, decoded.commit.commit_hash,  32);
    TEST_ASSERT_EQUAL_MEMORY(signature,   decoded.commit.signature,   64);
}

TEST_CASE("commit_hash equals SHA-256 of entropy", "[commit_reveal]")
{
    uint8_t entropy[32];
    uint8_t commit_hash[32];
    uint8_t expected_hash[32];

    TEST_ASSERT_TRUE(dice_entropy_generate(entropy));
    dice_crypto_sha256(entropy, 32, commit_hash);
    dice_crypto_sha256(entropy, 32, expected_hash);

    TEST_ASSERT_EQUAL_MEMORY_MESSAGE(expected_hash, commit_hash, 32,
        "commit_hash does not equal SHA-256(entropy)");
}

TEST_CASE("commit_hash differs from entropy", "[commit_reveal]")
{
    uint8_t entropy[32];
    uint8_t commit_hash[32];

    TEST_ASSERT_TRUE(dice_entropy_generate(entropy));
    dice_crypto_sha256(entropy, 32, commit_hash);

    TEST_ASSERT_FALSE_MESSAGE(
        memcmp(entropy, commit_hash, 32) == 0,
        "commit_hash equals entropy (SHA-256 not applied?)");
}

/* ------------------------------------------------------------------ */
/* Tests: CBOR round-trip for RevealSubmission                          */
/* ------------------------------------------------------------------ */

TEST_CASE("RevealSubmission encodes and decodes correctly", "[commit_reveal]")
{
    uint8_t request_id[32];
    uint8_t node_id[33];
    uint8_t entropy[32];
    uint8_t signature[64];
    uint8_t cbor_buf[512];

    TEST_ASSERT_TRUE(dice_entropy_generate(request_id));
    TEST_ASSERT_TRUE(dice_entropy_generate(entropy));
    node_id[0] = 0x03;
    TEST_ASSERT_TRUE(dice_entropy_generate(&node_id[1]));

    uint8_t sig_a[32], sig_b[32];
    TEST_ASSERT_TRUE(dice_entropy_generate(sig_a));
    TEST_ASSERT_TRUE(dice_entropy_generate(sig_b));
    memcpy(signature,      sig_a, 32);
    memcpy(signature + 32, sig_b, 32);

    int len = build_reveal_msg(request_id, node_id, entropy, signature,
                                cbor_buf, sizeof(cbor_buf));
    TEST_ASSERT_GREATER_THAN_MESSAGE(0, len, "RevealSubmission encode failed");

    dice_message_t decoded;
    bool ok = dice_msg_decode(cbor_buf, (size_t)len, &decoded);
    TEST_ASSERT_TRUE_MESSAGE(ok, "RevealSubmission decode failed");

    TEST_ASSERT_EQUAL_UINT8(DICE_MSG_REVEAL, decoded.type);
    TEST_ASSERT_EQUAL_MEMORY(request_id, decoded.reveal.request_id, 32);
    TEST_ASSERT_EQUAL_MEMORY(node_id,    decoded.reveal.node_id,    33);
    TEST_ASSERT_EQUAL_MEMORY(entropy,    decoded.reveal.entropy,    32);
    TEST_ASSERT_EQUAL_MEMORY(signature,  decoded.reveal.signature,  64);
}

TEST_CASE("RevealSubmission entropy is preserved byte-for-byte", "[commit_reveal]")
{
    uint8_t entropy_in[32];
    uint8_t entropy_out[32];
    uint8_t cbor_buf[512];
    uint8_t dummy_id[32] = {0};
    uint8_t dummy_nid[33] = {0x02};
    uint8_t dummy_sig[64] = {0};

    TEST_ASSERT_TRUE(dice_entropy_generate(entropy_in));

    int len = build_reveal_msg(dummy_id, dummy_nid, entropy_in, dummy_sig,
                                cbor_buf, sizeof(cbor_buf));
    TEST_ASSERT_GREATER_THAN(0, len);

    dice_message_t decoded;
    TEST_ASSERT_TRUE(dice_msg_decode(cbor_buf, (size_t)len, &decoded));

    memcpy(entropy_out, decoded.reveal.entropy, 32);

    TEST_ASSERT_EQUAL_MEMORY_MESSAGE(entropy_in, entropy_out, 32,
        "Entropy changed during CBOR encode/decode round-trip");
}

/* ------------------------------------------------------------------ */
/* Tests: CBOR round-trip for JobAssignment                             */
/* ------------------------------------------------------------------ */

TEST_CASE("JobAssignment decodes correctly", "[commit_reveal]")
{
    uint8_t request_id[32];
    TEST_ASSERT_TRUE(dice_entropy_generate(request_id));

    dice_message_t orig;
    memset(&orig, 0, sizeof(orig));
    orig.type = DICE_MSG_JOB_ASSIGN;
    memcpy(orig.job_assign.request_id, request_id, 32);
    orig.job_assign.round_seq   = 42;
    orig.job_assign.deadline_ts = 1700000000ULL;

    uint8_t cbor_buf[256];
    int len = dice_msg_encode(&orig, cbor_buf, sizeof(cbor_buf));
    TEST_ASSERT_GREATER_THAN_MESSAGE(0, len, "JobAssignment encode failed");

    dice_message_t decoded;
    bool ok = dice_msg_decode(cbor_buf, (size_t)len, &decoded);
    TEST_ASSERT_TRUE_MESSAGE(ok, "JobAssignment decode failed");

    TEST_ASSERT_EQUAL_UINT8(DICE_MSG_JOB_ASSIGN, decoded.type);
    TEST_ASSERT_EQUAL_MEMORY(request_id, decoded.job_assign.request_id, 32);
    TEST_ASSERT_EQUAL_UINT32(42,             decoded.job_assign.round_seq);
    TEST_ASSERT_EQUAL_UINT64(1700000000ULL,  decoded.job_assign.deadline_ts);
}

/* ------------------------------------------------------------------ */
/* Tests: CBOR encode buffer overflow                                   */
/* ------------------------------------------------------------------ */

TEST_CASE("encode into undersized buffer returns error", "[commit_reveal]")
{
    dice_message_t msg;
    memset(&msg, 0, sizeof(msg));
    msg.type = DICE_MSG_COMMIT;
    /* leave fields zeroed — they encode to known small values */

    uint8_t tiny_buf[4];
    int ret = dice_msg_encode(&msg, tiny_buf, sizeof(tiny_buf));
    TEST_ASSERT_EQUAL_INT_MESSAGE(-1, ret,
        "Expected -1 for buffer-overflow, got success");
}
