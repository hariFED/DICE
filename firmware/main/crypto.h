#pragma once
#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

/**
 * @file crypto.h
 * @brief ECDSA secp256k1 cryptography for DICE nodes.
 *
 * The device private key is stored as a DER-encoded blob in NVS under:
 *   namespace: "dice"
 *   key:       "priv_key_der"
 *
 * The corresponding 33-byte compressed public key is the node's identity
 * (node_id in all wire messages).
 */

/**
 * Initialize the cryptographic context.
 *
 * Reads the DER-encoded private key from NVS, parses it with mbedTLS,
 * and caches the context for subsequent sign operations.
 *
 * Must be called exactly once at startup before any other crypto call.
 *
 * @return true on success, false if NVS read or key parse fails.
 */
bool dice_crypto_init(void);

/**
 * Compute SHA-256 of arbitrary data.
 *
 * @param data      Input buffer.
 * @param len       Input length in bytes.
 * @param hash_out  Output: 32-byte digest.
 */
void dice_crypto_sha256(const uint8_t *data, size_t len, uint8_t hash_out[32]);

/**
 * Sign data with the device ECDSA secp256k1 private key.
 *
 * Internally computes SHA-256(data) and signs the digest.
 * Output is the raw 64-byte signature: r (32 bytes) || s (32 bytes).
 * The signature is NOT DER-encoded.
 *
 * @param data      Input data.
 * @param data_len  Length of input in bytes.
 * @param sig_out   Output: 64-byte raw signature (r || s).
 * @return true on success.
 */
bool dice_crypto_sign(const uint8_t *data, size_t data_len, uint8_t sig_out[64]);

/**
 * Retrieve the device's 33-byte compressed secp256k1 public key.
 *
 * dice_crypto_init() must have been called successfully first.
 *
 * @param pubkey_out  Output: 33-byte compressed public key.
 * @return true on success.
 */
bool dice_crypto_get_pubkey(uint8_t pubkey_out[33]);

/**
 * Sign a payout-wallet binding message with the device's hardware key.
 *
 * The binding message layout — which MUST match the on-chain verifier in
 * `programs/dice/src/instructions/register_node_vault.rs` exactly — is:
 *
 *     msg = "DICE_PAYOUT_BINDING_V1"     (22 bytes)
 *        || device_pubkey                 (33 bytes)
 *        || payout_wallet                 (32 bytes, raw Solana pubkey bytes)
 *        || timestamp_le                  (8 bytes, i64 little-endian)
 *        || nonce                         (32 bytes, locally-generated random)
 *     hash = SHA-256(msg)
 *     signature = ecdsa_sign(device_hw_key, hash)  (64 bytes r || s)
 *
 * The device's secp256k1 public key is the one DICE already uses for VRF
 * commit-reveal — reused here so there's only one identity key on the
 * chip.
 *
 * @param payout_wallet_32  The operator-entered Solana wallet, 32 raw bytes
 *                          (base58-decoded by the caller).
 * @param timestamp         Unix epoch seconds.
 * @param nonce             32-byte random nonce generated locally.
 * @param sig_out           Output: 64-byte raw signature (r || s).
 * @return true on success.
 */
bool dice_crypto_sign_payout_binding(
    const uint8_t payout_wallet_32[32],
    int64_t       timestamp,
    const uint8_t nonce[32],
    uint8_t       sig_out[64]);
