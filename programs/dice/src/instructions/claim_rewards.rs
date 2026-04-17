use anchor_lang::prelude::*;
use anchor_lang::system_program::{self, Transfer};

use crate::constants::{
    NODE_REWARD_BPS, RESERVE_REWARD_BPS, SEED_ESCROW, SEED_REQUEST, SEED_RESULT,
    TREASURY_REWARD_BPS,
};
use crate::error::DiceError;
use crate::state::{EscrowAccount, RandomnessRequest, RandomnessResult, RequestStatus};

#[derive(Accounts)]
#[instruction(device_pubkey: [u8; 33])]
pub struct ClaimRewards<'info> {
    pub coordinator: Signer<'info>,

    #[account(
        seeds = [
            SEED_REQUEST,
            randomness_request.requester.as_ref(),
            &randomness_request.sequence.to_le_bytes(),
        ],
        bump,
    )]
    pub randomness_request: Account<'info, RandomnessRequest>,

    #[account(
        seeds = [
            SEED_RESULT,
            randomness_request.requester.as_ref(),
            &randomness_request.sequence.to_le_bytes(),
        ],
        bump,
    )]
    pub randomness_result: Account<'info, RandomnessResult>,

    #[account(
        mut,
        seeds = [
            SEED_ESCROW,
            randomness_request.requester.as_ref(),
            &randomness_request.sequence.to_le_bytes(),
        ],
        bump,
    )]
    pub escrow: Account<'info, EscrowAccount>,

    /// Wallet that receives the node's share of the rewards
    #[account(mut)]
    pub node_wallet: SystemAccount<'info>,

    /// Protocol treasury account (20%)
    #[account(mut)]
    pub treasury: SystemAccount<'info>,

    /// Protocol reserve account (10%)
    #[account(mut)]
    pub reserve: SystemAccount<'info>,

    pub system_program: Program<'info, System>,
}

/// v1 claim_rewards is **deprecated** — returns `V1ClaimRewardsDeprecated`.
///
/// Background: the original design takes a single `device_pubkey` and pays
/// one node + full treasury + full reserve per call, setting
/// `escrow.is_claimed = true` at the end. Two bugs interact:
///
///   1. The `is_claimed` flag is escrow-scoped, not per-node. After the
///      first call sets it, every subsequent node's claim fails the
///      `!is_claimed` check — so only one of N contributing nodes ever
///      gets paid.
///
///   2. If `is_claimed` didn't exist, treasury + reserve would be paid
///      on EVERY node's claim (N times), draining the escrow.
///
/// The two bugs mask each other: `is_claimed` prevents the N-times drain
/// by also preventing N-1 of the nodes from collecting. There is no
/// minimal fix that preserves the single-node-per-call API without schema
/// changes to `EscrowAccount` (per-node claim tracking or per-node PDAs).
///
/// v2 `claim_rewards_v2` solves this cleanly: the coordinator passes all
/// contributing NodeVault PDAs in `remaining_accounts` and the instruction
/// atomically distributes the full split in one call. That's the
/// supported path — every deployed channel uses it.
///
/// Returning an error here prevents silent under-/over-payment on any
/// remaining caller; it does not touch existing escrow state.
pub fn handler(_ctx: Context<ClaimRewards>, _device_pubkey: [u8; 33]) -> Result<()> {
    msg!(
        "v1 claim_rewards is deprecated — use claim_rewards_v2 (v2 channel path)"
    );
    err!(DiceError::V1ClaimRewardsDeprecated)
}
