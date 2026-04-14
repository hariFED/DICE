#pragma once
#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

/**
 * @file dice_protocol.h
 * @brief Minimal CBOR encoder/decoder for DICE wire messages.
 *
 * CBOR major types used:
 *   0 = unsigned integer
 *   2 = byte string
 *   3 = text string
 *   5 = map
 *
 * All maps use small integer keys (0, 1, 2, ...) to keep encoding compact.
 *
 * Message type discriminator is always map key 0.
 */

/* ------------------------------------------------------------------ */
/* Message type constants                                               */
/* ------------------------------------------------------------------ */

#define DICE_MSG_HEARTBEAT       0   /**< Node → Coordinator */
#define DICE_MSG_JOB_ASSIGN      1   /**< Coordinator → Node */
#define DICE_MSG_COMMIT          2   /**< Node → Coordinator */
#define DICE_MSG_REVEAL          3   /**< Node → Coordinator */
#define DICE_MSG_ROUND_RESULT    4   /**< Coordinator → Node */
#define DICE_MSG_PAYOUT_BINDING  5   /**< Node → Coordinator (v7) */

/* ------------------------------------------------------------------ */
/* Unified message structure                                            */
/* ------------------------------------------------------------------ */

typedef struct {
    uint8_t type;   /**< One of DICE_MSG_* constants */
    union {
        /** DICE_MSG_HEARTBEAT (type=0): node → coordinator */
        struct {
            uint8_t  node_id[33];       /**< Compressed secp256k1 pubkey */
            uint32_t latency_ms;         /**< Last round-trip latency in ms */
            uint64_t uptime_secs;        /**< Seconds since boot */
            uint64_t jobs_completed;     /**< Total jobs completed */
            uint64_t timestamp;          /**< Unix timestamp (seconds) */
        } heartbeat;

        /** DICE_MSG_JOB_ASSIGN (type=1): coordinator → node */
        struct {
            uint8_t  request_id[32];    /**< 32-byte request identifier */
            uint32_t round_seq;          /**< Sequence number within the round */
            uint64_t deadline_ts;        /**< Unix timestamp deadline for commit */
        } job_assign;

        /** DICE_MSG_COMMIT (type=2): node → coordinator */
        struct {
            uint8_t request_id[32];     /**< 32-byte request identifier */
            uint8_t node_id[33];         /**< Compressed pubkey (node identity) */
            uint8_t commit_hash[32];     /**< SHA-256(entropy) */
            uint8_t signature[64];       /**< ECDSA sig over commit_hash (r||s) */
        } commit;

        /** DICE_MSG_REVEAL (type=3): node → coordinator */
        struct {
            uint8_t request_id[32];     /**< 32-byte request identifier */
            uint8_t node_id[33];         /**< Compressed pubkey (node identity) */
            uint8_t entropy[32];         /**< The raw entropy revealed */
            uint8_t signature[64];       /**< ECDSA sig over entropy (r||s) */
        } reveal;

        /** DICE_MSG_ROUND_RESULT (type=4): coordinator → node */
        struct {
            uint8_t request_id[32];     /**< 32-byte request identifier */
            char    status[16];          /**< "success" | "timeout" | "failed" */
            uint8_t randomness[32];      /**< Final aggregated randomness */
        } round_result;

        /** DICE_MSG_PAYOUT_BINDING (type=5): node → coordinator
         *
         * Sent once by the firmware after the operator enters a Solana
         * wallet in the captive portal. The device signs
         * `PAYOUT_BINDING_DOMAIN || node_id || payout_wallet || timestamp_le || nonce`
         * with its hardware ECDSA key (SHA-256 internally). The coordinator
         * submits `register_node_vault` on-chain, where Anchor verifies the
         * signature via secp256k1_recover.
         */
        struct {
            uint8_t node_id[33];         /**< Compressed secp256k1 pubkey */
            uint8_t payout_wallet[32];   /**< Raw Solana pubkey bytes */
            int64_t timestamp;           /**< Unix epoch seconds */
            uint8_t nonce[32];           /**< Local random nonce */
            uint8_t signature[64];       /**< ECDSA over SHA-256 of msg */
        } payout_binding;
    };
} dice_message_t;

/* ------------------------------------------------------------------ */
/* Encode / decode                                                      */
/* ------------------------------------------------------------------ */

/**
 * Encode a dice_message_t into a CBOR byte buffer.
 *
 * @param msg      Message to encode.
 * @param buf      Output buffer.
 * @param buf_len  Size of output buffer in bytes.
 * @return Number of bytes written, or -1 on error (buffer too small, etc.).
 */
int dice_msg_encode(const dice_message_t *msg, uint8_t *buf, size_t buf_len);

/**
 * Decode a CBOR byte buffer into a dice_message_t.
 *
 * Only recognises the 5 DICE message types.  Unknown types return false.
 *
 * @param buf      Input CBOR buffer.
 * @param buf_len  Length of input buffer.
 * @param msg_out  Destination message struct (zeroed by this function on entry).
 * @return true on success, false on parse error.
 */
bool dice_msg_decode(const uint8_t *buf, size_t buf_len, dice_message_t *msg_out);
