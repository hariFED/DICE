#include <string.h>
#include "unity.h"
#include "crypto.h"
#include "entropy.h"

/**
 * @file test_crypto.c
 * @brief Unity tests for the DICE crypto module.
 *
 * Note: dice_crypto_init() depends on NVS containing a valid secp256k1 key.
 * In the test environment this must be provisioned before running these tests.
 */

/* ------------------------------------------------------------------ */
/* SHA-256 tests (no NVS dependency)                                    */
/* ------------------------------------------------------------------ */

TEST_CASE("sha256 known value — abc", "[crypto]")
{
    /* SHA-256("abc") = ba7816bf8f01cfea414140de5dae2ec73b00361bbef0469049e182eb4d090099 */
    static const uint8_t expected[32] = {
        0xba, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea,
        0x41, 0x41, 0x40, 0xde, 0x5d, 0xae, 0x2e, 0xc7,
        0x3b, 0x00, 0x36, 0x1b, 0xbe, 0xf0, 0x46, 0x90,
        0x49, 0xe1, 0x82, 0xeb, 0x4d, 0x09, 0x00, 0x99
    };

    uint8_t result[32];
    dice_crypto_sha256((const uint8_t *)"abc", 3, result);

    TEST_ASSERT_EQUAL_MEMORY_MESSAGE(expected, result, 32,
        "SHA-256('abc') mismatch");
}

TEST_CASE("sha256 empty input", "[crypto]")
{
    /* SHA-256("") = e3b0c44298fc1c149afb... */
    static const uint8_t expected[32] = {
        0xe3, 0xb0, 0xc4, 0x42, 0x98, 0xfc, 0x1c, 0x14,
        0x9a, 0xfb, 0xf4, 0xc8, 0x99, 0x6f, 0xb9, 0x24,
        0x27, 0xae, 0x41, 0xe4, 0x64, 0x9b, 0x93, 0x4c,
        0xa4, 0x95, 0x99, 0x1b, 0x78, 0x52, 0xb8, 0x55
    };

    uint8_t result[32];
    static const uint8_t empty[1] = {0};
    dice_crypto_sha256(empty, 0, result);

    TEST_ASSERT_EQUAL_MEMORY_MESSAGE(expected, result, 32,
        "SHA-256('') mismatch");
}

TEST_CASE("sha256 deterministic — same input gives same output", "[crypto]")
{
    static const uint8_t input[] = "DICE determinism test";
    uint8_t result1[32];
    uint8_t result2[32];

    dice_crypto_sha256(input, sizeof(input) - 1, result1);
    dice_crypto_sha256(input, sizeof(input) - 1, result2);

    TEST_ASSERT_EQUAL_MEMORY_MESSAGE(result1, result2, 32,
        "SHA-256 is not deterministic");
}

TEST_CASE("sha256 different inputs give different outputs", "[crypto]")
{
    uint8_t hash_a[32];
    uint8_t hash_b[32];

    dice_crypto_sha256((const uint8_t *)"input_A", 7, hash_a);
    dice_crypto_sha256((const uint8_t *)"input_B", 7, hash_b);

    TEST_ASSERT_FALSE_MESSAGE(
        memcmp(hash_a, hash_b, 32) == 0,
        "Different inputs produced the same SHA-256 hash");
}

TEST_CASE("commit hash equals sha256 of entropy", "[crypto]")
{
    uint8_t entropy[32];
    uint8_t direct_hash[32];
    uint8_t commit_hash[32];

    /* Generate fresh entropy */
    TEST_ASSERT_TRUE(dice_entropy_generate(entropy));

    /* Hash it two ways: they must agree */
    dice_crypto_sha256(entropy, 32, direct_hash);
    dice_crypto_sha256(entropy, 32, commit_hash);

    TEST_ASSERT_EQUAL_MEMORY_MESSAGE(direct_hash, commit_hash, 32,
        "Repeated SHA-256 of same entropy gives different results");
}

/* ------------------------------------------------------------------ */
/* ECDSA signing tests (require NVS private key)                        */
/* ------------------------------------------------------------------ */

TEST_CASE("crypto_init succeeds", "[crypto][requires_nvs]")
{
    bool ok = dice_crypto_init();
    TEST_ASSERT_TRUE_MESSAGE(ok, "dice_crypto_init() failed");
}

TEST_CASE("get_pubkey returns 33-byte compressed key", "[crypto][requires_nvs]")
{
    TEST_ASSERT_TRUE(dice_crypto_init());

    uint8_t pubkey[33];
    memset(pubkey, 0, sizeof(pubkey));

    bool ok = dice_crypto_get_pubkey(pubkey);
    TEST_ASSERT_TRUE_MESSAGE(ok, "dice_crypto_get_pubkey() failed");

    /* Compressed secp256k1 pubkey starts with 0x02 or 0x03 */
    TEST_ASSERT_TRUE_MESSAGE(
        pubkey[0] == 0x02 || pubkey[0] == 0x03,
        "Compressed pubkey prefix is not 0x02 or 0x03");

    /* Must not be all zeros */
    bool all_zero = true;
    for (int i = 1; i < 33; i++) {
        if (pubkey[i] != 0) { all_zero = false; break; }
    }
    TEST_ASSERT_FALSE_MESSAGE(all_zero, "pubkey bytes are all zero");
}

TEST_CASE("sign produces 64-byte non-zero signature", "[crypto][requires_nvs]")
{
    TEST_ASSERT_TRUE(dice_crypto_init());

    static const uint8_t data[] = "test message for signing";
    uint8_t sig[64];
    memset(sig, 0, sizeof(sig));

    bool ok = dice_crypto_sign(data, sizeof(data) - 1, sig);
    TEST_ASSERT_TRUE_MESSAGE(ok, "dice_crypto_sign() failed");

    /* Signature must not be all zeros */
    bool all_zero = true;
    for (int i = 0; i < 64; i++) {
        if (sig[i] != 0) { all_zero = false; break; }
    }
    TEST_ASSERT_FALSE_MESSAGE(all_zero, "Signature is all zeros");
}

TEST_CASE("two signatures of same data differ (ECDSA uses random k)", "[crypto][requires_nvs]")
{
    TEST_ASSERT_TRUE(dice_crypto_init());

    static const uint8_t data[] = "determinism test";
    uint8_t sig1[64];
    uint8_t sig2[64];

    TEST_ASSERT_TRUE(dice_crypto_sign(data, sizeof(data) - 1, sig1));
    TEST_ASSERT_TRUE(dice_crypto_sign(data, sizeof(data) - 1, sig2));

    /* ECDSA with random k: signatures should be different each time */
    TEST_ASSERT_FALSE_MESSAGE(
        memcmp(sig1, sig2, 64) == 0,
        "Two signatures of the same data are identical (non-random k?)");
}
