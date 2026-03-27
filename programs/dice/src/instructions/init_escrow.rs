use anchor_lang::prelude::*;

use crate::constants::SEED_ESCROW;
use crate::state::EscrowAccount;

#[derive(Accounts)]
#[instruction(sequence: u64)]
pub struct InitEscrow<'info> {
    #[account(mut)]
    pub developer: Signer<'info>,

    #[account(
        init,
        payer = developer,
        space = EscrowAccount::LEN,
        seeds = [SEED_ESCROW, developer.key().as_ref(), &sequence.to_le_bytes()],
        bump,
    )]
    pub escrow: Account<'info, EscrowAccount>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<InitEscrow>, sequence: u64) -> Result<()> {
    let escrow = &mut ctx.accounts.escrow;
    escrow.requester = ctx.accounts.developer.key();
    escrow.sequence = sequence;
    escrow.amount = 0;
    escrow.is_claimed = false;

    msg!(
        "Escrow initialized for developer={} seq={}",
        ctx.accounts.developer.key(),
        sequence
    );
    Ok(())
}
