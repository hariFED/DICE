use anchor_lang::prelude::*;

use crate::constants::{MAX_NODES_SELECTED, SEED_COMMIT, SEED_REQUEST};
use crate::error::DiceError;
use crate::state::{CommitRecord, RandomnessRequest, RequestStatus};

#[derive(Accounts)]
#[instruction(device_id: [u8; 32], device_pubkey: [u8; 33], commit_hash: [u8; 32])]
pub struct SubmitCommit<'info> {
    /// The protocol coordinator that orchestrates the round
    #[account(mut)]
    pub coordinator: Signer<'info>,

    #[account(
        mut,
        seeds = [
            SEED_REQUEST,
            randomness_request.requester.as_ref(),
            &randomness_request.sequence.to_le_bytes(),
        ],
        bump,
    )]
    pub randomness_request: Account<'info, RandomnessRequest>,

    #[account(
        init,
        payer = coordinator,
        space = CommitRecord::LEN,
        seeds = [
            SEED_COMMIT,
            randomness_request.requester.as_ref(),
            &randomness_request.sequence.to_le_bytes(),
            &device_id,
        ],
        bump,
    )]
    pub commit_record: Account<'info, CommitRecord>,

    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<SubmitCommit>,
    device_id: [u8; 32],
    device_pubkey: [u8; 33],
    commit_hash: [u8; 32],
) -> Result<()> {
    require!(device_id == crate::constants::device_id(&device_pubkey), DiceError::InvalidDeviceId);

    let clock = Clock::get()?;
    let request_key = ctx.accounts.randomness_request.key();
    let req = &mut ctx.accounts.randomness_request;

    // Verify the round is in an acceptable state for commits
    require!(
        req.status == RequestStatus::Pending || req.status == RequestStatus::CommitPhase,
        DiceError::RoundAlreadyFinalized
    );

    // Verify the commit deadline has not passed
    require!(
        clock.slot <= req.commit_deadline_slot,
        DiceError::RoundTimedOut
    );

    // Verify we have not exceeded the maximum number of nodes
    require!(
        req.node_count < MAX_NODES_SELECTED,
        DiceError::InvalidNodeCount
    );

    // Check the device is not already in the selected_nodes list (prevent duplicate commits)
    for i in 0..req.node_count as usize {
        require!(
            req.selected_nodes[i] != device_pubkey,
            DiceError::AlreadyCommitted
        );
    }

    // Register the device as a selected node for this round.
    // Read node_count into a local first to satisfy the borrow checker.
    let slot = req.node_count as usize;
    req.selected_nodes[slot] = device_pubkey;
    req.node_count += 1;
    req.commits_received += 1;
    req.status = RequestStatus::CommitPhase;

    let seq = req.sequence;
    let commits = req.commits_received;

    // Record the commit
    let record = &mut ctx.accounts.commit_record;
    record.request = request_key;
    record.device_pubkey = device_pubkey;
    record.commit_hash = commit_hash;
    record.submitted_slot = clock.slot;

    msg!(
        "Commit received from device {:?} for request seq={} (total commits: {})",
        &device_pubkey[..4],
        seq,
        commits
    );
    Ok(())
}
