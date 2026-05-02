#![cfg(test)]
//! VRF Proof Tests — End-to-end verification that DICE produces
//! verifiable, unpredictable, and statistically sound randomness.
//!
//! These tests simulate the COMPLETE VRF flow:
//!   1. N hardware nodes generate independent entropy
//!   2. Each node commits SHA-256(entropy) and signs it with ECDSA
//!   3. After all commits, each node reveals entropy
//!   4. Verifier checks SHA-256(entropy) == commit_hash for each reveal
//!   5. Verifier checks ECDSA signature for each commit
//!   6. Final randomness = SHA-256(entropy_1 || entropy_2 || ... || entropy_N)
//!   7. Output is tested for statistical quality

#[cfg(test)]
mod tests {
    use crate::protocol::validation::{combine_entropy, verify_commit, verify_reveal};
    use k256::ecdsa::{signature::Signer, Signature, SigningKey};
    use sha2::{Digest, Sha256};
    use std::collections::HashSet;

    /// Simulated hardware node — holds a signing key and generates entropy.
    struct SimNode {
        sk: SigningKey,
        pk: [u8; 33],
    }

    impl SimNode {
        fn new() -> Self {
            let sk = SigningKey::random(&mut rand::thread_rng());
            let pk_point = sk.verifying_key().to_encoded_point(true);
            let mut pk = [0u8; 33];
            pk.copy_from_slice(pk_point.as_bytes());
            SimNode { sk, pk }
        }

        /// Generate fresh entropy from "hardware RNG" (simulated).
        fn generate_entropy(&self) -> [u8; 32] {
            let mut entropy = [0u8; 32];
            rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut entropy);
            entropy
        }

        /// Phase 1: Commit — hash entropy and sign the hash.
        fn commit(&self, entropy: &[u8; 32]) -> ([u8; 32], [u8; 64]) {
            let commit_hash: [u8; 32] = Sha256::digest(entropy).into();
            let sig: Signature = self.sk.sign(&commit_hash);
            let mut sig_bytes = [0u8; 64];
            sig_bytes.copy_from_slice(&sig.to_bytes());
            (commit_hash, sig_bytes)
        }
    }

    /// Run a complete VRF round with N nodes.
    /// Returns (randomness, all_entropies, all_commits, all_sigs, all_pubkeys).
    fn run_full_vrf_round(
        num_nodes: usize,
    ) -> (
        [u8; 32],
        Vec<[u8; 32]>,
        Vec<[u8; 32]>,
        Vec<[u8; 64]>,
        Vec<[u8; 33]>,
    ) {
        let nodes: Vec<SimNode> = (0..num_nodes).map(|_| SimNode::new()).collect();

        // Phase 0: Each node generates entropy (hardware RNG)
        let entropies: Vec<[u8; 32]> = nodes.iter().map(|n| n.generate_entropy()).collect();

        // Phase 1: COMMIT — each node commits SHA-256(entropy) + ECDSA signature
        let mut commits = Vec::new();
        let mut sigs = Vec::new();
        for (node, entropy) in nodes.iter().zip(entropies.iter()) {
            let (commit_hash, sig) = node.commit(entropy);
            commits.push(commit_hash);
            sigs.push(sig);
        }

        // At this point, commits are public but entropy is secret.
        // No one can derive the final randomness yet.

        // Phase 2: REVEAL — each node reveals its entropy
        // (entropies are already known from phase 0, in real system they're sent over WebSocket)

        // Phase 3: VERIFY — coordinator verifies each reveal
        let pubkeys: Vec<[u8; 33]> = nodes.iter().map(|n| n.pk).collect();
        for i in 0..num_nodes {
            // Verify ECDSA signature on commit
            assert!(
                verify_commit(&commits[i], &pubkeys[i], &sigs[i]),
                "node {} commit signature verification failed",
                i
            );
            // Verify SHA-256(entropy) == commit_hash
            assert!(
                verify_reveal(&entropies[i], &commits[i]),
                "node {} reveal verification failed",
                i
            );
        }

        // Phase 4: COMBINE — final randomness = SHA-256(entropy_0 || entropy_1 || ... || entropy_N)
        let randomness = combine_entropy(&entropies);

        (randomness, entropies, commits, sigs, pubkeys)
    }

    // =========================================================================
    // VERIFIABILITY TESTS
    // =========================================================================

    #[test]
    fn test_vrf_full_round_7_nodes() {
        // Standard DICE round with 7 hardware nodes
        let (randomness, entropies, commits, sigs, pubkeys) = run_full_vrf_round(7);

        // Randomness is non-zero
        assert_ne!(randomness, [0u8; 32], "randomness must not be zero");

        // Any third party can verify the result given the public data:
        // (commits, sigs, pubkeys, entropies) → all public after reveal phase
        for i in 0..7 {
            assert!(verify_commit(&commits[i], &pubkeys[i], &sigs[i]));
            assert!(verify_reveal(&entropies[i], &commits[i]));
        }

        // Re-derive randomness from entropies — must match
        let rederived = combine_entropy(&entropies);
        assert_eq!(randomness, rederived, "anyone can verify the final randomness");
    }

    #[test]
    fn test_vrf_full_round_4_nodes_minimum() {
        // Minimum viable round with 4 nodes (MIN_NODES_REQUIRED)
        let (randomness, entropies, commits, sigs, pubkeys) = run_full_vrf_round(4);
        assert_ne!(randomness, [0u8; 32]);

        // Verify
        for i in 0..4 {
            assert!(verify_commit(&commits[i], &pubkeys[i], &sigs[i]));
            assert!(verify_reveal(&entropies[i], &commits[i]));
        }
        assert_eq!(randomness, combine_entropy(&entropies));
    }

    #[test]
    fn test_vrf_full_round_20_nodes_large() {
        // Large round with 20 nodes
        let (randomness, entropies, commits, sigs, pubkeys) = run_full_vrf_round(20);
        assert_ne!(randomness, [0u8; 32]);

        for i in 0..20 {
            assert!(verify_commit(&commits[i], &pubkeys[i], &sigs[i]));
            assert!(verify_reveal(&entropies[i], &commits[i]));
        }
        assert_eq!(randomness, combine_entropy(&entropies));
    }

    #[test]
    fn test_vrf_verifiable_by_anyone() {
        // The "V" in VRF: any third party with the public data can verify
        let (randomness, entropies, commits, sigs, pubkeys) = run_full_vrf_round(5);

        // Simulate a third-party verifier who received:
        // - the commit hashes (published on-chain during commit phase)
        // - the ECDSA signatures (published on-chain during commit phase)
        // - the device public keys (registered on-chain)
        // - the revealed entropies (published on-chain during reveal phase)
        // - the final randomness (published on-chain after finalization)

        // Step 1: Verify each node's commit signature
        for i in 0..5 {
            let sig_valid = verify_commit(&commits[i], &pubkeys[i], &sigs[i]);
            assert!(sig_valid, "third party can verify node {}'s signature", i);
        }

        // Step 2: Verify each node's reveal matches its commit
        for i in 0..5 {
            let reveal_valid = verify_reveal(&entropies[i], &commits[i]);
            assert!(reveal_valid, "third party can verify node {}'s reveal", i);
        }

        // Step 3: Re-derive the final randomness
        let verified_randomness = combine_entropy(&entropies);
        assert_eq!(
            verified_randomness, randomness,
            "third party derives the exact same randomness"
        );

        // This proves: the randomness is VERIFIABLE.
        // Anyone can check that the output was correctly derived from
        // the committed+revealed entropy of the hardware nodes.
    }

    // =========================================================================
    // UNPREDICTABILITY TESTS
    // =========================================================================

    #[test]
    fn test_vrf_unpredictable_without_all_entropy() {
        // If even ONE node's entropy is unknown, the final randomness is unpredictable.
        let nodes: Vec<SimNode> = (0..5).map(|_| SimNode::new()).collect();
        let entropies: Vec<[u8; 32]> = nodes.iter().map(|n| n.generate_entropy()).collect();
        let real_randomness = combine_entropy(&entropies);

        // Attacker knows entropies[0..4] but NOT entropies[4].
        // Try 1000 guesses for the missing entropy.
        for _ in 0..1000 {
            let mut guessed_entropies = entropies[..4].to_vec();
            let mut guess = [0u8; 32];
            rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut guess);
            guessed_entropies.push(guess);

            let guessed_randomness = combine_entropy(&guessed_entropies);

            // Should never match (probability 1/2^256)
            assert_ne!(
                guessed_randomness, real_randomness,
                "attacker should not be able to guess final randomness"
            );
        }
    }

    #[test]
    fn test_vrf_one_honest_node_sufficient() {
        // Even if N-1 nodes are colluding (controlled by attacker),
        // ONE honest node makes the output unpredictable.
        let honest_node = SimNode::new();
        let honest_entropy = honest_node.generate_entropy();

        // Run 100 trials where the attacker controls 4 nodes
        // but the honest node provides different entropy each time
        let mut results = HashSet::new();
        for _ in 0..100 {
            let mut entropies = Vec::new();
            // Attacker's 4 nodes always submit the same entropy (trying to control output)
            for _ in 0..4 {
                entropies.push([0xAA; 32]); // attacker's fixed entropy
            }
            // Honest node's entropy changes each time
            let mut honest = [0u8; 32];
            rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut honest);
            entropies.push(honest);

            let randomness = combine_entropy(&entropies);
            results.insert(randomness);
        }

        // All 100 results should be unique (honest node provides unique entropy)
        assert_eq!(
            results.len(),
            100,
            "one honest node should make every round produce unique randomness"
        );
    }

    #[test]
    fn test_vrf_commit_hides_entropy() {
        // The commit phase must not leak information about the entropy.
        // SHA-256 is a one-way function — commit_hash reveals nothing about entropy.
        let entropy = [0x42u8; 32];
        let commit: [u8; 32] = Sha256::digest(entropy).into();

        // The commit hash should look nothing like the entropy
        assert_ne!(&commit[..], &entropy[..], "commit must not equal entropy");

        // Single bit change in entropy should completely change the commit
        let mut tweaked = entropy;
        tweaked[0] ^= 1;
        let tweaked_commit: [u8; 32] = Sha256::digest(tweaked).into();
        assert_ne!(commit, tweaked_commit, "bit flip should avalanche");

        // Count differing bits (should be roughly 50% = ~128 bits)
        let differing_bits: u32 = commit
            .iter()
            .zip(tweaked_commit.iter())
            .map(|(a, b)| (a ^ b).count_ones())
            .sum();
        // SHA-256 avalanche: expect roughly 128 ± 20 differing bits
        assert!(
            differing_bits > 90 && differing_bits < 170,
            "SHA-256 avalanche effect: {} differing bits (expected ~128)",
            differing_bits
        );
    }

    // =========================================================================
    // STATISTICAL QUALITY TESTS
    // =========================================================================

    #[test]
    fn test_vrf_output_bit_distribution() {
        // Run 1000 VRF rounds and check bit distribution of output bytes.
        // Each bit position should be roughly 50/50.
        let mut bit_counts = [0u32; 256]; // 32 bytes * 8 bits

        for _ in 0..1000 {
            let (randomness, _, _, _, _) = run_full_vrf_round(5);
            for byte_idx in 0..32 {
                for bit_idx in 0..8 {
                    if (randomness[byte_idx] >> bit_idx) & 1 == 1 {
                        bit_counts[byte_idx * 8 + bit_idx] += 1;
                    }
                }
            }
        }

        // Each bit should be set roughly 500/1000 times.
        // With 1000 samples, 99% CI is roughly 500 ± 50.
        for (i, &count) in bit_counts.iter().enumerate() {
            assert!(
                count > 400 && count < 600,
                "bit {} is biased: {} ones out of 1000 (expected ~500)",
                i,
                count
            );
        }
    }

    #[test]
    fn test_vrf_output_byte_distribution() {
        // Run 500 VRF rounds, collect first byte of each output.
        // Should be approximately uniformly distributed 0-255.
        let mut byte_counts = [0u32; 256];

        for _ in 0..500 {
            let (randomness, _, _, _, _) = run_full_vrf_round(5);
            byte_counts[randomness[0] as usize] += 1;
        }

        // With 500 samples over 256 bins, expected ~1.95 per bin.
        // No bin should have more than ~15 (extremely unlikely for uniform).
        let max_count = *byte_counts.iter().max().unwrap();
        assert!(
            max_count < 20,
            "byte distribution has suspiciously concentrated bin: max={}",
            max_count
        );

        // At least 150 of 256 bins should be non-empty
        let non_empty = byte_counts.iter().filter(|&&c| c > 0).count();
        assert!(
            non_empty > 150,
            "byte distribution too sparse: only {} of 256 bins filled",
            non_empty
        );
    }

    #[test]
    fn test_vrf_uniqueness_1000_rounds() {
        // 1000 VRF rounds should produce 1000 unique outputs.
        let mut results = HashSet::new();
        for _ in 0..1000 {
            let (randomness, _, _, _, _) = run_full_vrf_round(5);
            results.insert(randomness);
        }
        assert_eq!(
            results.len(),
            1000,
            "all 1000 VRF outputs must be unique"
        );
    }

    #[test]
    fn test_vrf_no_sequential_correlation() {
        // Consecutive VRF outputs should have no correlation.
        // Measure hamming distance between consecutive outputs.
        let mut prev = run_full_vrf_round(5).0;
        let mut distances = Vec::new();

        for _ in 0..200 {
            let curr = run_full_vrf_round(5).0;
            let dist: u32 = prev
                .iter()
                .zip(curr.iter())
                .map(|(a, b)| (a ^ b).count_ones())
                .sum();
            distances.push(dist);
            prev = curr;
        }

        // Average hamming distance should be ~128 bits (half of 256)
        let avg: f64 = distances.iter().map(|&d| d as f64).sum::<f64>() / distances.len() as f64;
        assert!(
            avg > 110.0 && avg < 146.0,
            "average hamming distance between consecutive outputs: {:.1} (expected ~128)",
            avg
        );
    }

    // =========================================================================
    // COIN-TOSS FAIRNESS TEST
    // =========================================================================

    #[test]
    fn test_vrf_coin_toss_fairness() {
        // Simulate the coin-toss game with real VRF outputs.
        // Run 2000 games and check heads/tails distribution.
        let mut heads = 0u32;
        let mut tails = 0u32;

        for _ in 0..2000 {
            let (randomness, _, _, _, _) = run_full_vrf_round(5);
            // Coin-toss uses: randomness[0] & 1
            if randomness[0] & 1 == 0 {
                heads += 1;
            } else {
                tails += 1;
            }
        }

        // Expected: ~1000 each. With 2000 trials, 99% CI is roughly ± 75.
        let heads_pct = heads as f64 / 2000.0 * 100.0;
        assert!(
            heads > 900 && heads < 1100,
            "coin toss biased: {} heads ({:.1}%), {} tails — expected ~50/50",
            heads,
            heads_pct,
            tails
        );
    }

    #[test]
    fn test_vrf_dice_roll_fairness() {
        // Simulate a 6-sided dice roll using VRF output.
        // dice_roll = (randomness[0] % 6) + 1
        let mut counts = [0u32; 6]; // faces 1-6

        for _ in 0..3000 {
            let (randomness, _, _, _, _) = run_full_vrf_round(5);
            let face = (randomness[0] % 6) as usize;
            counts[face] += 1;
        }

        // Expected: ~500 per face. Allow ± 100 for statistical variance.
        for (face, &count) in counts.iter().enumerate() {
            assert!(
                count > 380 && count < 620,
                "dice face {} is biased: {} out of 3000 (expected ~500)",
                face + 1,
                count
            );
        }
    }

    // =========================================================================
    // TAMPERING RESISTANCE TESTS
    // =========================================================================

    #[test]
    fn test_vrf_tampered_entropy_detected() {
        // If a node reveals different entropy than what it committed,
        // verify_reveal catches it.
        let node = SimNode::new();
        let real_entropy = node.generate_entropy();
        let (commit_hash, _sig) = node.commit(&real_entropy);

        // Attacker tries to reveal different entropy
        let mut fake_entropy = real_entropy;
        fake_entropy[0] ^= 1; // flip one bit

        assert!(
            !verify_reveal(&fake_entropy, &commit_hash),
            "tampered entropy must be detected"
        );
    }

    #[test]
    fn test_vrf_forged_signature_detected() {
        // If someone submits a commit with a forged signature, verify_commit catches it.
        let real_node = SimNode::new();
        let attacker = SimNode::new();

        let entropy = real_node.generate_entropy();
        let (commit_hash, _real_sig) = real_node.commit(&entropy);

        // Attacker signs the same commit hash with their key
        let forged_sig: Signature = attacker.sk.sign(&commit_hash);
        let mut forged_sig_bytes = [0u8; 64];
        forged_sig_bytes.copy_from_slice(&forged_sig.to_bytes());

        // Verification against the real node's pubkey should fail
        assert!(
            !verify_commit(&commit_hash, &real_node.pk, &forged_sig_bytes),
            "forged signature must be detected"
        );
    }

    #[test]
    fn test_vrf_partial_reveal_changes_output() {
        // If a node withholds its reveal, the output changes
        // (this is why we need minimum N nodes).
        let mut entropies: Vec<[u8; 32]> = (0..5).map(|_| {
            let mut e = [0u8; 32];
            rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut e);
            e
        }).collect();

        let full_randomness = combine_entropy(&entropies);

        // Remove last node's entropy (they withheld)
        let partial = &entropies[..4];
        let partial_randomness = combine_entropy(partial);

        assert_ne!(
            full_randomness, partial_randomness,
            "withholding a reveal must change the output"
        );
    }
}
