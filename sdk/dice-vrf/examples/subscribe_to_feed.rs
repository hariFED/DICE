//! Example: subscribing to a streaming RandomnessFeed.
//!
//! DICE streaming VRF works like this:
//!
//! 1. A developer calls `init_feed` **once** to create a `RandomnessFeed` PDA
//!    bound to one of their existing `DiceChannel`s.
//! 2. The DICE coordinator runs commit-reveal rounds on that channel on a
//!    cadence (the `publish_interval_slots` chosen at init) and calls
//!    `publish_feed_value` to push each finalized value into the feed PDA.
//! 3. Subscriber programs read the feed PDA as a **passive account input**
//!    in their own instructions — no callback, no per-request transaction,
//!    no CPI back into DICE. Off-chain consumers simply `get_account` the
//!    feed PDA and decode its current snapshot.
//!
//! This example is intentionally RPC-agnostic: it shows how to derive the
//! feed PDA, how to decode a raw account buffer into a `FeedSnapshot`, and
//! what fields are available to consumers. Plug in your preferred Solana
//! RPC client (solana-client, Helius, Triton, etc.) to actually fetch the
//! account bytes from chain.
//!
//! Run:
//! ```bash
//! cargo run --example subscribe_to_feed
//! ```

use dice_vrf::cpi::{decode_feed_current_randomness, FeedSnapshot};
use dice_vrf::pda::feed_pda;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

fn main() {
    let program_id = Pubkey::from_str(dice_vrf::DICE_PROGRAM_ID)
        .expect("DICE_PROGRAM_ID is a valid base58 pubkey");

    // ── Step 1: derive the feed PDA ──────────────────────────────────────
    //
    // In production, `feed_authority` and `feed_index` come from whoever
    // created the feed. The PDA is fully deterministic, so subscribers need
    // nothing else to find it.
    let feed_authority = Pubkey::new_unique();
    let feed_index: u16 = 0;

    let (feed_pda_key, bump) = feed_pda(&feed_authority, feed_index, &program_id);

    println!("DICE Streaming VRF — Subscriber Pattern");
    println!("=======================================");
    println!();
    println!("Step 1: derive the feed PDA");
    println!("  authority:  {feed_authority}");
    println!("  feed_index: {feed_index}");
    println!("  program_id: {program_id}");
    println!("  PDA:        {feed_pda_key}");
    println!("  bump:       {bump}");
    println!();

    // ── Step 2: fetch account bytes (via your RPC of choice) ─────────────
    //
    // In real code you would write something like:
    //   let account = rpc.get_account(&feed_pda_key).await?;
    //   let snapshot = decode_feed_current_randomness(&account.data);
    //
    // For this example we construct a synthetic account buffer that matches
    // what the on-chain program would write after three publishes.
    let synthetic_account_data = build_synthetic_feed_account();
    println!("Step 2: fetch account bytes (synthetic for this demo)");
    println!("  account_data.len() = {}", synthetic_account_data.len());
    println!();

    // ── Step 3: decode the current snapshot ──────────────────────────────
    println!("Step 3: decode the current snapshot");
    match decode_feed_current_randomness(&synthetic_account_data) {
        Some(FeedSnapshot {
            randomness,
            sequence,
            slot,
            round_id,
            status,
        }) => {
            println!("  sequence:   {sequence}");
            println!("  slot:       {slot}");
            println!("  round_id:   {round_id}");
            println!(
                "  status:     {} ({})",
                status,
                if status == 0 { "Active" } else { "Closed" }
            );
            println!("  randomness: {}", hex::encode(randomness));
            println!();
            println!("That 32-byte value is backed by a real finalized DICE");
            println!("commit-reveal round on the feed's bound channel. The");
            println!("subscriber did not send any transactions, did not pay");
            println!("any protocol fee, and did not wait on a callback.");
        }
        None => {
            println!("  (feed not yet published — still at genesis)");
        }
    }
    println!();

    // ── On-chain subscriber pattern (for Anchor programs) ────────────────
    println!("On-chain subscriber pattern (Anchor):");
    println!();
    println!("  #[derive(Accounts)]");
    println!("  pub struct MyGameRoll<'info> {{");
    println!("      pub player: Signer<'info>,");
    println!("      /// The DICE feed, passed as readonly. No CPI needed.");
    println!("      /// CHECK: validated by decode_feed_current_randomness");
    println!("      pub dice_feed: AccountInfo<'info>,");
    println!("  }}");
    println!();
    println!("  pub fn my_game_roll(ctx: Context<MyGameRoll>) -> Result<()> {{");
    println!("      let data = ctx.accounts.dice_feed.try_borrow_data()?;");
    println!("      let snap = dice_vrf::cpi::decode_feed_current_randomness(&data)");
    println!("          .ok_or(ProgramError::InvalidAccountData)?;");
    println!("      // Use snap.randomness directly in your game logic.");
    println!("      msg!(\"roll seed: {{:?}}\", &snap.randomness[..8]);");
    println!("      Ok(())");
    println!("  }}");
    println!();
    println!("Subscriber pattern summary:");
    println!("  - 0 transactions sent by the subscriber to DICE");
    println!("  - 0 callback handlers to wire up");
    println!("  - every read returns the most recently pushed value");
    println!("  - each value traces back to a real finalized commit-reveal round");
}

/// Build a 1024-byte buffer that mirrors the on-chain `RandomnessFeed` layout
/// after a coordinator has published `sequence=3`. Used purely for this
/// example — in real integrations the account data comes from an RPC call.
fn build_synthetic_feed_account() -> Vec<u8> {
    // Layout mirrors programs/dice/src/state/randomness_feed.rs:
    //   [0..8]     discriminator
    //   [8..40]    authority
    //   [40..72]   coordinator
    //   [72..104]  bound_channel
    //   [104..106] feed_index
    //   [106..138] name
    //   [138..142] publish_interval_slots
    //   [142]      status
    //   [143..175] current_randomness
    //   [175..183] current_sequence
    //   [183..191] current_slot
    //   [191..199] current_round_id
    //   ...
    let mut data = vec![0u8; 1024];
    data[142] = 0; // Active
    data[143..175].fill(0x7D); // current_randomness
    data[175..183].copy_from_slice(&3u64.to_le_bytes()); // sequence
    data[183..191].copy_from_slice(&1_234_567u64.to_le_bytes()); // slot
    data[191..199].copy_from_slice(&42u64.to_le_bytes()); // round_id
    data
}
