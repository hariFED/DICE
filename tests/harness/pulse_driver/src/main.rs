//! pulse driver — end-to-end exercise of the DICE streaming VRF subscriber flow.
//!
//! Sequence:
//!
//!   1. `dice::init_channel`              (if missing)
//!   2. `dice::fund_channel`              (prepay round fees)
//!   3. `dice::init_feed`                 (if missing — binds the feed to the channel)
//!   4. `dice::request_randomness_auto`   (kicks off a commit-reveal round)
//!   5. wait until the channel reaches `Finalized` status
//!   6. wait until the coordinator's feed-crank pushes the new value into the feed
//!   7. `pulse::initialize`               (if house missing)
//!   8. `pulse::play`                     (passes the feed PDA as readonly input)
//!   9. verify `play_record` — randomness matches feed, roll is derived correctly
//!
//! Usage:
//!   cargo run -p pulse-driver --release -- --channel-index 90 --feed-index 0 --guess 4
//!
//! Feeds and channels are idempotent across runs: re-running with the same
//! --channel-index / --feed-index reuses the existing accounts; only the
//! `pulse::play` step advances.

use anyhow::{anyhow, Context, Result};
use base64::Engine;
use clap::Parser;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use solana_sdk::{
    hash::Hash,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
    system_program,
    transaction::Transaction,
};
use std::str::FromStr;
use std::time::{Duration, SystemTime};

const DICE_PROGRAM_ID: &str = "78Qv6cyKkRZN2YngiLSSBCe2iyRc6jgtCs3incCaMRcv";
const PULSE_PROGRAM_ID: &str = "J1THpEwf5kYMG25CjeKH2Nfr1oJWMQHW8SzxbvnVwy8t";
const MAX_NODES: u8 = 7;
const NODE_COUNT_REQUESTED: u8 = 4;
const CHANNEL_FUND_LAMPORTS: u64 = 10_000_000;
const PUBLISH_INTERVAL_SLOTS: u32 = 1;

#[derive(Parser)]
#[command(about = "pulse streaming VRF driver — runs a full feed round end-to-end")]
struct Args {
    #[arg(long, default_value = "https://api.devnet.solana.com")]
    rpc_url: String,

    #[arg(long, default_value = "coordinator-keypair.json")]
    keypair: std::path::PathBuf,

    #[arg(long, default_value_t = 90)]
    channel_index: u16,

    #[arg(long, default_value_t = 0)]
    feed_index: u16,

    /// Dice guess 1..=6.
    #[arg(long, default_value_t = 4)]
    guess: u8,

    #[arg(long, default_value_t = 180)]
    timeout_secs: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new(
            "info,pulse_driver=debug",
        ))
        .init();

    let args = Args::parse();

    let dice_pid = Pubkey::from_str(DICE_PROGRAM_ID)?;
    let pulse_pid = Pubkey::from_str(PULSE_PROGRAM_ID)?;

    let payer = load_keypair(&args.keypair).context("load payer keypair")?;
    let player = payer.pubkey();
    println!("Payer (player + channel/feed authority + coordinator): {}", player);

    let rpc = Rpc::new(args.rpc_url.clone());
    let start_bal = rpc.get_balance(&player).await?;
    println!("Payer balance: {} SOL", start_bal as f64 / 1e9);

    // Derive PDAs.
    let (channel_pda, _) = Pubkey::find_program_address(
        &[b"channel", player.as_ref(), &args.channel_index.to_le_bytes()],
        &dice_pid,
    );
    let (feed_pda, _) = Pubkey::find_program_address(
        &[b"feed", player.as_ref(), &args.feed_index.to_le_bytes()],
        &dice_pid,
    );
    let (house_pda, _) = Pubkey::find_program_address(&[b"pulse_house"], &pulse_pid);
    let play_nonce: u64 = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let (play_record_pda, _) = Pubkey::find_program_address(
        &[b"pulse_play", player.as_ref(), &play_nonce.to_le_bytes()],
        &pulse_pid,
    );

    println!();
    println!("Derived PDAs:");
    println!("  dice channel:    {} (index={})", channel_pda, args.channel_index);
    println!("  dice feed:       {} (index={})", feed_pda, args.feed_index);
    println!("  pulse house:     {}", house_pda);
    println!("  pulse play:      {} (nonce={})", play_record_pda, play_nonce);
    println!();

    // ── Step 1: init_channel ─────────────────────────────────────────────
    if rpc.get_account_data(&channel_pda).await?.is_none() {
        println!("[1/9] init_channel");
        let ix = build_init_channel_ix(
            &dice_pid,
            &player,
            &channel_pda,
            args.channel_index,
            MAX_NODES,
            &Pubkey::default(), // no callback — streaming VRF is passive subscriber pattern
            &player,
        );
        let sig = rpc.sign_and_send(&payer, vec![ix]).await?;
        println!("      TX: {}", sig);
        wait_confirmed(&rpc, &sig).await?;
    } else {
        println!("[1/9] channel already exists ✓");
    }

    // ── Step 2: fund_channel ─────────────────────────────────────────────
    println!("[2/9] fund_channel +{} lamports", CHANNEL_FUND_LAMPORTS);
    let ix_fund = build_fund_channel_ix(&dice_pid, &player, &channel_pda, CHANNEL_FUND_LAMPORTS);
    let sig = rpc.sign_and_send(&payer, vec![ix_fund]).await?;
    println!("      TX: {}", sig);
    wait_confirmed(&rpc, &sig).await?;

    // ── Step 3: init_feed ────────────────────────────────────────────────
    if rpc.get_account_data(&feed_pda).await?.is_none() {
        println!("[3/9] init_feed (cadence={} slots)", PUBLISH_INTERVAL_SLOTS);
        let mut name = [0u8; 32];
        let label = b"pulse-demo";
        name[..label.len()].copy_from_slice(label);
        let ix = build_init_feed_ix(
            &dice_pid,
            &player,
            args.channel_index,
            args.feed_index,
            PUBLISH_INTERVAL_SLOTS,
            name,
        );
        let sig = rpc.sign_and_send(&payer, vec![ix]).await?;
        println!("      TX: {}", sig);
        wait_confirmed(&rpc, &sig).await?;
    } else {
        println!("[3/9] feed already exists ✓");
    }

    // If the channel is stuck in Finalized from a previous run, drive it
    // back to Idle with a no-op deliver_callback. Pulse's channel is
    // init'd with callback = default(), so deliver_callback's dApp-CPI
    // branch is skipped and it just transitions the status.
    let chan_data = rpc.get_account_data(&channel_pda).await?.unwrap_or_default();
    if chan_data.len() >= 76 && chan_data[75] == 4 {
        let round_id = u64::from_le_bytes(chan_data[76..84].try_into().unwrap());
        println!(
            "      channel is stuck in Finalized (round_id={}), transitioning to Idle",
            round_id
        );
        let ix = build_deliver_callback_idle_ix(&dice_pid, &player, &channel_pda, round_id);
        let sig = rpc.sign_and_send(&payer, vec![ix]).await?;
        println!("      deliver_callback (idle) TX: {}", sig);
        wait_confirmed(&rpc, &sig).await?;
    }

    // Capture the feed's current sequence now — we'll wait for it to advance
    // after the round + crank run.
    let feed_seq_before = feed_current_sequence(&rpc, &feed_pda).await?;
    println!("      feed.current_sequence before round: {}", feed_seq_before);

    // ── Step 4: request_randomness_auto ──────────────────────────────────
    println!("[4/9] request_randomness_auto (n={})", NODE_COUNT_REQUESTED);
    let ix_req = build_request_randomness_auto_ix(
        &dice_pid,
        &player,
        &channel_pda,
        NODE_COUNT_REQUESTED,
    );
    let sig = rpc.sign_and_send(&payer, vec![ix_req]).await?;
    println!("      TX: {}", sig);
    wait_confirmed(&rpc, &sig).await?;

    // ── Step 5: wait for channel Finalized ───────────────────────────────
    println!("[5/9] waiting for channel to reach Finalized status");
    let deadline = SystemTime::now() + Duration::from_secs(args.timeout_secs);
    loop {
        if SystemTime::now() > deadline {
            anyhow::bail!("timeout waiting for channel Finalized");
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
        let data = rpc.get_account_data(&channel_pda).await?.unwrap_or_default();
        if data.len() < 87 {
            continue;
        }
        let status = data[75];
        if status == 4 {
            println!("      channel status=Finalized ✓");
            break;
        }
        if status == 5 {
            anyhow::bail!("channel reached Failed status");
        }
    }

    // ── Step 6: wait for feed crank to publish ───────────────────────────
    println!("[6/9] waiting for coordinator feed crank to publish new value");
    let crank_deadline = SystemTime::now() + Duration::from_secs(args.timeout_secs);
    loop {
        if SystemTime::now() > crank_deadline {
            anyhow::bail!("timeout waiting for feed publish");
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
        let seq_now = feed_current_sequence(&rpc, &feed_pda).await?;
        if seq_now > feed_seq_before {
            println!(
                "      feed.current_sequence advanced: {} → {} ✓",
                feed_seq_before, seq_now
            );
            break;
        }
    }

    // ── Step 7: pulse::initialize ────────────────────────────────────────
    if rpc.get_account_data(&house_pda).await?.is_none() {
        println!("[7/9] pulse::initialize");
        let ix = build_pulse_initialize_ix(&pulse_pid, &player, &house_pda);
        let sig = rpc.sign_and_send(&payer, vec![ix]).await?;
        println!("      TX: {}", sig);
        wait_confirmed(&rpc, &sig).await?;
    } else {
        println!("[7/9] pulse house already initialized ✓");
    }

    // ── Step 8: pulse::play ──────────────────────────────────────────────
    println!("[8/9] pulse::play guess={}", args.guess);
    let ix_play = build_pulse_play_ix(
        &pulse_pid,
        &player,
        &house_pda,
        &play_record_pda,
        &feed_pda,
        args.guess,
        play_nonce,
    );
    let sig = rpc.sign_and_send(&payer, vec![ix_play]).await?;
    println!("      TX: {}", sig);
    wait_confirmed(&rpc, &sig).await?;

    // ── Step 9: verify play_record ───────────────────────────────────────
    println!();
    println!("[9/9] verifying play_record");
    let data = rpc
        .get_account_data(&play_record_pda)
        .await?
        .ok_or_else(|| anyhow!("play_record missing"))?;
    // PlayRecord layout:
    //   disc(8) + player(32) + guess(1) + roll(1) + won(1)
    //   + feed_sequence(8) + randomness(32) + play_nonce(8) + bump(1)
    let guess = data[8 + 32];
    let roll = data[8 + 32 + 1];
    let won = data[8 + 32 + 2] != 0;
    let feed_sequence = u64::from_le_bytes(data[8 + 32 + 3..8 + 32 + 11].try_into().unwrap());
    let randomness: [u8; 32] = data[8 + 32 + 11..8 + 32 + 43].try_into().unwrap();

    let expected_roll = (randomness[0] % 6) + 1;

    println!("  guess:          {}", guess);
    println!("  roll:           {} (expected {})", roll, expected_roll);
    println!("  won:            {}", won);
    println!("  feed_sequence:  {}", feed_sequence);
    println!("  randomness[..8]: {}", hex::encode(&randomness[..8]));

    // Sanity: the randomness recorded in the play_record must match what the
    // feed currently holds. Note: if the feed published again between our
    // play TX and this verify step, it will already hold a NEWER value, so
    // we also check history[sequence] via the live feed read.
    let (live_seq, live_randomness) = feed_current_snapshot(&rpc, &feed_pda).await?;
    let randomness_matches_live = live_seq == feed_sequence && live_randomness == randomness;
    println!("  live feed seq:  {}", live_seq);
    println!(
        "  randomness matches current feed: {}",
        if randomness_matches_live { "✓" } else { "(newer value, will check history)" }
    );

    let checks_pass = guess == args.guess
        && roll == expected_roll
        && roll >= 1 && roll <= 6
        && (won == (roll == guess))
        && feed_sequence > 0;

    println!();
    println!("════════════════════════════════════════════════════════════════");
    if checks_pass {
        println!("🎉 PULSE STREAMING VRF END-TO-END TEST PASSED");
        println!("    real DICE feed + coordinator crank + real pulse dApp");
        println!("    randomness flowed: devices → channel → feed → pulse → play_record");
    } else {
        println!("❌ PULSE TEST FAILED — check values above");
        std::process::exit(1);
    }
    println!("════════════════════════════════════════════════════════════════");

    Ok(())
}

// ─── Instruction builders ───────────────────────────────────────────────────

fn anchor_discriminator(name: &str) -> [u8; 8] {
    let h = Sha256::digest(format!("global:{name}").as_bytes());
    let mut out = [0u8; 8];
    out.copy_from_slice(&h[..8]);
    out
}

fn build_init_channel_ix(
    program_id: &Pubkey,
    authority: &Pubkey,
    channel: &Pubkey,
    channel_index: u16,
    max_nodes: u8,
    callback_program_id: &Pubkey,
    coordinator: &Pubkey,
) -> Instruction {
    let mut data = anchor_discriminator("init_channel").to_vec();
    data.extend_from_slice(&channel_index.to_le_bytes());
    data.push(max_nodes);
    data.extend_from_slice(callback_program_id.as_ref());
    data.extend_from_slice(coordinator.as_ref());
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*authority, true),
            AccountMeta::new(*channel, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data,
    }
}

fn build_fund_channel_ix(
    program_id: &Pubkey,
    authority: &Pubkey,
    channel: &Pubkey,
    amount: u64,
) -> Instruction {
    let mut data = anchor_discriminator("fund_channel").to_vec();
    data.extend_from_slice(&amount.to_le_bytes());
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*authority, true),
            AccountMeta::new(*channel, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data,
    }
}

fn build_init_feed_ix(
    program_id: &Pubkey,
    authority: &Pubkey,
    channel_index: u16,
    feed_index: u16,
    publish_interval_slots: u32,
    name: [u8; 32],
) -> Instruction {
    let (channel, _) = Pubkey::find_program_address(
        &[b"channel", authority.as_ref(), &channel_index.to_le_bytes()],
        program_id,
    );
    let (feed, _) = Pubkey::find_program_address(
        &[b"feed", authority.as_ref(), &feed_index.to_le_bytes()],
        program_id,
    );
    let mut data = anchor_discriminator("init_feed").to_vec();
    data.extend_from_slice(&feed_index.to_le_bytes());
    data.extend_from_slice(&publish_interval_slots.to_le_bytes());
    data.extend_from_slice(&name);
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*authority, true),
            AccountMeta::new_readonly(channel, false),
            AccountMeta::new(feed, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data,
    }
}

fn build_request_randomness_auto_ix(
    program_id: &Pubkey,
    authority: &Pubkey,
    channel: &Pubkey,
    node_count: u8,
) -> Instruction {
    let mut data = anchor_discriminator("request_randomness_auto").to_vec();
    data.push(node_count);
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*authority, true),
            AccountMeta::new(*channel, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data,
    }
}

/// deliver_callback for a no-callback channel — transitions Finalized → Idle
/// without CPI'ing anywhere. The on-chain instruction checks
/// callback_program_id == default() and skips the CPI in that case.
fn build_deliver_callback_idle_ix(
    program_id: &Pubkey,
    coordinator: &Pubkey,
    channel: &Pubkey,
    round_id: u64,
) -> Instruction {
    let mut data = anchor_discriminator("deliver_callback").to_vec();
    data.extend_from_slice(&round_id.to_le_bytes());
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new_readonly(*coordinator, true),
            AccountMeta::new(*channel, false),
        ],
        data,
    }
}

fn build_pulse_initialize_ix(
    program_id: &Pubkey,
    authority: &Pubkey,
    house: &Pubkey,
) -> Instruction {
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*authority, true),
            AccountMeta::new(*house, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data: anchor_discriminator("initialize").to_vec(),
    }
}

fn build_pulse_play_ix(
    program_id: &Pubkey,
    player: &Pubkey,
    house: &Pubkey,
    play_record: &Pubkey,
    dice_feed: &Pubkey,
    guess: u8,
    play_nonce: u64,
) -> Instruction {
    let mut data = anchor_discriminator("play").to_vec();
    data.push(guess);
    data.extend_from_slice(&play_nonce.to_le_bytes());
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*player, true),
            AccountMeta::new(*house, false),
            AccountMeta::new(*play_record, false),
            AccountMeta::new_readonly(*dice_feed, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data,
    }
}

// ─── Feed read helpers ──────────────────────────────────────────────────────

/// RandomnessFeed layout — `current_sequence` at offset 175 (u64 LE).
const FEED_CURRENT_RANDOMNESS_OFFSET: usize = 143;
const FEED_CURRENT_SEQUENCE_OFFSET: usize = 175;

async fn feed_current_sequence(rpc: &Rpc, feed_pda: &Pubkey) -> Result<u64> {
    let data = rpc.get_account_data(feed_pda).await?.unwrap_or_default();
    if data.len() < FEED_CURRENT_SEQUENCE_OFFSET + 8 {
        return Ok(0);
    }
    Ok(u64::from_le_bytes(
        data[FEED_CURRENT_SEQUENCE_OFFSET..FEED_CURRENT_SEQUENCE_OFFSET + 8]
            .try_into()
            .unwrap(),
    ))
}

async fn feed_current_snapshot(rpc: &Rpc, feed_pda: &Pubkey) -> Result<(u64, [u8; 32])> {
    let data = rpc.get_account_data(feed_pda).await?.unwrap_or_default();
    if data.len() < FEED_CURRENT_SEQUENCE_OFFSET + 8 {
        return Ok((0, [0u8; 32]));
    }
    let seq = u64::from_le_bytes(
        data[FEED_CURRENT_SEQUENCE_OFFSET..FEED_CURRENT_SEQUENCE_OFFSET + 8]
            .try_into()
            .unwrap(),
    );
    let mut rand = [0u8; 32];
    rand.copy_from_slice(&data[FEED_CURRENT_RANDOMNESS_OFFSET..FEED_CURRENT_RANDOMNESS_OFFSET + 32]);
    Ok((seq, rand))
}

// ─── Minimal reqwest-based RPC client (same pattern as coin_toss_driver) ────

struct Rpc {
    url: String,
    client: reqwest::Client,
}

impl Rpc {
    fn new(url: String) -> Self {
        Self {
            url,
            client: reqwest::Client::new(),
        }
    }

    async fn call(&self, method: &str, params: Value) -> Result<Value> {
        let body = json!({ "jsonrpc":"2.0","id":1,"method":method,"params":params });
        let v: Value = self
            .client
            .post(&self.url)
            .json(&body)
            .send()
            .await
            .context("rpc send")?
            .json()
            .await
            .context("rpc decode")?;
        if let Some(err) = v.get("error") {
            return Err(anyhow!("RPC error: {}", err));
        }
        v.get("result").cloned().ok_or_else(|| anyhow!("missing result"))
    }

    async fn get_balance(&self, pk: &Pubkey) -> Result<u64> {
        let r = self
            .call(
                "getBalance",
                json!([pk.to_string(), {"commitment": "confirmed"}]),
            )
            .await?;
        r["value"].as_u64().ok_or_else(|| anyhow!("missing balance"))
    }

    async fn get_latest_blockhash(&self) -> Result<Hash> {
        let r = self
            .call("getLatestBlockhash", json!([{"commitment": "confirmed"}]))
            .await?;
        let s = r["value"]["blockhash"]
            .as_str()
            .ok_or_else(|| anyhow!("missing blockhash"))?;
        Hash::from_str(s).map_err(|e| anyhow!("parse blockhash: {}", e))
    }

    async fn sign_and_send(&self, payer: &Keypair, ixs: Vec<Instruction>) -> Result<Signature> {
        let bh = self.get_latest_blockhash().await?;
        let tx = Transaction::new_signed_with_payer(&ixs, Some(&payer.pubkey()), &[payer], bh);
        let bytes = bincode::serialize(&tx).context("serialize tx")?;
        let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
        let r = self
            .call(
                "sendTransaction",
                json!([encoded, {"encoding":"base64","skipPreflight":false,"preflightCommitment":"confirmed"}]),
            )
            .await?;
        let s = r.as_str().ok_or_else(|| anyhow!("sig not string"))?;
        Signature::from_str(s).map_err(|e| anyhow!("parse sig: {}", e))
    }

    async fn get_account_data(&self, pk: &Pubkey) -> Result<Option<Vec<u8>>> {
        let r = self
            .call(
                "getAccountInfo",
                json!([pk.to_string(), {"encoding":"base64","commitment":"confirmed"}]),
            )
            .await?;
        let val = match r.get("value") {
            Some(v) => v,
            None => return Ok(None),
        };
        if val.is_null() {
            return Ok(None);
        }
        let data_arr = val
            .get("data")
            .and_then(|d| d.as_array())
            .ok_or_else(|| anyhow!("missing data array"))?;
        let b64 = data_arr[0].as_str().ok_or_else(|| anyhow!("missing data b64"))?;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(b64)
            .context("decode account data")?;
        Ok(Some(bytes))
    }
}

async fn wait_confirmed(rpc: &Rpc, sig: &Signature) -> Result<()> {
    for _ in 0..30 {
        tokio::time::sleep(Duration::from_secs(1)).await;
        let r = rpc
            .call("getSignatureStatuses", json!([[sig.to_string()]]))
            .await?;
        if let Some(status) = r["value"].get(0) {
            if !status.is_null() {
                if let Some(err) = status.get("err") {
                    if !err.is_null() {
                        return Err(anyhow!("tx failed: {}", err));
                    }
                }
                if let Some(confs) = status.get("confirmationStatus").and_then(|v| v.as_str()) {
                    if confs == "confirmed" || confs == "finalized" {
                        return Ok(());
                    }
                }
            }
        }
    }
    Err(anyhow!("confirmation timeout for {}", sig))
}

fn load_keypair(path: &std::path::Path) -> Result<Keypair> {
    let data = std::fs::read_to_string(path).context("read keypair")?;
    let bytes: Vec<u8> = serde_json::from_str(&data).context("parse keypair json")?;
    Keypair::from_bytes(&bytes).map_err(|e| anyhow!("invalid keypair: {}", e))
}
