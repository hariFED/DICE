use solana_program::hash::hashv;

/// Compute a 32-byte PDA-safe device identifier from a 33-byte compressed secp256k1 pubkey.
/// `device_id = SHA-256(device_pubkey)`
pub fn device_id(device_pubkey: &[u8; 33]) -> [u8; 32] {
    hashv(&[device_pubkey]).to_bytes()
}

// These seed values MUST match sdk/dice-vrf/src/pda.rs exactly
pub const SEED_DEVICE:     &[u8] = b"device";
pub const SEED_REQUEST:    &[u8] = b"request";
pub const SEED_COMMIT:     &[u8] = b"commit";
pub const SEED_REVEAL:     &[u8] = b"reveal";
pub const SEED_RESULT:     &[u8] = b"result";
pub const SEED_ESCROW:     &[u8] = b"escrow";
pub const SEED_CHANNEL:    &[u8] = b"channel";
pub const SEED_FEED:       &[u8] = b"feed";
pub const SEED_NODE_VAULT: &[u8] = b"node_vault";

pub const REQUEST_FEE_LAMPORTS: u64 = 2_000_000; // 0.002 SOL
pub const NODE_REWARD_BPS:      u64 = 7_000;     // 70%
pub const TREASURY_REWARD_BPS:  u64 = 2_000;     // 20%
pub const RESERVE_REWARD_BPS:   u64 = 1_000;     // 10%
pub const MIN_NODES_REQUIRED:   u8  = 4;
pub const MAX_NODES_SELECTED:   u8  = 50;
pub const COMMIT_TIMEOUT_SLOTS: u64 = 150; // ~60 seconds
pub const REVEAL_TIMEOUT_SLOTS: u64 = 150;

// ── Streaming VRF feed constants ─────────────────────────────────────
/// Minimum slot cadence between feed publishes (1 slot ≈ 400ms).
pub const MIN_PUBLISH_INTERVAL_SLOTS: u32 = 1;
/// Maximum slot cadence between feed publishes (~24 hours at 400ms/slot).
pub const MAX_PUBLISH_INTERVAL_SLOTS: u32 = 216_000;
/// Number of historical values retained in a feed's ring buffer.
pub const FEED_HISTORY_SIZE: usize = 16;

// ── Universal payout (NodeVault) constants ───────────────────────────
/// Domain separator prefix embedded in every payout-binding signature.
/// Any change here MUST be reflected in the firmware binding code.
pub const PAYOUT_BINDING_DOMAIN: &[u8] = b"DICE_PAYOUT_BINDING_V1";
/// Minimum number of slots between payout-wallet rotations (~24 hours at 400ms/slot).
pub const ROTATION_COOLDOWN_SLOTS: u64 = 216_000;
/// Number of services tracked per vault (VRF_v1, VRF_v2, PoL, Sensor, HSM, +3 reserved).
pub const VAULT_SERVICE_COUNT: usize = 8;
/// Logical service identifiers used by `credit_vault`. Must match firmware expectations.
pub const SERVICE_ID_VRF_V1:   u8 = 0;
pub const SERVICE_ID_VRF_V2:   u8 = 1;
pub const SERVICE_ID_POL:      u8 = 2;
pub const SERVICE_ID_SENSOR:   u8 = 3;
pub const SERVICE_ID_HSM:      u8 = 4;

#[cfg(test)]
mod tests {
    use super::*;

    // ── device_id() tests ────────────────────────────────────────────────

    #[test]
    fn test_device_id_deterministic() {
        let pubkey: [u8; 33] = [
            0x02, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00,
            0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88,
            0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00,
            0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88,
            0x99,
        ];
        let id1 = device_id(&pubkey);
        let id2 = device_id(&pubkey);
        assert_eq!(id1, id2, "same input must produce the same device_id");
    }

    #[test]
    fn test_device_id_differs_for_different_keys() {
        let key_a: [u8; 33] = [0x02; 33];
        let key_b: [u8; 33] = [0x03; 33];
        let id_a = device_id(&key_a);
        let id_b = device_id(&key_b);
        assert_ne!(id_a, id_b, "different keys must produce different device_ids");
    }

    #[test]
    fn test_device_id_differs_single_bit_change() {
        let mut key_a: [u8; 33] = [0x02; 33];
        let mut key_b = key_a;
        key_b[32] ^= 0x01; // flip one bit in the last byte
        let id_a = device_id(&key_a);
        let id_b = device_id(&key_b);
        assert_ne!(id_a, id_b, "a single-bit difference must produce a different device_id");
    }

    #[test]
    fn test_device_id_is_32_bytes() {
        let pubkey: [u8; 33] = [0x04; 33];
        let id = device_id(&pubkey);
        assert_eq!(id.len(), 32, "device_id must always be 32 bytes");
    }

    #[test]
    fn test_device_id_all_zeros() {
        let pubkey: [u8; 33] = [0x00; 33];
        let id = device_id(&pubkey);
        // Should still produce a valid hash, not all zeros
        assert_eq!(id.len(), 32);
        // SHA-256 of 33 zero bytes is not all zeros
        assert_ne!(id, [0u8; 32], "SHA-256 of zeros is not zeros");
    }

    // ── Fee constants tests ──────────────────────────────────────────────

    #[test]
    fn test_fee_constants_reasonable() {
        assert!(REQUEST_FEE_LAMPORTS > 0, "request fee must be positive");
        assert_eq!(REQUEST_FEE_LAMPORTS, 2_000_000, "request fee should be 0.002 SOL");

        // BPS splits must sum to exactly 10000 (100%)
        let total_bps = NODE_REWARD_BPS + TREASURY_REWARD_BPS + RESERVE_REWARD_BPS;
        assert_eq!(total_bps, 10_000, "reward BPS must sum to 10000");
    }

    #[test]
    fn test_individual_bps_values() {
        assert_eq!(NODE_REWARD_BPS, 7_000, "nodes get 70%");
        assert_eq!(TREASURY_REWARD_BPS, 2_000, "treasury gets 20%");
        assert_eq!(RESERVE_REWARD_BPS, 1_000, "reserve gets 10%");
    }

    #[test]
    fn test_fee_distribution_arithmetic() {
        // Verify that the BPS splits actually divide the fee cleanly
        let fee = REQUEST_FEE_LAMPORTS;
        let node_share = fee * NODE_REWARD_BPS / 10_000;
        let treasury_share = fee * TREASURY_REWARD_BPS / 10_000;
        let reserve_share = fee * RESERVE_REWARD_BPS / 10_000;
        assert_eq!(
            node_share + treasury_share + reserve_share,
            fee,
            "shares must sum to exact fee (no rounding dust)"
        );
    }

    // ── Timeout constants tests ──────────────────────────────────────────

    #[test]
    fn test_timeout_slots_nonzero() {
        assert!(COMMIT_TIMEOUT_SLOTS > 0, "commit timeout must be positive");
        assert!(REVEAL_TIMEOUT_SLOTS > 0, "reveal timeout must be positive");
    }

    #[test]
    fn test_timeout_slots_reasonable_range() {
        // Timeouts should give nodes enough time (~60 seconds at ~0.4s/slot)
        // but not be absurdly long (e.g., > 1000 slots = ~7 minutes)
        assert!(COMMIT_TIMEOUT_SLOTS >= 50, "commit timeout too short");
        assert!(COMMIT_TIMEOUT_SLOTS <= 1000, "commit timeout too long");
        assert!(REVEAL_TIMEOUT_SLOTS >= 50, "reveal timeout too short");
        assert!(REVEAL_TIMEOUT_SLOTS <= 1000, "reveal timeout too long");
    }

    // ── Node bounds tests ────────────────────────────────────────────────

    #[test]
    fn test_node_bounds() {
        assert!(MIN_NODES_REQUIRED > 0, "minimum nodes must be positive");
        assert!(MAX_NODES_SELECTED > 0, "maximum nodes must be positive");
        assert!(
            MIN_NODES_REQUIRED < MAX_NODES_SELECTED,
            "min must be strictly less than max"
        );
    }

    #[test]
    fn test_node_bounds_specific_values() {
        assert_eq!(MIN_NODES_REQUIRED, 4, "minimum is 4 nodes for BFT threshold");
        assert_eq!(MAX_NODES_SELECTED, 50, "maximum is 50 nodes per channel capacity");
    }

    // ── Seed constants tests ─────────────────────────────────────────────

    #[test]
    fn test_seed_constants_non_empty() {
        assert!(!SEED_DEVICE.is_empty(), "SEED_DEVICE must not be empty");
        assert!(!SEED_REQUEST.is_empty(), "SEED_REQUEST must not be empty");
        assert!(!SEED_COMMIT.is_empty(), "SEED_COMMIT must not be empty");
        assert!(!SEED_REVEAL.is_empty(), "SEED_REVEAL must not be empty");
        assert!(!SEED_RESULT.is_empty(), "SEED_RESULT must not be empty");
        assert!(!SEED_ESCROW.is_empty(), "SEED_ESCROW must not be empty");
        assert!(!SEED_CHANNEL.is_empty(), "SEED_CHANNEL must not be empty");
    }

    #[test]
    fn test_seed_constants_are_valid_ascii() {
        let seeds: &[&[u8]] = &[
            SEED_DEVICE, SEED_REQUEST, SEED_COMMIT, SEED_REVEAL,
            SEED_RESULT, SEED_ESCROW, SEED_CHANNEL,
        ];
        for seed in seeds {
            assert!(
                std::str::from_utf8(seed).is_ok(),
                "all seeds must be valid UTF-8"
            );
            for &b in *seed {
                assert!(b.is_ascii_lowercase(), "seed bytes should be lowercase ASCII: found 0x{:02x}", b);
            }
        }
    }

    #[test]
    fn test_seed_constants_are_distinct() {
        let seeds: Vec<&[u8]> = vec![
            SEED_DEVICE, SEED_REQUEST, SEED_COMMIT, SEED_REVEAL,
            SEED_RESULT, SEED_ESCROW, SEED_CHANNEL,
        ];
        // All pairs must be distinct to avoid PDA collisions
        for i in 0..seeds.len() {
            for j in (i + 1)..seeds.len() {
                assert_ne!(
                    seeds[i], seeds[j],
                    "seeds at index {} and {} must be distinct",
                    i, j
                );
            }
        }
    }

    #[test]
    fn test_seed_values_match_expected() {
        assert_eq!(SEED_DEVICE, b"device");
        assert_eq!(SEED_REQUEST, b"request");
        assert_eq!(SEED_COMMIT, b"commit");
        assert_eq!(SEED_REVEAL, b"reveal");
        assert_eq!(SEED_RESULT, b"result");
        assert_eq!(SEED_ESCROW, b"escrow");
        assert_eq!(SEED_CHANNEL, b"channel");
    }
}
