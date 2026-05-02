use anchor_lang::prelude::*;
use solana_program::hash::hashv;
use solana_program::secp256k1_recover::secp256k1_recover;

use crate::constants::{PAYOUT_BINDING_DOMAIN, SEED_NODE_VAULT, VAULT_SERVICE_COUNT};
use crate::error::DiceError;
use crate::state::{NodeVault, NodeVaultStatus};

/// Register a `NodeVault` by binding a payout wallet to a physical device.
///
/// This is the first-time binding flow. The vault PDA is created if missing,
/// its status is checked to be `Unbound` (no existing binding), the provided
/// ECDSA signature is verified against the device's hardware key, and the
/// vault transitions to `Bound` with the supplied `payout_wallet`.
///
/// The binding message layout — which the firmware must mirror exactly —
/// is:
///
/// ```text
/// msg = PAYOUT_BINDING_DOMAIN ("DICE_PAYOUT_BINDING_V1", 22 bytes)
///    || device_pubkey        (33 bytes, compressed secp256k1)
///    || payout_wallet        (32 bytes)
///    || timestamp_le         (8 bytes, i64 little-endian)
///    || nonce                (32 bytes, coordinator-supplied)
/// hash = keccak256(msg)
/// signature = ecdsa_sign(device_hw_key, hash)  // 64 bytes r || s
/// ```
///
/// On-chain we recover the public key via `secp256k1_recover`, compress it,
/// and reject if it does not match `device_pubkey`.
///
/// PDA seeds use `device_id(&device_pubkey)` — the 32-byte SHA-256 hash of
/// the compressed pubkey — because Solana PDA seed slices are limited to
/// 32 bytes each.
#[derive(Accounts)]
#[instruction(device_pubkey: [u8; 33])]
pub struct RegisterNodeVault<'info> {
    /// The coordinator submits this instruction. It pays rent for first-time
    /// vault creation but is NOT the authority over the vault — the
    /// `payout_wallet` embedded in the signed message is.
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        init_if_needed,
        payer = payer,
        space = NodeVault::space(),
        seeds = [SEED_NODE_VAULT, &crate::constants::device_id(&device_pubkey)],
        bump,
    )]
    pub vault: Account<'info, NodeVault>,

    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<RegisterNodeVault>,
    device_pubkey: [u8; 33],
    payout_wallet: Pubkey,
    timestamp: i64,
    nonce: [u8; 32],
    signature: [u8; 64],
) -> Result<()> {
    let clock = Clock::get()?;
    let vault = &mut ctx.accounts.vault;

    // If this is a fresh init_if_needed creation, the device_pubkey field is
    // zeroed. In that case, initialize it now. Otherwise it must already
    // match (should always be true thanks to the PDA seed, but we defensively
    // check after re-binding logic).
    if vault.device_pubkey == [0u8; 33] {
        vault.device_pubkey = device_pubkey;
        vault.created_slot = clock.slot;
        vault.total_earned = 0;
        vault.total_withdrawn = 0;
        vault.last_credit_slot = 0;
        vault.service_counts = [0u64; VAULT_SERVICE_COUNT];
        vault.reserved = [0u8; 64];
        vault.status = NodeVaultStatus::Unbound;
    }

    // The seed derivation already guarantees this, but a defensive check
    // never hurts — it catches state corruption or migration mishaps.
    require!(
        vault.device_pubkey == device_pubkey,
        DiceError::VaultBindingSignatureInvalid
    );

    // First-time binding only. Rotation happens through rotate_payout_wallet.
    require!(
        vault.status == NodeVaultStatus::Unbound,
        DiceError::VaultAlreadyBound
    );

    // Rebuild the binding message exactly as the firmware constructs it.
    verify_binding_signature(
        &device_pubkey,
        &payout_wallet,
        timestamp,
        &nonce,
        &signature,
    )?;

    // Store the binding.
    vault.payout_wallet = payout_wallet;
    vault.binding_slot = clock.slot;
    vault.binding_signature = signature;
    vault.binding_nonce = nonce;
    vault.binding_timestamp = timestamp;
    vault.status = NodeVaultStatus::Bound;

    msg!(
        "Vault bound: device={:?}, payout_wallet={}, slot={}",
        &device_pubkey[..4],
        payout_wallet,
        clock.slot
    );
    Ok(())
}

/// Reconstruct and verify the binding message's ECDSA signature.
///
/// Exposed `pub(crate)` so that `rotate_payout_wallet` can reuse the exact
/// same verification path — any change to the message layout must update
/// both instructions in lockstep.
pub(crate) fn verify_binding_signature(
    device_pubkey: &[u8; 33],
    payout_wallet: &Pubkey,
    timestamp: i64,
    nonce: &[u8; 32],
    signature: &[u8; 64],
) -> Result<()> {
    // msg = DOMAIN || device_pubkey || payout_wallet || timestamp_le || nonce
    // hash = SHA-256(msg)
    //
    // Note on hash choice: the firmware's `dice_crypto_sign` helper already
    // computes SHA-256 internally (via mbedTLS) before signing. Using SHA-256
    // here lets the firmware reuse that helper verbatim — no keccak256 crypto
    // library needs to be pulled into the ESP32 build. This is independent of
    // the (unrelated) legacy submit_reveal path which uses keccak256 for
    // "Ethereum signing convention" reasons.
    let msg_hash = hashv(&[
        PAYOUT_BINDING_DOMAIN,
        device_pubkey.as_ref(),
        payout_wallet.as_ref(),
        &timestamp.to_le_bytes(),
        nonce.as_ref(),
    ]);

    // secp256k1_recover succeeds for any valid (hash, signature) pair — the
    // recovery ID just picks which of the two possible pubkeys to return.
    // So we MUST try both recovery IDs and accept if either matches the
    // expected device pubkey. A naive .or_else() chain is buggy because
    // recovery_id=0 will return a valid-but-wrong pubkey half the time.
    let msg_hash_bytes = msg_hash.to_bytes();
    let mut matched = false;
    for rec_id in [0u8, 1u8] {
        if let Ok(recovered) = secp256k1_recover(&msg_hash_bytes, rec_id, signature) {
            // Compress the recovered 64-byte (x || y) pubkey into the 33-byte
            // form DICE stores on-chain. Prefix is 0x02 for even y, 0x03 odd.
            let raw = recovered.0;
            let y_parity = raw[63] & 1;
            let prefix = if y_parity == 0 { 0x02u8 } else { 0x03u8 };
            let mut compressed = [0u8; 33];
            compressed[0] = prefix;
            compressed[1..33].copy_from_slice(&raw[..32]);

            if compressed == *device_pubkey {
                matched = true;
                break;
            }
        }
    }

    require!(matched, DiceError::VaultBindingSignatureInvalid);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use k256::ecdsa::{signature::hazmat::PrehashSigner, Signature, SigningKey};
    use k256::elliptic_curve::sec1::ToEncodedPoint;
    use k256::SecretKey;
    use sha2::{Digest, Sha256};

    /// Generate a deterministic test keypair so the tests are reproducible.
    /// Returns (secret_key, compressed_pubkey[33]).
    fn test_keypair(seed: u8) -> (SecretKey, [u8; 33]) {
        let bytes = [seed; 32];
        let secret =
            SecretKey::from_slice(&bytes).expect("valid test secret");
        let pubkey_point = secret.public_key().to_encoded_point(true);
        let compressed: [u8; 33] = pubkey_point.as_bytes().try_into().unwrap();
        (secret, compressed)
    }

    /// Build the exact binding message the on-chain verifier rebuilds.
    /// Returns the SHA-256 hash of the concatenated message — ready to
    /// sign via PrehashSigner.
    fn build_binding_hash(
        device_pubkey: &[u8; 33],
        payout_wallet: &Pubkey,
        timestamp: i64,
        nonce: &[u8; 32],
    ) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(PAYOUT_BINDING_DOMAIN);
        hasher.update(device_pubkey);
        hasher.update(payout_wallet.as_ref());
        hasher.update(&timestamp.to_le_bytes());
        hasher.update(nonce);
        hasher.finalize().into()
    }

    fn sign_binding(
        secret: &SecretKey,
        device_pubkey: &[u8; 33],
        payout_wallet: &Pubkey,
        timestamp: i64,
        nonce: &[u8; 32],
    ) -> [u8; 64] {
        let signing_key = SigningKey::from(secret);
        let msg_hash = build_binding_hash(device_pubkey, payout_wallet, timestamp, nonce);
        let signature: Signature = signing_key
            .sign_prehash(&msg_hash)
            .expect("deterministic sign");
        let sig_bytes = signature.to_bytes();
        let mut out = [0u8; 64];
        out.copy_from_slice(&sig_bytes);
        out
    }

    #[test]
    fn verify_accepts_valid_signature() {
        let (secret, device_pubkey) = test_keypair(0x42);
        let payout_wallet = Pubkey::new_unique();
        let timestamp: i64 = 1_000_000;
        let nonce = [0xAA; 32];
        let signature = sign_binding(&secret, &device_pubkey, &payout_wallet, timestamp, &nonce);

        let r = verify_binding_signature(
            &device_pubkey,
            &payout_wallet,
            timestamp,
            &nonce,
            &signature,
        );
        assert!(r.is_ok(), "valid signature must verify");
    }

    #[test]
    fn verify_rejects_wrong_device_pubkey() {
        let (secret, device_pubkey) = test_keypair(0x11);
        let (_, other_device) = test_keypair(0x22);
        let payout_wallet = Pubkey::new_unique();
        let timestamp: i64 = 42;
        let nonce = [0x01; 32];
        let signature = sign_binding(&secret, &device_pubkey, &payout_wallet, timestamp, &nonce);

        // Sign against device A, try to verify against device B → should fail.
        let r = verify_binding_signature(
            &other_device,
            &payout_wallet,
            timestamp,
            &nonce,
            &signature,
        );
        assert!(r.is_err(), "signature must not verify against a different device");
    }

    #[test]
    fn verify_rejects_tampered_wallet() {
        let (secret, device_pubkey) = test_keypair(0x33);
        let original_wallet = Pubkey::new_unique();
        let attacker_wallet = Pubkey::new_unique();
        let timestamp: i64 = 123;
        let nonce = [0x55; 32];
        let signature =
            sign_binding(&secret, &device_pubkey, &original_wallet, timestamp, &nonce);

        // Attacker tries to swap in their own wallet.
        let r = verify_binding_signature(
            &device_pubkey,
            &attacker_wallet,
            timestamp,
            &nonce,
            &signature,
        );
        assert!(r.is_err(), "swapped wallet must be rejected");
    }

    #[test]
    fn verify_rejects_tampered_timestamp() {
        let (secret, device_pubkey) = test_keypair(0x44);
        let payout_wallet = Pubkey::new_unique();
        let nonce = [0x77; 32];
        let signature =
            sign_binding(&secret, &device_pubkey, &payout_wallet, 100, &nonce);

        let r = verify_binding_signature(&device_pubkey, &payout_wallet, 101, &nonce, &signature);
        assert!(r.is_err(), "timestamp mutation must be rejected");
    }

    #[test]
    fn verify_rejects_tampered_nonce() {
        let (secret, device_pubkey) = test_keypair(0x55);
        let payout_wallet = Pubkey::new_unique();
        let timestamp: i64 = 999;
        let signature =
            sign_binding(&secret, &device_pubkey, &payout_wallet, timestamp, &[0xAA; 32]);

        let r = verify_binding_signature(
            &device_pubkey,
            &payout_wallet,
            timestamp,
            &[0xBB; 32],
            &signature,
        );
        assert!(r.is_err(), "nonce mutation must be rejected");
    }

    #[test]
    fn verify_rejects_garbage_signature() {
        let (_, device_pubkey) = test_keypair(0x66);
        let payout_wallet = Pubkey::new_unique();
        let timestamp: i64 = 1;
        let nonce = [0u8; 32];
        let garbage = [0xDE; 64];

        let r = verify_binding_signature(
            &device_pubkey,
            &payout_wallet,
            timestamp,
            &nonce,
            &garbage,
        );
        assert!(r.is_err(), "garbage signature must be rejected");
    }

    #[test]
    fn verify_domain_separator_is_enforced() {
        // Manually build a hash WITHOUT the domain separator, sign that,
        // then verify — should fail because on-chain verify_binding_signature
        // re-computes the hash WITH the domain.
        let (secret, device_pubkey) = test_keypair(0x77);
        let payout_wallet = Pubkey::new_unique();
        let timestamp: i64 = 1;
        let nonce = [0xCC; 32];

        let mut hasher = Sha256::new();
        hasher.update(&device_pubkey);
        hasher.update(payout_wallet.as_ref());
        hasher.update(&timestamp.to_le_bytes());
        hasher.update(&nonce);
        let hash_without_domain: [u8; 32] = hasher.finalize().into();

        let signing_key = SigningKey::from(&secret);
        let signature: Signature = signing_key
            .sign_prehash(&hash_without_domain)
            .expect("sign");
        let mut sig_bytes = [0u8; 64];
        sig_bytes.copy_from_slice(&signature.to_bytes());

        let r = verify_binding_signature(
            &device_pubkey,
            &payout_wallet,
            timestamp,
            &nonce,
            &sig_bytes,
        );
        assert!(r.is_err(), "domain-less signature must be rejected");
    }

    #[test]
    fn verify_accepts_both_recovery_ids() {
        // Some keys require recovery_id=0, others require 1. The verifier
        // tries both. Loop over several seeds to hit both paths.
        for seed in 1u8..=20 {
            let (secret, device_pubkey) = test_keypair(seed);
            let payout_wallet = Pubkey::new_unique();
            let timestamp = seed as i64;
            let nonce = [seed; 32];
            let signature =
                sign_binding(&secret, &device_pubkey, &payout_wallet, timestamp, &nonce);

            let r = verify_binding_signature(
                &device_pubkey,
                &payout_wallet,
                timestamp,
                &nonce,
                &signature,
            );
            assert!(r.is_ok(), "seed {seed} must verify");
        }
    }
}
