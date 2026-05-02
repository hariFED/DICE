/**
 * @dice-vrf/sdk — TypeScript SDK for DICE.
 *
 * Hardware-backed VRF oracle on Solana. This SDK covers the v2 channel
 * path only — v1 is deprecated on chain (see Rust crate `dice-vrf`'s
 * README for the full rationale). Instruction builders accept plain
 * values and return `@solana/web3.js` `TransactionInstruction`s that
 * callers wrap into their own `Transaction` / `VersionedTransaction`.
 *
 * Coverage:
 *   - `DICE_PROGRAM_ID` constant
 *   - PDA derivations for channel, node vault, randomness feed
 *   - v2 instruction builders: init_channel, fund_channel, request_randomness_auto
 *   - streaming VRF: init_feed, feed PDA decoder
 *
 * Not yet covered (PRs welcome):
 *   - TypeScript versions of submit_commit_v2 / submit_reveal_v2 /
 *     finalize_v2 / claim_rewards_v2 — these are coordinator-only
 *     instructions and your dApp frontend never needs to call them.
 *   - deliver_callback — dApps that need a callback CPI usually let
 *     the coordinator submit deliver_callback for them. Build the IX
 *     client-side only if you're driving the round yourself.
 */

export * from "./constants.js";
export * from "./pda.js";
export * from "./instructions.js";
export * from "./feed.js";
