// PDA derivation helpers
// Agent: SDK Agent
// These seed constants MUST match programs/dice/src/constants.rs exactly.
// verify_pda_compat.py (Phase 3) checks this automatically.

use solana_sdk::pubkey::Pubkey;

pub const SEED_DEVICE:  &[u8] = b"device";
pub const SEED_REQUEST: &[u8] = b"request";
pub const SEED_COMMIT:  &[u8] = b"commit";
pub const SEED_REVEAL:  &[u8] = b"reveal";
pub const SEED_RESULT:  &[u8] = b"result";
pub const SEED_ESCROW:  &[u8] = b"escrow";
pub const SEED_CHANNEL: &[u8] = b"channel";

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
