use k256::{
    ecdsa::{signature::Verifier, Signature, VerifyingKey},
    EncodedPoint,
};
use sha2::{Digest, Sha256};

/// Verify an ECDSA secp256k1 signature over `commit_hash`.
///
/// `node_pubkey` — compressed secp256k1 public key (33 bytes).
/// `signature`   — DER-encoded 64-byte compact (r‖s) ECDSA signature.
///
/// Returns `true` iff the signature is valid.
pub fn verify_commit(commit_hash: &[u8; 32], node_pubkey: &[u8; 33], signature: &[u8; 64]) -> bool {
    let ep = match EncodedPoint::from_bytes(node_pubkey) {
        Ok(ep) => ep,
        Err(_) => return false,
    };

    let vk = match VerifyingKey::from_encoded_point(&ep) {
        Ok(vk) => vk,
        Err(_) => return false,
    };

    let sig = match Signature::from_bytes(signature.into()) {
        Ok(s) => s,
        Err(_) => return false,
    };

    vk.verify(commit_hash, &sig).is_ok()
}

/// Verify that `SHA-256(entropy) == commit_hash`.
///
/// This ensures the revealed entropy matches the previously submitted commit.
pub fn verify_reveal(entropy: &[u8; 32], commit_hash: &[u8; 32]) -> bool {
    let digest: [u8; 32] = Sha256::digest(entropy).into();
    digest == *commit_hash
}

/// Combine multiple entropy values into a single 32-byte randomness output.
///
/// Algorithm: `SHA-256(entropy[0] ‖ entropy[1] ‖ … ‖ entropy[n-1])`
pub fn combine_entropy(entropies: &[[u8; 32]]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    for e in entropies {
        hasher.update(e);
    }
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_reveal_roundtrip() {
        let entropy = [0xABu8; 32];
        let commit: [u8; 32] = sha2::Sha256::digest(entropy).into();
        assert!(verify_reveal(&entropy, &commit));
    }

    #[test]
    fn verify_reveal_wrong_entropy() {
        let entropy = [0xABu8; 32];
        let commit: [u8; 32] = sha2::Sha256::digest(entropy).into();
        let wrong = [0xCDu8; 32];
        assert!(!verify_reveal(&wrong, &commit));
    }

    #[test]
    fn combine_entropy_deterministic() {
        let e1 = [1u8; 32];
        let e2 = [2u8; 32];
        let r1 = combine_entropy(&[e1, e2]);
        let r2 = combine_entropy(&[e1, e2]);
        assert_eq!(r1, r2);
    }

    #[test]
    fn combine_entropy_order_matters() {
        let e1 = [1u8; 32];
        let e2 = [2u8; 32];
        assert_ne!(combine_entropy(&[e1, e2]), combine_entropy(&[e2, e1]));
    }

    // -----------------------------------------------------------------------
    // Additional tests
    // -----------------------------------------------------------------------

    use k256::ecdsa::{signature::Signer, Signature, SigningKey};

    /// Helper: generate a signing key and its 33-byte compressed pubkey.
    fn gen_keypair() -> (SigningKey, [u8; 33]) {
        let sk = SigningKey::random(&mut rand::thread_rng());
        let pk = sk.verifying_key().to_encoded_point(true);
        let mut pubkey = [0u8; 33];
        pubkey.copy_from_slice(pk.as_bytes());
        (sk, pubkey)
    }

    /// Helper: sign a commit_hash with a signing key.
    fn sign_commit(sk: &SigningKey, commit_hash: &[u8; 32]) -> [u8; 64] {
        let sig: Signature = sk.sign(commit_hash);
        let mut out = [0u8; 64];
        out.copy_from_slice(&sig.to_bytes());
        out
    }

    #[test]
    fn test_verify_commit_rejects_wrong_key() {
        let (sk_a, _pk_a) = gen_keypair();
        let (_sk_b, pk_b) = gen_keypair();

        let commit_hash = [0x42u8; 32];
        let sig = sign_commit(&sk_a, &commit_hash);

        // Signature was made by key A but we verify against key B's pubkey
        assert!(
            !verify_commit(&commit_hash, &pk_b, &sig),
            "verify_commit should reject signature made by a different key"
        );
    }

    #[test]
    fn test_verify_commit_rejects_wrong_hash() {
        let (sk, pk) = gen_keypair();

        let real_hash = [0x42u8; 32];
        let wrong_hash = [0x99u8; 32];
        let sig = sign_commit(&sk, &real_hash);

        // Signature is for real_hash, but we verify against wrong_hash
        assert!(
            !verify_commit(&wrong_hash, &pk, &sig),
            "verify_commit should reject when commit_hash does not match signed data"
        );
    }

    #[test]
    fn test_verify_reveal_rejects_wrong_commit() {
        let entropy = [0xABu8; 32];
        let correct_commit: [u8; 32] = Sha256::digest(entropy).into();

        // Tamper with one byte in the commit hash
        let mut wrong_commit = correct_commit;
        wrong_commit[0] ^= 0xFF;

        assert!(
            !verify_reveal(&entropy, &wrong_commit),
            "verify_reveal should reject when commit hash is wrong"
        );
    }

    #[test]
    fn test_combine_entropy_single_value() {
        let e = [0x77u8; 32];
        let result = combine_entropy(&[e]);

        // SHA-256 of a single 32-byte value
        let expected: [u8; 32] = Sha256::digest(e).into();
        assert_eq!(
            result, expected,
            "single-value combine should equal SHA-256 of that value"
        );
    }

    #[test]
    fn test_combine_entropy_empty_panics_or_handles() {
        // With an empty slice the function should still work (SHA-256 of empty input).
        let result = combine_entropy(&[]);
        let expected: [u8; 32] = Sha256::digest([]).into();
        assert_eq!(
            result, expected,
            "empty combine should equal SHA-256 of empty input"
        );
    }

    #[test]
    fn test_combine_entropy_large_set() {
        // Generate 15 distinct entropy values
        let entropies: Vec<[u8; 32]> = (0u8..15)
            .map(|i| {
                let mut e = [0u8; 32];
                e[0] = i;
                e[31] = 255 - i;
                e
            })
            .collect();

        let r1 = combine_entropy(&entropies);
        let r2 = combine_entropy(&entropies);
        assert_eq!(r1, r2, "combine_entropy should be deterministic for large sets");

        // Verify it matches manual SHA-256 concatenation
        let mut hasher = Sha256::new();
        for e in &entropies {
            hasher.update(e);
        }
        let expected: [u8; 32] = hasher.finalize().into();
        assert_eq!(r1, expected, "should match manual SHA-256 concatenation");

        // Removing one element should produce a different result
        let fewer = &entropies[..14];
        assert_ne!(
            combine_entropy(fewer),
            r1,
            "different set sizes should produce different results"
        );
    }

    #[test]
    fn test_verify_commit_roundtrip_multiple_keys() {
        // Generate 3 different keys and verify each independently
        let keys: Vec<(SigningKey, [u8; 33])> = (0..3).map(|_| gen_keypair()).collect();

        for (i, (sk, pk)) in keys.iter().enumerate() {
            let mut commit_hash = [0u8; 32];
            commit_hash[0] = i as u8;
            commit_hash[31] = (i * 7 + 3) as u8;

            let sig = sign_commit(sk, &commit_hash);

            assert!(
                verify_commit(&commit_hash, pk, &sig),
                "verify_commit should succeed for key {} with its own signature",
                i
            );

            // Cross-verify: signature from key i should NOT verify with key (i+1)%3
            let other_idx = (i + 1) % 3;
            let (_, other_pk) = &keys[other_idx];
            assert!(
                !verify_commit(&commit_hash, other_pk, &sig),
                "verify_commit should fail when pubkey {} is used with key {}'s signature",
                other_idx,
                i
            );
        }
    }
}
