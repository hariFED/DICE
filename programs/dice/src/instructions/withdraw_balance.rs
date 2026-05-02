use anchor_lang::prelude::*;

use crate::constants::SEED_CHANNEL;
use crate::error::DiceError;
use crate::state::{ChannelStatus, DiceChannel};

#[derive(Accounts)]
pub struct WithdrawBalance<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [SEED_CHANNEL, channel.authority.as_ref(), &channel.channel_index.to_le_bytes()],
        bump,
        has_one = authority,
    )]
    pub channel: Account<'info, DiceChannel>,
}

pub fn handler(ctx: Context<WithdrawBalance>, amount: u64) -> Result<()> {
    // Validate and update balance first
    {
        let channel = &mut ctx.accounts.channel;
        require!(
            channel.status == ChannelStatus::Idle,
            DiceError::RoundNotComplete
        );
        require!(channel.balance >= amount, DiceError::EscrowInsufficient);
        channel.balance = channel
            .balance
            .checked_sub(amount)
            .ok_or(error!(DiceError::EscrowInsufficient))?;
    }

    // Transfer lamports (separate borrow scope)
    let channel_info = ctx.accounts.channel.to_account_info();
    let authority_info = ctx.accounts.authority.to_account_info();
    **channel_info.try_borrow_mut_lamports()? -= amount;
    **authority_info.try_borrow_mut_lamports()? += amount;

    msg!("Withdrawn {} lamports", amount);
    Ok(())
}
