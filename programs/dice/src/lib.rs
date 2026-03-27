// DICE Anchor Program — Hardware-backed VRF oracle on Solana
// Protocol fee: 0.002 SOL per request
// Distribution: 70% nodes / 20% treasury / 10% reserve

use anchor_lang::prelude::*;

pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use instructions::*;

declare_id!("78Qv6cyKkRZN2YngiLSSBCe2iyRc6jgtCs3incCaMRcv");

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
        ctx: Context<'_, '_, 'info, 'info, FinalizeRandomness<'info>>,
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
    ) -> Result<()> {
        instructions::init_channel::handler(ctx, channel_index, max_nodes, callback_program_id)
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

    /// Deliver CPI callback to the developer's program (separate from finalize).
    /// If callback fails, randomness is still finalized — developer can retry or poll.
    pub fn deliver_callback<'info>(
        ctx: Context<'_, '_, 'info, 'info, DeliverCallback<'info>>,
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

    /// Resize a channel's max_nodes capacity (Idle state only).
    pub fn resize_channel(ctx: Context<ResizeChannel>, new_max_nodes: u8) -> Result<()> {
        instructions::resize_channel::handler(ctx, new_max_nodes)
    }

    // ── v1.0 Legacy instructions (kept for backwards compatibility) ──────

    /// Add lamports to an existing escrow account.
    pub fn fund_escrow(ctx: Context<FundEscrow>, amount: u64) -> Result<()> {
        instructions::fund_escrow::handler(ctx, amount)
    }
}
