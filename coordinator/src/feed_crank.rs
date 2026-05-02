//! Streaming-VRF feed crank loop.
//!
//! Polls all Active `RandomnessFeed` accounts on a cadence. For each feed,
//! checks whether its bound DiceChannel has a newer finalized round than
//! what the feed currently holds, and — if the rate-limit window has
//! elapsed — submits a `publish_feed_value` TX that copies the channel's
//! current randomness into the feed.
//!
//! This runs as a background task alongside the round dispatcher: it
//! doesn't create new rounds, it just pushes randomness that other drivers
//! already caused to exist. Drivers pair this with periodic
//! `request_randomness_auto` calls on the bound channel — whenever the
//! channel finalizes, the feed crank picks up the new value and publishes
//! it to every feed bound to that channel.

use std::{sync::Arc, time::Duration};

use anyhow::{Context, Result};
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};
use tracing::{debug, info, warn};

use crate::{solana_rpc::SolanaRpc, solana_tx};

/// Anchor account discriminator for `RandomnessFeed`.
/// SHA-256("account:RandomnessFeed")[0..8]
const FEED_DISCRIMINATOR: [u8; 8] = [171, 75, 107, 39, 47, 205, 68, 158];

/// Anchor account discriminator for `DiceChannel`.
/// SHA-256("account:DiceChannel")[0..8]
const DICE_CHANNEL_DISC: [u8; 8] = [13, 92, 61, 143, 179, 94, 32, 52];

/// Byte at offset 75 of DiceChannel = status; variant 4 is Finalized.
const DICE_CHANNEL_STATUS_OFFSET: usize = 75;
const DICE_CHANNEL_STATUS_FINALIZED: u8 = 4;

/// RandomnessFeed layout offsets (must match programs/dice/src/state/randomness_feed.rs).
/// Every field here is fixed-size (no Vec) so offsets are compile-time constants.
const FEED_AUTHORITY_OFFSET: usize = 8;
const FEED_COORDINATOR_OFFSET: usize = 40;
const FEED_BOUND_CHANNEL_OFFSET: usize = 72;
const FEED_INDEX_OFFSET: usize = 104;
const FEED_PUBLISH_INTERVAL_OFFSET: usize = 138;
const FEED_STATUS_OFFSET: usize = 142;
const FEED_CURRENT_RANDOMNESS_OFFSET: usize = 143;
const FEED_CURRENT_SEQUENCE_OFFSET: usize = 175;
const FEED_CURRENT_SLOT_OFFSET: usize = 183;
const FEED_CURRENT_ROUND_ID_OFFSET: usize = 191;

const FEED_STATUS_ACTIVE: u8 = 0;

/// DiceChannel layout offsets (same ones the poller uses).
const CHANNEL_AUTHORITY_OFFSET: usize = 8;
const CHANNEL_INDEX_OFFSET: usize = 72;
const CHANNEL_ROUND_ID_OFFSET: usize = 76;
/// Offset of the 32-byte `randomness` field inside a DiceChannel.
/// Layout: disc(8) + authority(32) + coordinator(32) + channel_index(2)
///   + max_nodes(1) + status(1) + round_id(8) + node_count(1)
///   + commits_received(1) + reveals_received(1) + created_slot(8)
///   + commit_deadline_slot(8) + reveal_deadline_slot(8) + balance(8)
///   + callback_program_id(32) = 151; then randomness(32).
const CHANNEL_RANDOMNESS_OFFSET: usize = 151;

/// Minimum poll interval. Solana slots are ~400ms; polling faster than
/// 2 seconds is pointless because finality takes longer than that anyway.
const POLL_INTERVAL: Duration = Duration::from_secs(3);

/// One feed extracted from on-chain state.
#[derive(Debug, Clone)]
struct FeedRow {
    pubkey: Pubkey,
    authority: Pubkey,
    coordinator: Pubkey,
    bound_channel: Pubkey,
    feed_index: u16,
    publish_interval_slots: u32,
    current_slot: u64,
    current_round_id: u64,
    current_sequence: u64,
}

/// Run the feed-crank loop until the task is cancelled.
pub async fn run_feed_crank(
    rpc: Arc<SolanaRpc>,
    program_id: Pubkey,
    coordinator_keypair: Arc<Keypair>,
) {
    info!(
        program = %program_id,
        coordinator = %coordinator_keypair.pubkey(),
        "feed crank starting — polling Active RandomnessFeeds every 3s"
    );

    let mut interval = tokio::time::interval(POLL_INTERVAL);

    loop {
        interval.tick().await;

        let feeds = match scan_active_feeds(&rpc, &program_id).await {
            Ok(f) => f,
            Err(e) => {
                debug!(error = %e, "feed scan failed (retrying)");
                continue;
            }
        };

        if feeds.is_empty() {
            continue;
        }

        debug!(count = feeds.len(), "active feeds found");

        // Current slot — used for the publish_interval rate-limit check.
        let current_slot = match rpc.get_slot().await {
            Ok(s) => s,
            Err(e) => {
                debug!(error = %e, "get_slot failed");
                continue;
            }
        };

        for feed in feeds {
            // Only crank feeds we're the bound coordinator for.
            if feed.coordinator != coordinator_keypair.pubkey() {
                continue;
            }

            // Rate-limit: don't publish before publish_interval has elapsed.
            // Skip on genesis (current_slot == 0 means "never published").
            if feed.current_slot > 0 {
                let next_allowed = feed
                    .current_slot
                    .saturating_add(feed.publish_interval_slots as u64);
                if current_slot < next_allowed {
                    continue;
                }
            }

            // Fetch the bound channel and check whether it has a newer
            // finalized round than the feed currently holds.
            let channel_data = match rpc.get_account_data(&feed.bound_channel).await {
                Ok(Some(d)) => d,
                Ok(None) => {
                    warn!(feed = %feed.pubkey, channel = %feed.bound_channel, "bound channel missing");
                    continue;
                }
                Err(e) => {
                    debug!(error = %e, "get bound channel failed");
                    continue;
                }
            };

            if channel_data.len() < CHANNEL_RANDOMNESS_OFFSET + 32 {
                continue;
            }
            if &channel_data[0..8] != DICE_CHANNEL_DISC.as_ref() {
                continue;
            }
            if channel_data[DICE_CHANNEL_STATUS_OFFSET] != DICE_CHANNEL_STATUS_FINALIZED {
                // Channel hasn't finalized a new round yet.
                continue;
            }

            let channel_round_id = u64::from_le_bytes(
                channel_data[CHANNEL_ROUND_ID_OFFSET..CHANNEL_ROUND_ID_OFFSET + 8]
                    .try_into()
                    .unwrap(),
            );
            if channel_round_id <= feed.current_round_id {
                // No new randomness to publish.
                continue;
            }

            let channel_authority: Pubkey = Pubkey::try_from(
                &channel_data[CHANNEL_AUTHORITY_OFFSET..CHANNEL_AUTHORITY_OFFSET + 32],
            )
            .unwrap_or_default();
            let channel_index_bytes: [u8; 2] = channel_data
                [CHANNEL_INDEX_OFFSET..CHANNEL_INDEX_OFFSET + 2]
                .try_into()
                .unwrap();
            let channel_index = u16::from_le_bytes(channel_index_bytes);

            let mut randomness = [0u8; 32];
            randomness.copy_from_slice(
                &channel_data[CHANNEL_RANDOMNESS_OFFSET..CHANNEL_RANDOMNESS_OFFSET + 32],
            );

            info!(
                feed = %feed.pubkey,
                feed_sequence = feed.current_sequence,
                channel = %feed.bound_channel,
                channel_round_id,
                "publishing new feed value"
            );

            let ix = solana_tx::build_publish_feed_value_ix(
                &program_id,
                &coordinator_keypair.pubkey(),
                &feed.authority,
                feed.feed_index,
                &channel_authority,
                channel_index,
                &randomness,
                channel_round_id,
            );

            match rpc
                .sign_and_send(&coordinator_keypair, vec![ix])
                .await
            {
                Ok(sig) => info!(
                    feed = %feed.pubkey,
                    sig = %sig,
                    round_id = channel_round_id,
                    "feed value published"
                ),
                Err(e) => warn!(
                    feed = %feed.pubkey,
                    error = %e,
                    "publish_feed_value TX failed"
                ),
            }
        }
    }
}

/// Query all `RandomnessFeed` accounts in Active status owned by the
/// DICE program.
async fn scan_active_feeds(rpc: &SolanaRpc, program_id: &Pubkey) -> Result<Vec<FeedRow>> {
    let accounts = rpc
        .get_program_accounts(
            program_id,
            None,
            &[
                (0, &FEED_DISCRIMINATOR),
                (FEED_STATUS_OFFSET, &[FEED_STATUS_ACTIVE]),
            ],
        )
        .await
        .context("getProgramAccounts for Active feeds")?;

    let mut out = Vec::with_capacity(accounts.len());
    for (pubkey, data) in accounts {
        if data.len() < FEED_CURRENT_ROUND_ID_OFFSET + 8 {
            continue;
        }
        let authority =
            Pubkey::try_from(&data[FEED_AUTHORITY_OFFSET..FEED_AUTHORITY_OFFSET + 32])
                .unwrap_or_default();
        let coordinator =
            Pubkey::try_from(&data[FEED_COORDINATOR_OFFSET..FEED_COORDINATOR_OFFSET + 32])
                .unwrap_or_default();
        let bound_channel =
            Pubkey::try_from(&data[FEED_BOUND_CHANNEL_OFFSET..FEED_BOUND_CHANNEL_OFFSET + 32])
                .unwrap_or_default();
        let feed_index = u16::from_le_bytes(
            data[FEED_INDEX_OFFSET..FEED_INDEX_OFFSET + 2]
                .try_into()
                .unwrap(),
        );
        let publish_interval_slots = u32::from_le_bytes(
            data[FEED_PUBLISH_INTERVAL_OFFSET..FEED_PUBLISH_INTERVAL_OFFSET + 4]
                .try_into()
                .unwrap(),
        );
        let current_sequence = u64::from_le_bytes(
            data[FEED_CURRENT_SEQUENCE_OFFSET..FEED_CURRENT_SEQUENCE_OFFSET + 8]
                .try_into()
                .unwrap(),
        );
        let current_slot = u64::from_le_bytes(
            data[FEED_CURRENT_SLOT_OFFSET..FEED_CURRENT_SLOT_OFFSET + 8]
                .try_into()
                .unwrap(),
        );
        let current_round_id = u64::from_le_bytes(
            data[FEED_CURRENT_ROUND_ID_OFFSET..FEED_CURRENT_ROUND_ID_OFFSET + 8]
                .try_into()
                .unwrap(),
        );

        // Silence "randomness field unused" lint — we don't need the current
        // randomness for the decision whether to publish; we only compare
        // round_id/slot and fetch the new randomness from the channel.
        let _ = FEED_CURRENT_RANDOMNESS_OFFSET;

        out.push(FeedRow {
            pubkey,
            authority,
            coordinator,
            bound_channel,
            feed_index,
            publish_interval_slots,
            current_slot,
            current_round_id,
            current_sequence,
        });
    }
    Ok(out)
}
