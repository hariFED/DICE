use anchor_lang::prelude::*;

use crate::constants::{
    MAX_PUBLISH_INTERVAL_SLOTS, MIN_PUBLISH_INTERVAL_SLOTS, SEED_CHANNEL, SEED_FEED,
};
use crate::error::DiceError;
use crate::state::{DiceChannel, FeedHistoryEntry, FeedStatus, RandomnessFeed};
use crate::constants::FEED_HISTORY_SIZE;

/// Initialize a streaming RandomnessFeed bound to an existing DiceChannel.
///
/// The feed authority must also be the channel authority — this prevents a stranger
/// from creating a feed pointing at someone else's channel. The feed's coordinator
/// is copied from the channel at init time, so the same node network that services
/// regular VRF requests also cranks the feed.
#[derive(Accounts)]
#[instruction(feed_index: u16, publish_interval_slots: u32, name: [u8; 32])]
pub struct InitFeed<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    /// The DiceChannel that every published value must originate from.
    /// Must be owned by the same authority as the feed.
    #[account(
        seeds = [SEED_CHANNEL, bound_channel.authority.as_ref(), &bound_channel.channel_index.to_le_bytes()],
        bump,
        has_one = authority,
    )]
    pub bound_channel: Account<'info, DiceChannel>,

    #[account(
        init,
        payer = authority,
        space = RandomnessFeed::space(),
        seeds = [SEED_FEED, authority.key().as_ref(), &feed_index.to_le_bytes()],
        bump,
    )]
    pub feed: Account<'info, RandomnessFeed>,

    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<InitFeed>,
    feed_index: u16,
    publish_interval_slots: u32,
    name: [u8; 32],
) -> Result<()> {
    // Validate cadence
    require!(
        publish_interval_slots >= MIN_PUBLISH_INTERVAL_SLOTS
            && publish_interval_slots <= MAX_PUBLISH_INTERVAL_SLOTS,
        DiceError::FeedInvalidInterval
    );

    let clock = Clock::get()?;
    let feed = &mut ctx.accounts.feed;
    let channel = &ctx.accounts.bound_channel;

    feed.authority = ctx.accounts.authority.key();
    feed.coordinator = channel.coordinator;
    feed.bound_channel = channel.key();
    feed.feed_index = feed_index;
    feed.name = name;
    feed.publish_interval_slots = publish_interval_slots;

    feed.status = FeedStatus::Active;
    feed.current_randomness = [0u8; 32];
    feed.current_sequence = 0;
    feed.current_slot = 0;
    feed.current_round_id = 0;
    feed.prev_randomness = [0u8; 32];
    feed.prev_sequence = 0;

    feed.history_head = 0;
    feed.history = [FeedHistoryEntry::default(); FEED_HISTORY_SIZE];

    feed.total_published = 0;
    feed.created_slot = clock.slot;

    msg!(
        "Feed initialized: authority={}, index={}, bound_channel={}, interval_slots={}",
        feed.authority,
        feed_index,
        feed.bound_channel,
        publish_interval_slots
    );
    Ok(())
}
