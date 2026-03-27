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

pub fn handler(ctx: Context<ClaimRewards>, device_pubkey: [u8; 33]) -> Result<()> {
    // Verify the request has been finalized
    require!(
        ctx.accounts.randomness_request.status == RequestStatus::Finalized,
        DiceError::RoundNotComplete
    );

    // Verify the escrow has not already been claimed
    require!(
        !ctx.accounts.escrow.is_claimed,
        DiceError::EscrowInsufficient
    );

    // Verify the device participated in this round
    let contributing_count = ctx.accounts.randomness_result.contributing_count;
    let mut node_participated = false;
    for i in 0..contributing_count as usize {
        if ctx.accounts.randomness_result.contributing_nodes[i] == device_pubkey {
            node_participated = true;
            break;
        }
    }
    require!(node_participated, DiceError::UnauthorizedNode);

    let escrow_amount = ctx.accounts.escrow.amount;
    require!(escrow_amount > 0, DiceError::EscrowInsufficient);

    let num_nodes = contributing_count as u64;
    require!(num_nodes > 0, DiceError::InsufficientNodes);

    // Calculate distribution amounts
    // NODE_REWARD_BPS = 7000 (70%) split equally across all contributing nodes
    let total_node_share = escrow_amount
        .checked_mul(NODE_REWARD_BPS)
        .unwrap()
        .checked_div(10_000)
        .unwrap();
    let per_node_share = total_node_share.checked_div(num_nodes).unwrap();

    let treasury_share = escrow_amount
        .checked_mul(TREASURY_REWARD_BPS)
        .unwrap()
        .checked_div(10_000)
        .unwrap();

    let reserve_share = escrow_amount
        .checked_mul(RESERVE_REWARD_BPS)
        .unwrap()
        .checked_div(10_000)
        .unwrap();

    // Build escrow PDA signer seeds using the bump stored in ctx.bumps
    let requester_key = ctx.accounts.randomness_request.requester;
    let sequence_le = ctx.accounts.randomness_request.sequence.to_le_bytes();
    let escrow_bump = ctx.bumps.escrow;
    let signer_seeds: &[&[&[u8]]] = &[&[
        SEED_ESCROW,
        requester_key.as_ref(),
        sequence_le.as_ref(),
        &[escrow_bump],
    ]];

    // Transfer per-node share to node_wallet
    system_program::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.system_program.to_account_info(),
            Transfer {
                from: ctx.accounts.escrow.to_account_info(),
                to: ctx.accounts.node_wallet.to_account_info(),
            },
            signer_seeds,
        ),
        per_node_share,
    )?;

    // Transfer treasury share
    system_program::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.system_program.to_account_info(),
            Transfer {
                from: ctx.accounts.escrow.to_account_info(),
                to: ctx.accounts.treasury.to_account_info(),
            },
            signer_seeds,
        ),
        treasury_share,
    )?;

    // Transfer reserve share
    system_program::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.system_program.to_account_info(),
            Transfer {
                from: ctx.accounts.escrow.to_account_info(),
                to: ctx.accounts.reserve.to_account_info(),
            },
            signer_seeds,
        ),
        reserve_share,
    )?;

    // Mark escrow as claimed
    ctx.accounts.escrow.is_claimed = true;

    msg!(
        "Rewards claimed: node_wallet={} lamports, treasury={} lamports, reserve={} lamports",
        per_node_share,
        treasury_share,
        reserve_share,
    );
    Ok(())
}
