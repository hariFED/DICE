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
}
