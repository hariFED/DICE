use anchor_lang::prelude::*;
use anchor_lang::system_program::{self, Transfer};

use crate::constants::{
    COMMIT_TIMEOUT_SLOTS, REQUEST_FEE_LAMPORTS, SEED_ESCROW, SEED_REQUEST,
};
use crate::state::{EscrowAccount, RandomnessRequest, RequestStatus};

#[derive(Accounts)]
#[instruction(sequence: u64)]
pub struct RequestRandomness<'info> {
    #[account(mut)]
    pub requester: Signer<'info>,

    #[account(
        init,
        payer = requester,
        space = RandomnessRequest::LEN,
        seeds = [SEED_REQUEST, requester.key().as_ref(), &sequence.to_le_bytes()],
        bump,
    )]
    pub randomness_request: Account<'info, RandomnessRequest>,

    #[account(
        init,
        payer = requester,
        space = EscrowAccount::LEN,
        seeds = [SEED_ESCROW, requester.key().as_ref(), &sequence.to_le_bytes()],
        bump,
    )]
    pub escrow: Account<'info, EscrowAccount>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<RequestRandomness>, sequence: u64, callback_program_id: Pubkey) -> Result<()> {
    let clock = Clock::get()?;
    let requester_key = ctx.accounts.requester.key();

    // Transfer the protocol fee from requester to escrow PDA
    let cpi_ctx = CpiContext::new(
        ctx.accounts.system_program.to_account_info(),
        Transfer {
            from: ctx.accounts.requester.to_account_info(),
            to: ctx.accounts.escrow.to_account_info(),
        },
    );
    system_program::transfer(cpi_ctx, REQUEST_FEE_LAMPORTS)?;

    // Initialize the escrow state
    let escrow = &mut ctx.accounts.escrow;
    escrow.requester = requester_key;
    escrow.sequence = sequence;
    escrow.amount = REQUEST_FEE_LAMPORTS;
    escrow.is_claimed = false;

    // Initialize the randomness request
    let req = &mut ctx.accounts.randomness_request;
    req.requester = requester_key;
    req.sequence = sequence;
    req.status = RequestStatus::Pending;
    req.selected_nodes = [[0u8; 33]; 7];
    req.node_count = 0;
    req.commits_received = 0;
    req.reveals_received = 0;
    req.created_slot = clock.slot;
    req.commit_deadline_slot = clock.slot + COMMIT_TIMEOUT_SLOTS;
    req.reveal_deadline_slot = 0; // set when commit phase ends
    req.callback_program_id = callback_program_id;

    msg!(
        "Randomness requested by {} seq={} at slot {}",
        requester_key,
        sequence,
        clock.slot
    );
    Ok(())
}
