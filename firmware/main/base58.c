#include "base58.h"

#include <string.h>

/* Bitcoin / Solana base58 alphabet. Position in this string = value. */
static const char BASE58_ALPHABET[] =
    "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

/**
 * Map a base58 character to its 0–57 numeric value, or -1 if not valid.
 * A 128-entry lookup table keyed on the ASCII code.
 */
static int8_t base58_char_value(char c)
{
    /* Build the table lazily on first call. Thread-safety is fine —
     * the table is deterministic, so racing writers produce the same result. */
    static int8_t table[128];
    static bool   built = false;

    if (!built) {
        for (int i = 0; i < 128; i++) {
            table[i] = -1;
        }
        for (int i = 0; i < 58; i++) {
            table[(int)BASE58_ALPHABET[i]] = (int8_t)i;
        }
        built = true;
    }

    if ((unsigned char)c >= 128) {
        return -1;
    }
    return table[(int)c];
}

bool dice_base58_decode(const char *input,
                        size_t input_len,
                        uint8_t *out,
                        size_t out_cap,
                        size_t *out_len)
{
    if (!input || !out || !out_len || out_cap == 0 || input_len == 0) {
        return false;
    }

    /* Working buffer: at most one byte per input char (conservative).
     * For 44-char inputs this is 44 bytes — fits comfortably on the stack. */
    uint8_t buf[64];
    if (input_len > sizeof(buf)) {
        return false;
    }
    memset(buf, 0, sizeof(buf));
    size_t buf_len = 0;

    /* Count leading '1' chars → leading zero bytes in the output. */
    size_t leading_ones = 0;
    while (leading_ones < input_len && input[leading_ones] == '1') {
        leading_ones++;
    }

    /* Classic base58 decode: for each input char, multiply the accumulator
     * by 58 and add the digit. The accumulator is stored big-endian in `buf`. */
    for (size_t i = 0; i < input_len; i++) {
        int8_t digit = base58_char_value(input[i]);
        if (digit < 0) {
            return false;  /* invalid character */
        }

        unsigned int carry = (unsigned int)digit;
        for (size_t j = 0; j < buf_len; j++) {
            carry += (unsigned int)buf[j] * 58U;
            buf[j] = (uint8_t)(carry & 0xFF);
            carry >>= 8;
        }
        while (carry > 0) {
            if (buf_len >= sizeof(buf)) {
                return false;
            }
            buf[buf_len++] = (uint8_t)(carry & 0xFF);
            carry >>= 8;
        }
    }

    /* Output = (leading_ones zeros) + reversed(buf[0..buf_len]) */
    size_t total = leading_ones + buf_len;
    if (total > out_cap) {
        return false;
    }

    /* Leading zeros first */
    size_t out_idx = 0;
    for (size_t i = 0; i < leading_ones; i++) {
        out[out_idx++] = 0x00;
    }
    /* Then the big-endian bytes (buf is little-endian, reverse it) */
    for (size_t i = 0; i < buf_len; i++) {
        out[out_idx++] = buf[buf_len - 1 - i];
    }

    *out_len = total;
    return true;
}

bool dice_is_valid_solana_wallet(const char *wallet)
{
    if (!wallet) {
        return false;
    }

    size_t len = strlen(wallet);

    /* Solana pubkey = 32 bytes base58-encoded. The encoded length is
     * always 32–44 characters for a 32-byte payload (1 byte = ~1.37 chars). */
    if (len < 32 || len > 44) {
        return false;
    }

    /* Reject any character that is not in the base58 alphabet. */
    for (size_t i = 0; i < len; i++) {
        if (base58_char_value(wallet[i]) < 0) {
            return false;
        }
    }

    /* Decode and check the byte length is exactly 32. */
    uint8_t decoded[32];
    size_t decoded_len = 0;
    if (!dice_base58_decode(wallet, len, decoded, sizeof(decoded), &decoded_len)) {
        return false;
    }
    return decoded_len == 32;
}
