// DICE Anchor Program — Hardware-backed VRF oracle on Solana
// Protocol fee: 0.002 SOL per request
// Distribution: 70% nodes / 20% treasury / 10% reserve

use anchor_lang::prelude::*;

pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use instructions::*;

// v7.7 deployed at NEW address (not an upgrade of 78Qv6cy...) so we keep
// version-based on-chain contracts side-by-side: clients on v7.5/v7.6 keep
// hitting the original program; new integrators target v7.7 explicitly.
declare_id!("FMwPuCjkfZXN2MuNJQiUzZC3hnxHcD8mrTuntsqA84XD");

#[program]
pub mod dice {
    use super::*;

    /// Register a new ESP32-S3 hardware node by its compressed secp256k1 public key.
    pub fn register_device(ctx: Context<RegisterDevice>, device_id: [u8; 32], device_pubkey: [u8; 33]) -> Result<()> {
        instructions::register_device::handler(ctx, device_id, device_pubkey)
    }

    /// Developer requests randomness, paying 0.002 SOL into an escrow PDA.
    /// `sequence` is a caller-supplied monotonic counter that makes the PDA unique.
    /// `callback_program_id` — program to CPI-invoke with the result; use `Pubkey::default()` for poll-only.
    pub fn request_randomness(ctx: Context<RequestRandomness>, sequence: u64, callback_program_id: Pubkey) -> Result<()> {
        instructions::request_randomness::handler(ctx, sequence, callback_program_id)
    }

    /// Coordinator submits a commit (SHA-256 hash of entropy) on behalf of a node.
    pub fn submit_commit(
        ctx: Context<SubmitCommit>,
        device_id: [u8; 32],
        device_pubkey: [u8; 33],
        commit_hash: [u8; 32],
    ) -> Result<()> {
        instructions::submit_commit::handler(ctx, device_id, device_pubkey, commit_hash)
    }

    /// Coordinator submits the entropy reveal + ECDSA signature from a node.
    pub fn submit_reveal(
        ctx: Context<SubmitReveal>,
        device_id: [u8; 32],
        device_pubkey: [u8; 33],
        entropy: [u8; 32],
        signature: [u8; 64],
    ) -> Result<()> {
        instructions::submit_reveal::handler(ctx, device_id, device_pubkey, entropy, signature)
    }

    /// Finalise the round: combine entropy values via SHA-256, write RandomnessResult.
    /// Pass all RevealRecord PDAs as remaining_accounts, followed by callback program
    /// and any callback accounts if `callback_program_id` was set in the request.
    pub fn finalize_randomness<'info>(
        ctx: Context<'info, FinalizeRandomness<'info>>,
    ) -> Result<()> {
        instructions::finalize_randomness::handler(ctx)
    }

    /// Distribute escrow funds: 70% to node, 20% treasury, 10% reserve.
    /// Must be called once per contributing node (each call pays that node's share).
    pub fn claim_rewards(ctx: Context<ClaimRewards>, device_pubkey: [u8; 33]) -> Result<()> {
        instructions::claim_rewards::handler(ctx, device_pubkey)
    }

    /// Explicitly initialise an escrow PDA before funding (optional; `request_randomness`
    /// also creates one).
    pub fn init_escrow(ctx: Context<InitEscrow>, sequence: u64) -> Result<()> {
        instructions::init_escrow::handler(ctx, sequence)
    }

    // ── v2.0 Channel-based instructions ─────────────────────────────────

    /// Create a reusable DiceChannel PDA. Developer pays rent once.
    /// `max_nodes` sets the capacity (4-50). `callback_program_id` for CPI callback.
    pub fn init_channel(
        ctx: Context<InitChannel>,
        channel_index: u16,
        max_nodes: u8,
        callback_program_id: Pubkey,
        coordinator: Pubkey,
    ) -> Result<()> {
        instructions::init_channel::handler(ctx, channel_index, max_nodes, callback_program_id, coordinator)
    }

    /// Add SOL to a channel's prepaid balance for protocol fees.
    pub fn fund_channel(ctx: Context<FundChannel>, amount: u64) -> Result<()> {
        instructions::fund_channel::handler(ctx, amount)
    }

    /// Request randomness via a channel. Resets the channel, deducts fee from balance.
    /// `node_count` must be between MIN_NODES_REQUIRED and channel.max_nodes.
    pub fn request_randomness_v2(ctx: Context<RequestRandomnessV2>, node_count: u8) -> Result<()> {
        instructions::request_randomness_v2::handler(ctx, node_count)
    }

    /// Auto-fund randomness request. Channel must exist (call init_channel first).
    /// Automatically tops up channel balance from developer wallet if insufficient.
    ///
    /// v7.3+: if `remaining_accounts` is non-empty, the first entry must be
    /// the SlotHashes sysvar and the rest must be DeviceRegistry PDAs — the
    /// Fisher-Yates shuffle in `select_nodes::run_node_selection` then picks
    /// N devices on-chain and writes them into `channel.device_pubkeys`.
    /// Legacy callers that pass no remaining accounts fall back to the
    /// coordinator's off-chain SelectionEngine.
    pub fn request_randomness_auto<'info>(
        ctx: Context<'info, RequestRandomnessAuto<'info>>,
        node_count: u8,
    ) -> Result<()> {
        instructions::request_randomness_auto::handler(ctx, node_count)
    }

    /// Submit a commit hash to a channel (inline, no separate PDA).
    pub fn submit_commit_v2(
        ctx: Context<SubmitCommitV2>,
        round_id: u64,
        device_id: [u8; 32],
        device_pubkey: [u8; 33],
        commit_hash: [u8; 32],
    ) -> Result<()> {
        instructions::submit_commit_v2::handler(ctx, round_id, device_id, device_pubkey, commit_hash)
    }

    /// Submit a reveal to a channel (inline, no separate PDA).
    pub fn submit_reveal_v2(
        ctx: Context<SubmitRevealV2>,
        round_id: u64,
        device_id: [u8; 32],
        device_pubkey: [u8; 33],
        entropy: [u8; 32],
        signature: [u8; 64],
    ) -> Result<()> {
        instructions::submit_reveal_v2::handler(ctx, round_id, device_id, device_pubkey, entropy, signature)
    }

    /// Finalize randomness from inline reveals in the channel.
    pub fn finalize_v2(ctx: Context<FinalizeV2>, round_id: u64) -> Result<()> {
        instructions::finalize_v2::handler(ctx, round_id)
    }

    /// v7.5 single-shot round submission. Replaces submit_commit_v2 +
    /// submit_reveal_v2 + finalize_v2 with one atomic ix that takes all
    /// device contributions (commit_hash + entropy + signature per device).
    /// claim_rewards_v2 still runs separately in the same TX.
    pub fn submit_round_v2(
        ctx: Context<SubmitRoundV2>,
        round_id: u64,
        contributions: Vec<crate::instructions::submit_round_v2::RoundContribution>,
    ) -> Result<()> {
        instructions::submit_round_v2::handler(ctx, round_id, contributions)
    }

    /// Deliver CPI callback to the developer's program (separate from finalize).
    /// If callback fails, randomness is still finalized — developer can retry or poll.
    pub fn deliver_callback<'info>(
        ctx: Context<'info, DeliverCallback<'info>>,
        round_id: u64,
    ) -> Result<()> {
        instructions::deliver_callback::handler(ctx, round_id)
    }

    /// Withdraw prepaid balance from a channel (Idle state only).
    pub fn withdraw_balance(ctx: Context<WithdrawBalance>, amount: u64) -> Result<()> {
        instructions::withdraw_balance::handler(ctx, amount)
    }

    /// Close a channel and reclaim rent (Idle + zero balance only).
    pub fn close_channel(ctx: Context<CloseChannel>) -> Result<()> {
        instructions::close_channel::handler(ctx)
    }

    /// Mark a stuck round as failed and reset channel to Idle.
    /// Can be called by authority or coordinator when deadline has passed.
    pub fn fail_round(ctx: Context<FailRound>) -> Result<()> {
        instructions::fail_round::handler(ctx)
    }

    /// Resize a channel's max_nodes capacity (Idle state only).
    pub fn resize_channel(ctx: Context<ResizeChannel>, new_max_nodes: u8) -> Result<()> {
        instructions::resize_channel::handler(ctx, new_max_nodes)
    }

    /// On-chain verifiable node selection using SlotHashes sysvar.
    /// Deterministically selects `channel.node_count` nodes from registered DeviceRegistry
    /// accounts passed as remaining_accounts. Uses SHA-256(slot_hash || channel || round_id || block_height)
    /// as seed for Fisher-Yates shuffle — no one can predict or manipulate the selection.
    pub fn select_nodes<'info>(
        ctx: Context<'info, SelectNodes<'info>>,
        round_id: u64,
    ) -> Result<()> {
        instructions::select_nodes::handler(ctx, round_id)
    }

    // ── Streaming VRF (v7) ──────────────────────────────────────────────
    //
    // A RandomnessFeed is a persistent PDA the coordinator writes into on a cadence.
    // Subscriber dApps read it as a passive account input in their own instructions —
    // no callback, no per-request TX. Every published value must originate from a real
    // finalized DiceChannel round, so the hardware-backed audit trail is preserved.
    // These instructions are fully additive: they do not modify or affect the existing
    // v1 or v2 VRF paths.

    /// Create a streaming RandomnessFeed bound to an existing DiceChannel.
    /// The feed's authority must also own the bound channel.
    pub fn init_feed(
        ctx: Context<InitFeed>,
        feed_index: u16,
        publish_interval_slots: u32,
        name: [u8; 32],
    ) -> Result<()> {
        instructions::init_feed::handler(ctx, feed_index, publish_interval_slots, name)
    }

    /// Publish a new randomness value into a feed. Only the feed's bound coordinator
    /// can call this. Must reference the bound DiceChannel in Finalized status with
    /// a matching round_id and randomness. Enforces publish_interval_slots rate limit.
    pub fn publish_feed_value(
        ctx: Context<PublishFeedValue>,
        randomness: [u8; 32],
        round_id: u64,
    ) -> Result<()> {
        instructions::publish_feed_value::handler(ctx, randomness, round_id)
    }

    /// Close a feed and refund rent to the authority.
    pub fn close_feed(ctx: Context<CloseFeed>) -> Result<()> {
        instructions::close_feed::handler(ctx)
    }

    // ── Universal payout / NodeVault (v7) ───────────────────────────────
    //
    // Every DICE service (VRF v1, VRF v2, future PoL / sensor / HSM) credits
    // a per-device NodeVault. The operator binds their Solana wallet once
    // via hardware-signed attestation, then withdraws from a single place.
    // These instructions are fully additive; they do not modify the existing
    // VRF paths. Service instructions call into credit_vault_helper directly.

    /// Register a NodeVault for a device and bind a payout wallet.
    /// First-time binding only — rotation happens through rotate_payout_wallet.
    /// The binding message must be ECDSA-signed by the device's hardware key.
    pub fn register_node_vault(
        ctx: Context<RegisterNodeVault>,
        device_pubkey: [u8; 33],
        payout_wallet: Pubkey,
        timestamp: i64,
        nonce: [u8; 32],
        signature: [u8; 64],
    ) -> Result<()> {
        instructions::register_node_vault::handler(
            ctx,
            device_pubkey,
            payout_wallet,
            timestamp,
            nonce,
            signature,
        )
    }

    /// Rotate a NodeVault's payout wallet. Requires dual signature
    /// (device + current wallet) and enforces ROTATION_COOLDOWN_SLOTS.
    pub fn rotate_payout_wallet(
        ctx: Context<RotatePayoutWallet>,
        new_payout_wallet: Pubkey,
        timestamp: i64,
        nonce: [u8; 32],
        device_signature: [u8; 64],
    ) -> Result<()> {
        instructions::rotate_payout_wallet::handler(
            ctx,
            new_payout_wallet,
            timestamp,
            nonce,
            device_signature,
        )
    }

    /// Withdraw lamports from a NodeVault to its bound payout wallet.
    /// Only the operator's wallet can sign. Vault must be Bound.
    pub fn withdraw_from_vault(ctx: Context<WithdrawFromVault>, amount: u64) -> Result<()> {
        instructions::withdraw_from_vault::handler(ctx, amount)
    }

    /// Distribute rewards for a finalized v2 channel round into NodeVaults.
    ///
    /// Called by the channel's bound coordinator exactly once per round.
    /// Transfers 70% across contributing nodes' vaults, 20% to treasury,
    /// 10% to reserve. Transitions the channel Finalized -> Idle.
    ///
    /// Pass one writable NodeVault account per contributing node as
    /// `remaining_accounts`, in the same order as
    /// `channel.device_pubkeys[0..reveals_received]`.
    pub fn claim_rewards_v2<'info>(
        ctx: Context<'info, ClaimRewardsV2<'info>>,
    ) -> Result<()> {
        instructions::claim_rewards_v2::handler(ctx)
    }

    // ── v1.0 Legacy instructions (kept for backwards compatibility) ──────

    /// Add lamports to an existing escrow account.
    pub fn fund_escrow(ctx: Context<FundEscrow>, amount: u64) -> Result<()> {
        instructions::fund_escrow::handler(ctx, amount)
    }
}
