use anchor_lang::prelude::*;

use crate::constants::SEED_CHANNEL;
use crate::error::DiceError;
use crate::state::{ChannelStatus, DiceChannel};

#[derive(Accounts)]
pub struct CloseChannel<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [SEED_CHANNEL, channel.authority.as_ref(), &channel.channel_index.to_le_bytes()],
        bump,
        has_one = authority,
        close = authority,
    )]
    pub channel: Account<'info, DiceChannel>,
}

pub fn handler(ctx: Context<CloseChannel>) -> Result<()> {
    let channel = &ctx.accounts.channel;

    // Only close when idle and balance is zero
    require!(
        channel.status == ChannelStatus::Idle,
        DiceError::RoundNotComplete
    );

    require!(
        channel.balance == 0,
        DiceError::EscrowInsufficient // reusing: "has funds, withdraw first"
    );

    msg!(
        "Channel closed: authority={}, index={}",
        channel.authority,
        channel.channel_index
    );
    Ok(())
}
