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

    /// Add lamports to an existing escrow account.
    pub fn fund_escrow(ctx: Context<FundEscrow>, amount: u64) -> Result<()> {
        instructions::fund_escrow::handler(ctx, amount)
    }
}
