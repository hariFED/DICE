import { PublicKey } from "@solana/web3.js";

/**
 * Canonical DICE program ID on devnet and mainnet.
 *
 * Mirrors `sdk/dice-vrf/src/lib.rs::DICE_PROGRAM_ID_PUBKEY`. Keep the two
 * in lockstep — the Rust crate has a unit test that asserts they resolve
 * to the same base58 value.
 */
export const DICE_PROGRAM_ID = new PublicKey(
  "78Qv6cyKkRZN2YngiLSSBCe2iyRc6jgtCs3incCaMRcv"
);

/** Seed bytes for the `DiceChannel` PDA. */
export const SEED_CHANNEL = Buffer.from("channel");

/** Seed bytes for the per-device `NodeVault` PDA. */
export const SEED_NODE_VAULT = Buffer.from("node_vault");

/** Seed bytes for the `RandomnessFeed` PDA. */
export const SEED_FEED = Buffer.from("feed");

/**
 * Flat per-request fee collected inside `fund_channel` and distributed on
 * `claim_rewards_v2`. 2,000,000 lamports = 0.002 SOL.
 *
 * Split: 70% to contributing nodes, 20% treasury, 10% reserve.
 */
export const REQUEST_FEE_LAMPORTS = 2_000_000n;

/** Anchor discriminator = SHA-256("account:DiceChannel")[0..8]. */
export const DICE_CHANNEL_DISCRIMINATOR = Buffer.from([
  13, 92, 61, 143, 179, 94, 32, 52,
]);

/** Anchor discriminator = SHA-256("account:RandomnessFeed")[0..8]. */
export const RANDOMNESS_FEED_DISCRIMINATOR = Buffer.from([
  171, 75, 107, 39, 47, 205, 68, 158,
]);
