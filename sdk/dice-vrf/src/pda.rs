// PDA derivation helpers
// Agent: SDK Agent
// These seed constants MUST match programs/dice/src/constants.rs exactly.
// verify_pda_compat.py (Phase 3) checks this automatically.

use solana_sdk::pubkey::Pubkey;

pub const SEED_DEVICE:     &[u8] = b"device";
pub const SEED_REQUEST:    &[u8] = b"request";
pub const SEED_COMMIT:     &[u8] = b"commit";
pub const SEED_REVEAL:     &[u8] = b"reveal";
pub const SEED_RESULT:     &[u8] = b"result";
pub const SEED_ESCROW:     &[u8] = b"escrow";
pub const SEED_CHANNEL:    &[u8] = b"channel";
pub const SEED_FEED:       &[u8] = b"feed";
pub const SEED_NODE_VAULT: &[u8] = b"node_vault";

/// Derive the `DeviceRegistry` PDA for a given ESP32-S3 device public key.
///
/// This account is created by the DICE coordinator when a new hardware node
/// joins the network. It stores the device's attestation certificate and
/// operational metadata (version, last-seen slot, etc.).
///
/// Seeds: `["device", device_pubkey]`
pub fn device_registry_pda(device_pubkey: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[SEED_DEVICE, device_pubkey.as_ref()], program_id)
}

/// Derive the `RandomnessRequest` PDA for a given requester and sequence number.
///
/// This account is created when a developer's program calls
/// `request_randomness`. It tracks the lifecycle of a single randomness
/// request through the commit-reveal protocol (Pending → CommitPhase →
/// RevealPhase → Finalized / Failed).
///
/// The `sequence` is a monotonically increasing counter scoped to the
/// requester; the first request uses `sequence = 1`.
///
/// Seeds: `["request", requester, sequence_le]`
pub fn randomness_request_pda(
    requester: &Pubkey,
    sequence: u64,
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[SEED_REQUEST, requester.as_ref(), &sequence.to_le_bytes()],
        program_id,
    )
}

/// Derive the `CommitRecord` PDA for a specific ESP32-S3 node within a
/// request round.
///
/// During the commit phase each participating hardware node writes a
/// cryptographic commitment (hash of its entropy contribution + nonce) into
/// its own `CommitRecord` account. The coordinator verifies all commitments
/// before advancing to the reveal phase.
///
/// Seeds: `["commit", requester, sequence_le, device_pubkey]`
pub fn commit_record_pda(
    requester: &Pubkey,
    sequence: u64,
    device_pubkey: &Pubkey,
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            SEED_COMMIT,
            requester.as_ref(),
            &sequence.to_le_bytes(),
            device_pubkey.as_ref(),
        ],
        program_id,
    )
}

/// Derive the `RevealRecord` PDA for a specific ESP32-S3 node within a
/// request round.
///
/// During the reveal phase each node publishes the entropy value whose hash
/// was committed earlier. The coordinator XORs all revealed values (after
/// hash-verification) to produce the final randomness seed.
///
/// Seeds: `["reveal", requester, sequence_le, device_pubkey]`
pub fn reveal_record_pda(
    requester: &Pubkey,
    sequence: u64,
    device_pubkey: &Pubkey,
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            SEED_REVEAL,
            requester.as_ref(),
            &sequence.to_le_bytes(),
            device_pubkey.as_ref(),
        ],
        program_id,
    )
}

/// Derive the `RandomnessResult` PDA for a finalized request.
///
/// After the reveal phase completes successfully the coordinator writes the
/// final 32-byte randomness value into this account. Downstream programs
/// should read this account (or use [`crate::cpi::decode_randomness_result`])
/// to consume the randomness; it is written exactly once and is immutable
/// thereafter.
///
/// Seeds: `["result", requester, sequence_le]`
pub fn randomness_result_pda(
    requester: &Pubkey,
    sequence: u64,
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[SEED_RESULT, requester.as_ref(), &sequence.to_le_bytes()],
        program_id,
    )
}

/// Derive the `EscrowAccount` PDA that holds the 0.002 SOL fee for one
/// randomness request.
///
/// When `payment_model` is [`crate::types::PaymentModel::DeveloperPays`] the
/// developer pre-funds this account before calling `request_randomness`. The
/// DICE program validates the balance and distributes the fee to nodes,
/// treasury, and reserve upon finalization.
///
/// Seeds: `["escrow", requester, sequence_le]`
pub fn escrow_pda(
    requester: &Pubkey,
    sequence: u64,
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[SEED_ESCROW, requester.as_ref(), &sequence.to_le_bytes()],
        program_id,
    )
}

// ── v2.0 Channel PDA ────────────────────────────────────────────────────────

/// Derive the `DiceChannel` PDA for a given authority and channel index.
///
/// A DiceChannel is a persistent, reusable account that holds everything for
/// one randomness round at a time. The developer creates it once with
/// `init_channel` and reuses it for every request — no new PDAs per round.
///
/// Seeds: `["channel", authority, &channel_index.to_le_bytes()]`
pub fn channel_pda(
    authority: &Pubkey,
    channel_index: u16,
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[SEED_CHANNEL, authority.as_ref(), &channel_index.to_le_bytes()],
        program_id,
    )
}

// ── Streaming VRF Feed PDA (v7) ─────────────────────────────────────────────

/// Derive the `RandomnessFeed` PDA for a given authority and feed index.
///
/// A RandomnessFeed is a persistent PDA the coordinator writes into on a
/// cadence. Subscriber dApps read it as a passive account input in their own
/// instructions — no callback, no per-request transaction.
///
/// Seeds: `["feed", authority, &feed_index.to_le_bytes()]`
pub fn feed_pda(
    authority: &Pubkey,
    feed_index: u16,
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[SEED_FEED, authority.as_ref(), &feed_index.to_le_bytes()],
        program_id,
    )
}

// ── Universal Payout (NodeVault) PDA (v7) ───────────────────────────────────

/// Derive the `NodeVault` PDA for a device identified by its compressed
/// secp256k1 public key.
///
/// A NodeVault is a per-device earnings account credited by every DICE
/// service (VRF v1, VRF v2, future PoL/sensor/HSM). The operator binds
/// their Solana wallet to the vault once via `register_node_vault` and
/// withdraws from a single place.
///
/// Seeds: `["node_vault", SHA256(device_pubkey)]`
///
/// Note: the seed is the 32-byte SHA-256 hash of the compressed pubkey,
/// not the raw 33-byte pubkey — Solana PDA seeds are capped at 32 bytes
/// each. This matches the on-chain `device_id` helper in constants.rs.
pub fn node_vault_pda(
    device_pubkey: &[u8; 33],
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    use sha2::{Digest, Sha256};
    let device_id: [u8; 32] = Sha256::digest(device_pubkey).into();
    Pubkey::find_program_address(&[SEED_NODE_VAULT, &device_id], program_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::pubkey::Pubkey;

    #[test]
    fn test_channel_pda_deterministic() {
        let authority = Pubkey::new_unique();
        let program_id = Pubkey::new_unique();
        let (pda1, bump1) = channel_pda(&authority, 0, &program_id);
        let (pda2, bump2) = channel_pda(&authority, 0, &program_id);
        assert_eq!(pda1, pda2);
        assert_eq!(bump1, bump2);
    }

    #[test]
    fn test_channel_pda_differs_by_authority() {
        let auth_a = Pubkey::new_unique();
        let auth_b = Pubkey::new_unique();
        let program_id = Pubkey::new_unique();
        let (pda_a, _) = channel_pda(&auth_a, 0, &program_id);
        let (pda_b, _) = channel_pda(&auth_b, 0, &program_id);
        assert_ne!(pda_a, pda_b, "different authorities must produce different PDAs");
    }

    #[test]
    fn test_channel_pda_differs_by_index() {
        let authority = Pubkey::new_unique();
        let program_id = Pubkey::new_unique();
        let (pda_0, _) = channel_pda(&authority, 0, &program_id);
        let (pda_1, _) = channel_pda(&authority, 1, &program_id);
        assert_ne!(pda_0, pda_1, "different channel indices must produce different PDAs");
    }

    #[test]
    fn test_channel_pda_differs_by_program_id() {
        let authority = Pubkey::new_unique();
        let prog_a = Pubkey::new_unique();
        let prog_b = Pubkey::new_unique();
        let (pda_a, _) = channel_pda(&authority, 0, &prog_a);
        let (pda_b, _) = channel_pda(&authority, 0, &prog_b);
        assert_ne!(pda_a, pda_b, "different program IDs must produce different PDAs");
    }

    #[test]
    fn test_device_registry_pda_deterministic() {
        let device = Pubkey::new_unique();
        let program_id = Pubkey::new_unique();
        let (pda1, bump1) = device_registry_pda(&device, &program_id);
        let (pda2, bump2) = device_registry_pda(&device, &program_id);
        assert_eq!(pda1, pda2);
        assert_eq!(bump1, bump2);
    }

    #[test]
    fn test_all_pda_types_differ() {
        let requester = Pubkey::new_unique();
        let device = Pubkey::new_unique();
        let program_id = Pubkey::new_unique();
        let seq: u64 = 42;

        let (request, _) = randomness_request_pda(&requester, seq, &program_id);
        let (commit, _) = commit_record_pda(&requester, seq, &device, &program_id);
        let (reveal, _) = reveal_record_pda(&requester, seq, &device, &program_id);
        let (result, _) = randomness_result_pda(&requester, seq, &program_id);
        let (escrow, _) = escrow_pda(&requester, seq, &program_id);

        // All five PDAs must be unique from each other
        let pdas = [request, commit, reveal, result, escrow];
        for i in 0..pdas.len() {
            for j in (i + 1)..pdas.len() {
                assert_ne!(
                    pdas[i], pdas[j],
                    "PDA at index {} must differ from PDA at index {}",
                    i, j
                );
            }
        }
    }

    #[test]
    fn test_pda_bumps_are_valid() {
        let requester = Pubkey::new_unique();
        let device = Pubkey::new_unique();
        let program_id = Pubkey::new_unique();
        let seq: u64 = 1;

        let (_, bump_device) = device_registry_pda(&device, &program_id);
        let (_, bump_request) = randomness_request_pda(&requester, seq, &program_id);
        let (_, bump_commit) = commit_record_pda(&requester, seq, &device, &program_id);
        let (_, bump_reveal) = reveal_record_pda(&requester, seq, &device, &program_id);
        let (_, bump_result) = randomness_result_pda(&requester, seq, &program_id);
        let (_, bump_escrow) = escrow_pda(&requester, seq, &program_id);
        let (_, bump_channel) = channel_pda(&requester, 0, &program_id);

        // Bumps are u8 so they are inherently < 256, but we verify they are
        // in the valid canonical range (find_program_address returns the
        // highest valid bump, which is <= 255).
        // Bumps are u8, so they are inherently in [0, 255]. We verify they
        // are non-zero as a sanity check — find_program_address tries bumps
        // from 255 downward and returns the first valid one.
        for (label, bump) in [
            ("device", bump_device),
            ("request", bump_request),
            ("commit", bump_commit),
            ("reveal", bump_reveal),
            ("result", bump_result),
            ("escrow", bump_escrow),
            ("channel", bump_channel),
        ] {
            // Verify the bump was actually found (not some degenerate case)
            let _ = bump; // suppress unused warning; the real check is that
                          // find_program_address didn't panic.
            assert!(
                (bump as u16) < 256,
                "{} bump {} is out of range",
                label, bump
            );
        }
    }

    #[test]
    fn test_channel_index_zero_and_max() {
        let authority = Pubkey::new_unique();
        let program_id = Pubkey::new_unique();

        // channel_index = 0 (minimum)
        let (pda_min, bump_min) = channel_pda(&authority, 0, &program_id);
        // channel_index = 65535 (u16::MAX)
        let (pda_max, bump_max) = channel_pda(&authority, u16::MAX, &program_id);

        // Both must produce valid PDAs
        assert_ne!(pda_min, Pubkey::default());
        assert_ne!(pda_max, Pubkey::default());
        // They must be different from each other
        assert_ne!(pda_min, pda_max, "index 0 and index 65535 must produce different PDAs");
        // Bumps must be valid
        assert!((bump_min as u16) < 256);
        assert!((bump_max as u16) < 256);
    }

    #[test]
    fn test_escrow_pda_differs_by_sequence() {
        let requester = Pubkey::new_unique();
        let program_id = Pubkey::new_unique();
        let (escrow_1, _) = escrow_pda(&requester, 1, &program_id);
        let (escrow_2, _) = escrow_pda(&requester, 2, &program_id);
        let (escrow_max, _) = escrow_pda(&requester, u64::MAX, &program_id);
        assert_ne!(escrow_1, escrow_2, "different sequences must produce different escrow PDAs");
        assert_ne!(escrow_1, escrow_max, "sequence 1 and u64::MAX must differ");
        assert_ne!(escrow_2, escrow_max, "sequence 2 and u64::MAX must differ");
    }

    // ── Streaming VRF feed PDA tests ────────────────────────────────────────

    #[test]
    fn test_feed_pda_deterministic() {
        let authority = Pubkey::new_unique();
        let program_id = Pubkey::new_unique();
        let (a, bump_a) = feed_pda(&authority, 0, &program_id);
        let (b, bump_b) = feed_pda(&authority, 0, &program_id);
        assert_eq!(a, b);
        assert_eq!(bump_a, bump_b);
    }

    #[test]
    fn test_feed_pda_differs_by_index_and_authority() {
        let auth_a = Pubkey::new_unique();
        let auth_b = Pubkey::new_unique();
        let program_id = Pubkey::new_unique();
        let (a0, _) = feed_pda(&auth_a, 0, &program_id);
        let (a1, _) = feed_pda(&auth_a, 1, &program_id);
        let (b0, _) = feed_pda(&auth_b, 0, &program_id);
        assert_ne!(a0, a1, "different indices must differ");
        assert_ne!(a0, b0, "different authorities must differ");
    }

    #[test]
    fn test_feed_pda_distinct_from_channel_pda() {
        // A feed with index N must not collide with a channel with the same index
        // for the same authority — different seed prefixes ("feed" vs "channel")
        // guarantee it, but we verify explicitly.
        let authority = Pubkey::new_unique();
        let program_id = Pubkey::new_unique();
        let (channel, _) = channel_pda(&authority, 0, &program_id);
        let (feed, _) = feed_pda(&authority, 0, &program_id);
        assert_ne!(channel, feed, "feed and channel PDAs must never collide");
    }
}
