use anchor_lang::prelude::*;

use crate::constants::SEED_FEED;
use crate::state::RandomnessFeed;

/// Close a streaming RandomnessFeed and refund rent to the authority.
///
/// The authority is the only party who can close a feed. `close = authority` on the
/// feed account tells Anchor to zero the account and send its lamports back.
#[derive(Accounts)]
pub struct CloseFeed<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [SEED_FEED, feed.authority.as_ref(), &feed.feed_index.to_le_bytes()],
        bump,
        has_one = authority,
        close = authority,
    )]
    pub feed: Account<'info, RandomnessFeed>,
}

pub fn handler(ctx: Context<CloseFeed>) -> Result<()> {
    msg!(
        "Feed closed: authority={}, index={}, total_published={}",
        ctx.accounts.feed.authority,
        ctx.accounts.feed.feed_index,
        ctx.accounts.feed.total_published
    );
    Ok(())
}
