#pragma once

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

/**
 * Decode a base58-encoded string into a byte buffer.
 *
 * Used by the captive portal to validate Solana wallet addresses before
 * saving them to NVS. A Solana pubkey is a base58 encoding of exactly
 * 32 bytes (so the input string is typically 32–44 characters long).
 *
 * Returns:
 *   true  — decoded successfully. `*out_len` is set to the number of
 *           bytes written into `out`.
 *   false — malformed base58, or the decoded length exceeds `out_cap`.
 *
 * NOTE: this is NOT a general-purpose base58 implementation. It does not
 * handle leading '1' → leading zero bytes correctly for very long inputs.
 * For Solana pubkeys (always exactly 32 bytes decoded, rarely with leading
 * zeros in practice) it is sufficient. If you need to decode arbitrary
 * base58 blobs, use a dedicated library.
 */
bool dice_base58_decode(const char *input,
                        size_t input_len,
                        uint8_t *out,
                        size_t out_cap,
                        size_t *out_len);

/**
 * Validate that a string is a well-formed Solana wallet address.
 *
 * A Solana pubkey is a base58-encoded ed25519 public key — exactly 32
 * bytes when decoded. This function checks:
 *   1. Length is within the plausible base58 range for a 32-byte payload
 *      (32–44 characters)
 *   2. All characters are valid base58
 *   3. Decoded length is exactly 32 bytes
 *
 * Returns true iff the address passes all three checks.
 */
bool dice_is_valid_solana_wallet(const char *wallet);
