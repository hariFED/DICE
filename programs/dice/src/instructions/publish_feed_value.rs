use anchor_lang::prelude::*;

use crate::constants::{SEED_CHANNEL, SEED_FEED};
use crate::error::DiceError;
use crate::state::{ChannelStatus, DiceChannel, FeedStatus, RandomnessFeed};

/// Publish a new value into a streaming RandomnessFeed.
///
/// This is called by the feed's bound coordinator on a cadence. It does NOT produce
/// randomness — the randomness must already exist as a finalized round on the feed's
/// bound DiceChannel. This instruction copies that value into the feed's current
/// slot and advances the ring buffer. Subscribers who read the feed immediately
/// observe the new value.
///
/// Guards:
///   1. Caller must be the feed's bound coordinator.
///   2. Feed must be Active (not Closed).
///   3. Current slot must satisfy `feed.current_slot + publish_interval_slots <= clock.slot`
///      (except on the very first publish when current_slot == 0).
///   4. Bound channel must match the feed's configured bound_channel.
///   5. Channel must be in Finalized status with a matching round_id and randomness.
#[derive(Accounts)]
pub struct PublishFeedValue<'info> {
    /// The coordinator bound to the feed.
    pub coordinator: Signer<'info>,

    #[account(
        mut,
        seeds = [SEED_FEED, feed.authority.as_ref(), &feed.feed_index.to_le_bytes()],
        bump,
    )]
    pub feed: Account<'info, RandomnessFeed>,

    /// The DiceChannel whose finalized randomness is being streamed into the feed.
    /// Passed as readonly — we only read `randomness`, `round_id`, and `status`.
    #[account(
        seeds = [SEED_CHANNEL, bound_channel.authority.as_ref(), &bound_channel.channel_index.to_le_bytes()],
        bump,
    )]
    pub bound_channel: Account<'info, DiceChannel>,
}

pub fn handler(
    ctx: Context<PublishFeedValue>,
    randomness: [u8; 32],
    round_id: u64,
) -> Result<()> {
    let clock = Clock::get()?;
    let feed = &mut ctx.accounts.feed;
    let channel = &ctx.accounts.bound_channel;

    // Guard 1: coordinator must match
    require!(
        ctx.accounts.coordinator.key() == feed.coordinator,
        DiceError::FeedWrongCoordinator
    );

    // Guard 2: feed must be active
    require!(feed.status == FeedStatus::Active, DiceError::FeedNotActive);

    // Guard 3: rate limit — enforce publish_interval_slots between publishes.
    // Skip on genesis publish (current_slot == 0 means "never published yet").
    if feed.current_slot > 0 {
        let next_allowed = feed
            .current_slot
            .saturating_add(feed.publish_interval_slots as u64);
        require!(clock.slot >= next_allowed, DiceError::FeedPublishTooSoon);
    }

    // Guard 4: bound channel must match
    require!(
        channel.key() == feed.bound_channel,
        DiceError::FeedChannelMismatch
    );

    // Guard 5: channel must be finalized, round_id must match, randomness must match
    require!(
        channel.status == ChannelStatus::Finalized,
        DiceError::FeedChannelNotFinalized
    );
    require!(channel.round_id == round_id, DiceError::FeedRoundIdMismatch);
    require!(
        channel.randomness == randomness,
        DiceError::FeedRandomnessMismatch
    );

    // All guards passed — record the publish.
    feed.record_publish(randomness, round_id, clock.slot);

    msg!(
        "Feed publish: seq={}, round_id={}, slot={}, rand_prefix={:?}",
        feed.current_sequence,
        round_id,
        clock.slot,
        &randomness[..8]
    );
    Ok(())
}
