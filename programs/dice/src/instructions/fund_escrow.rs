use anchor_lang::prelude::*;
use anchor_lang::system_program::{self, Transfer};

use crate::constants::SEED_ESCROW;
use crate::state::EscrowAccount;

#[derive(Accounts)]
#[instruction(amount: u64)]
pub struct FundEscrow<'info> {
    #[account(mut)]
    pub developer: Signer<'info>,

    #[account(
        mut,
        seeds = [
            SEED_ESCROW,
            developer.key().as_ref(),
            &escrow.sequence.to_le_bytes(),
        ],
        bump,
        constraint = escrow.requester == developer.key(),
    )]
    pub escrow: Account<'info, EscrowAccount>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<FundEscrow>, amount: u64) -> Result<()> {
    let cpi_ctx = CpiContext::new(
        anchor_lang::system_program::System::id(),
        Transfer {
            from: ctx.accounts.developer.to_account_info(),
            to: ctx.accounts.escrow.to_account_info(),
        },
    );
    system_program::transfer(cpi_ctx, amount)?;

    ctx.accounts.escrow.amount = ctx.accounts.escrow.amount.checked_add(amount).unwrap();

    msg!(
        "Escrow funded: +{} lamports (total: {})",
        amount,
        ctx.accounts.escrow.amount
    );
    Ok(())
}
