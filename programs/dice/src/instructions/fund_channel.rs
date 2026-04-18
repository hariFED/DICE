use anchor_lang::prelude::*;
use anchor_lang::system_program::{self, Transfer};

use crate::constants::SEED_CHANNEL;
use crate::state::DiceChannel;

#[derive(Accounts)]
pub struct FundChannel<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [SEED_CHANNEL, channel.authority.as_ref(), &channel.channel_index.to_le_bytes()],
        bump,
        has_one = authority,
    )]
    pub channel: Account<'info, DiceChannel>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<FundChannel>, amount: u64) -> Result<()> {
    // Transfer lamports from authority to channel account
    let cpi_ctx = CpiContext::new(
        anchor_lang::system_program::System::id(),
        Transfer {
            from: ctx.accounts.authority.to_account_info(),
            to: ctx.accounts.channel.to_account_info(),
        },
    );
    system_program::transfer(cpi_ctx, amount)?;

    // Update the tracked balance
    let channel = &mut ctx.accounts.channel;
    channel.balance = channel
        .balance
        .checked_add(amount)
        .ok_or(error!(crate::error::DiceError::EscrowInsufficient))?;

    msg!(
        "Channel funded: +{} lamports, new balance={}",
        amount,
        channel.balance
    );
    Ok(())
}
