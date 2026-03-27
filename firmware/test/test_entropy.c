#include <string.h>
#include "unity.h"
#include "entropy.h"

/**
 * @file test_entropy.c
 * @brief Unity tests for the DICE entropy module.
 */

TEST_CASE("entropy generates 32 non-zero bytes", "[entropy]")
{
    uint8_t buf[32];
    memset(buf, 0, sizeof(buf));

    bool ok = dice_entropy_generate(buf);
    TEST_ASSERT_TRUE_MESSAGE(ok, "dice_entropy_generate returned false");

    /* Verify output is not all zeros */
    bool all_zero = true;
    for (int i = 0; i < 32; i++) {
        if (buf[i] != 0) {
            all_zero = false;
            break;
        }
    }
    TEST_ASSERT_FALSE_MESSAGE(all_zero, "entropy output is all zeros");

    /* Verify output is not all 0xFF */
    bool all_ff = true;
    for (int i = 0; i < 32; i++) {
        if (buf[i] != 0xFF) {
            all_ff = false;
            break;
        }
    }
    TEST_ASSERT_FALSE_MESSAGE(all_ff, "entropy output is all 0xFF");
}

TEST_CASE("two successive entropy samples differ", "[entropy]")
{
    uint8_t a[32];
    uint8_t b[32];

    TEST_ASSERT_TRUE(dice_entropy_generate(a));
    TEST_ASSERT_TRUE(dice_entropy_generate(b));

    /* Samples must not be identical */
    TEST_ASSERT_FALSE_MESSAGE(
        memcmp(a, b, 32) == 0,
        "Two consecutive entropy samples are identical");
}

TEST_CASE("entropy output length is exactly 32 bytes", "[entropy]")
{
    /* SHA-256 output is always 32 bytes.
     * We write into a guarded buffer and check guards are intact. */
    uint8_t guarded[34];
    guarded[0]  = 0xAB;  /* guard byte before */
    guarded[33] = 0xCD;  /* guard byte after  */

    bool ok = dice_entropy_generate(&guarded[1]);
    TEST_ASSERT_TRUE(ok);

    TEST_ASSERT_EQUAL_HEX8_MESSAGE(0xAB, guarded[0],
        "Guard byte before entropy buffer was overwritten");
    TEST_ASSERT_EQUAL_HEX8_MESSAGE(0xCD, guarded[33],
        "Guard byte after entropy buffer was overwritten");
}

TEST_CASE("entropy self-test passes", "[entropy]")
{
    TEST_ASSERT_TRUE_MESSAGE(
        dice_entropy_selftest(),
        "dice_entropy_selftest() returned false");
}

TEST_CASE("ten entropy samples are all distinct", "[entropy]")
{
#define N_SAMPLES 10
    uint8_t samples[N_SAMPLES][32];

    for (int i = 0; i < N_SAMPLES; i++) {
        TEST_ASSERT_TRUE(dice_entropy_generate(samples[i]));
    }

    for (int i = 0; i < N_SAMPLES; i++) {
        for (int j = i + 1; j < N_SAMPLES; j++) {
            TEST_ASSERT_FALSE_MESSAGE(
                memcmp(samples[i], samples[j], 32) == 0,
                "Two of the ten entropy samples are identical");
        }
    }
#undef N_SAMPLES
}
