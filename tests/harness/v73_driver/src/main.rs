//! v7.3 on-chain node-selection driver.
//!
//! Validates the v7.3 security win end-to-end without needing live
//! devices or a running coordinator:
//!
//!   1. Register 5 `DeviceRegistry` PDAs for the known ESP32-S3 device
//!      pubkeys (idempotent — skips any that already exist).
//!   2. Create a fresh DiceChannel (no callback, no coordinator needed).
//!   3. Call `request_randomness_auto` with the SlotHashes sysvar and all
//!      5 DeviceRegistry accounts in `remaining_accounts`.
//!   4. Read the channel account back and verify that
//!      `channel.device_pubkeys[0..4]` was populated with 4 of the 5
//!      registered device pubkeys — proof that on-chain Fisher-Yates
//!      selection actually ran and picked the devices deterministically.
//!
//! No commits/reveals are submitted — this test is about the selection
//! algorithm and the instruction wiring, not the round itself.

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
    sysvar::slot_hashes,
    transaction::Transaction,
};
use std::str::FromStr;
use std::time::Duration;

const DICE_PROGRAM_ID: &str = "78Qv6cyKkRZN2YngiLSSBCe2iyRc6jgtCs3incCaMRcv";

/// The 5 ESP32-S3 devices currently bound to the cluster.
/// Captured from the coordinator's /nodes endpoint during the A4 stress run.
const KNOWN_DEVICES_HEX: &[&str] = &[
    "029ac5ec1f7bf25ead7e6b3479baa24e789ed772bbf0840dd37623e4baab353cc4",
    "03856d1a3f860a4524d7cf1eddde165cceaae0f096fc97bc7d3ce709d3ae1ad20a",
    "0271270853b3ad239b2c421874f9b25efc53c6fe3298f18a7b24c19c4f8bcc7c5d",
    "03869239a204df3a1ebd17c8084c914f8ca2fb9ec865258f3830c3d7ae75e5a774",
    "0208949b2ff57140e8361427377151583a5581b171e7536a4bf5a6668ede4f4c9c",
];

#[derive(Parser)]
struct Args {
    #[arg(long, default_value = "https://api.devnet.solana.com")]
    rpc_url: String,

    #[arg(long, default_value = "coordinator-keypair.json")]
    keypair: std::path::PathBuf,

    #[arg(long, default_value_t = 700)]
    channel_index: u16,

    #[arg(long, default_value_t = 4)]
    node_count: u8,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let rpc = Rpc::new(args.rpc_url);
    let dice_pid = Pubkey::from_str(DICE_PROGRAM_ID)?;
    let payer = load_keypair(&args.keypair)?;
    let player = payer.pubkey();

    println!("v7.3 on-chain selection driver");
    println!("  payer: {}", player);
    println!("  dice program: {}", dice_pid);
    println!();

    // Parse known device pubkeys.
    let mut devices: Vec<[u8; 33]> = Vec::new();
    for hex_str in KNOWN_DEVICES_HEX {
        let bytes = hex::decode(hex_str).context("hex decode")?;
        if bytes.len() != 33 {
            anyhow::bail!("device pubkey must be 33 bytes (got {})", bytes.len());
        }
        let mut pk = [0u8; 33];
        pk.copy_from_slice(&bytes);
        devices.push(pk);
    }
    println!("Known devices: {}", devices.len());

    // ── Step 1: register each device if missing ─────────────────────────
    println!();
    println!("[1/4] Registering DeviceRegistry PDAs...");
    let mut device_registry_pdas: Vec<Pubkey> = Vec::new();
    for (i, device_pk) in devices.iter().enumerate() {
        let device_id: [u8; 32] = Sha256::digest(device_pk).into();
        let (registry_pda, _) =
            Pubkey::find_program_address(&[b"device", &device_id], &dice_pid);
        device_registry_pdas.push(registry_pda);

        if rpc.get_account_data(&registry_pda).await?.is_some() {
            println!("  [{}] {}... already registered ✓", i, &hex::encode(device_pk)[..16]);
            continue;
        }

        println!("  [{}] {}... registering", i, &hex::encode(device_pk)[..16]);
        let ix = build_register_device_ix(&dice_pid, &player, &registry_pda, device_id, *device_pk);
        let sig = rpc.sign_and_send(&payer, vec![ix]).await?;
        println!("        TX: {}", sig);
        wait_confirmed(&rpc, &sig).await?;
    }

    // ── Step 2: create a fresh test channel ────────────────────────────
    let (channel_pda, _) = Pubkey::find_program_address(
        &[b"channel", player.as_ref(), &args.channel_index.to_le_bytes()],
        &dice_pid,
    );
    println!();
    println!("[2/4] Setting up test channel {} (index {})", channel_pda, args.channel_index);
    if rpc.get_account_data(&channel_pda).await?.is_none() {
        let ix = build_init_channel_ix(
            &dice_pid,
            &player,
            &channel_pda,
            args.channel_index,
            7, // max_nodes
            &Pubkey::default(),
            &player, // coordinator = self (not used in this test)
        );
        let sig = rpc.sign_and_send(&payer, vec![ix]).await?;
        println!("      init_channel TX: {}", sig);
        wait_confirmed(&rpc, &sig).await?;
    } else {
        println!("      channel already exists");
    }

    // Fund the channel (one round worth, 0.003 SOL buffer).
    let ix = build_fund_channel_ix(&dice_pid, &player, &channel_pda, 3_000_000);
    let sig = rpc.sign_and_send(&payer, vec![ix]).await?;
    println!("      fund_channel TX: {}", sig);
    wait_confirmed(&rpc, &sig).await?;

    // Drain to Idle if stuck in Finalized from an earlier run.
    let data = rpc.get_account_data(&channel_pda).await?.unwrap_or_default();
    if data.len() >= 84 && data[75] == 4 {
        let round_id = u64::from_le_bytes(data[76..84].try_into().unwrap());
        let ix = build_deliver_callback_empty_ix(&dice_pid, &player, &channel_pda, round_id);
        let sig = rpc.sign_and_send(&payer, vec![ix]).await?;
        println!("      drain-to-idle deliver_callback TX: {}", sig);
        wait_confirmed(&rpc, &sig).await?;
    }

    // ── Step 3: call request_randomness_auto WITH on-chain selection ───
    println!();
    println!("[3/4] request_randomness_auto with SlotHashes + {} DeviceRegistry accounts", device_registry_pdas.len());

    // Build the accounts list. Fixed slots first, then remaining:
    //   [authority, channel, system_program, | slot_hashes, device_registry * 5]
    let mut accounts = vec![
        AccountMeta::new(player, true),
        AccountMeta::new(channel_pda, false),
        AccountMeta::new_readonly(system_program::id(), false),
    ];
    // First remaining account MUST be SlotHashes sysvar.
    accounts.push(AccountMeta::new_readonly(slot_hashes::id(), false));
    // Then all DeviceRegistry PDAs.
    for pda in &device_registry_pdas {
        accounts.push(AccountMeta::new_readonly(*pda, false));
    }

    let mut data = anchor_discriminator("request_randomness_auto").to_vec();
    data.push(args.node_count);
    let ix = Instruction {
        program_id: dice_pid,
        accounts,
        data,
    };
    let sig = rpc.sign_and_send(&payer, vec![ix]).await?;
    println!("      request_randomness_auto TX: {}", sig);
    wait_confirmed(&rpc, &sig).await?;

    // ── Step 4: read channel state and verify on-chain selection ──────
    println!();
    println!("[4/4] Verifying channel.device_pubkeys was populated on-chain");
    let chan = rpc.get_account_data(&channel_pda).await?
        .ok_or_else(|| anyhow!("channel vanished"))?;

    // DiceChannel fixed offsets:
    //   disc(8) + authority(32) + coordinator(32) + index(2)
    //   + max_nodes(1) + status(1) + round_id(8) + node_count(1)
    //   + commits_received(1) + reveals_received(1) + created_slot(8)
    //   + commit_deadline_slot(8) + reveal_deadline_slot(8) + balance(8)
    //   + callback_program_id(32) + randomness(32) = 183 bytes
    // Then Vec fields:
    //   device_ids Vec: 4-byte length + N*32
    //   device_pubkeys Vec: 4-byte length + N*33
    let status = chan[75];
    let round_id = u64::from_le_bytes(chan[76..84].try_into().unwrap());
    let on_chain_node_count = chan[84];
    let device_ids_len = u32::from_le_bytes(chan[183..187].try_into().unwrap()) as usize;
    let device_pubkeys_start = 183 + 4 + device_ids_len * 32 + 4;

    println!("  channel status: {} (1=Pending expected)", status);
    println!("  round_id: {}", round_id);
    println!("  node_count: {}", on_chain_node_count);
    println!();
    println!("  on-chain selected devices:");
    let mut selected: Vec<[u8; 33]> = Vec::new();
    for i in 0..on_chain_node_count as usize {
        let start = device_pubkeys_start + i * 33;
        if start + 33 > chan.len() {
            break;
        }
        let pk: [u8; 33] = chan[start..start + 33].try_into().unwrap();
        let any_nonzero = pk.iter().any(|b| *b != 0);
        let in_known = devices.contains(&pk);
        println!(
            "    [{}] {}... {} {}",
            i,
            hex::encode(&pk)[..16].to_string(),
            if any_nonzero { "✓ non-zero" } else { "✗ ZERO" },
            if in_known { "✓ matches a registered device" } else { "✗ UNKNOWN" }
        );
        if any_nonzero {
            selected.push(pk);
        }
    }

    // All picked devices should be in the registered set, and we should
    // have exactly `node_count` non-zero picks.
    let all_valid = selected.len() == args.node_count as usize
        && selected.iter().all(|pk| devices.contains(pk));
    let unique = {
        let mut sorted = selected.clone();
        sorted.sort();
        sorted.dedup();
        sorted.len() == selected.len()
    };

    println!();
    println!("════════════════════════════════════════════════════════════════");
    if status == 1 && all_valid && unique {
        println!("✅ v7.3 ON-CHAIN SELECT_NODES TEST PASSED");
        println!("    channel.status = Pending after request_randomness_auto");
        println!("    {} devices selected on-chain via Fisher-Yates(SlotHashes)", selected.len());
        println!("    all picks are registered devices, no duplicates");
    } else {
        println!("❌ v7.3 TEST FAILED");
        println!("    status={}  selected={}/{}  all_valid={}  unique={}",
            status, selected.len(), args.node_count, all_valid, unique);
        std::process::exit(1);
    }
    println!("════════════════════════════════════════════════════════════════");

    Ok(())
}

// ─── Instruction builders ───────────────────────────────────────────────

fn anchor_discriminator(name: &str) -> [u8; 8] {
    let h = Sha256::digest(format!("global:{name}").as_bytes());
    let mut out = [0u8; 8];
    out.copy_from_slice(&h[..8]);
    out
}

fn build_register_device_ix(
    program_id: &Pubkey,
    payer: &Pubkey,
    registry_pda: &Pubkey,
    device_id: [u8; 32],
    device_pubkey: [u8; 33],
) -> Instruction {
    let mut data = anchor_discriminator("register_device").to_vec();
    data.extend_from_slice(&device_id);
    data.extend_from_slice(&device_pubkey);
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*payer, true),
            AccountMeta::new(*registry_pda, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data,
    }
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

fn build_deliver_callback_empty_ix(
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

// ─── Minimal RPC ───────────────────────────────────────────────────────

struct Rpc {
    url: String,
    client: reqwest::Client,
}
impl Rpc {
    fn new(url: String) -> Self {
        Self { url, client: reqwest::Client::new() }
    }
    async fn call(&self, method: &str, params: Value) -> Result<Value> {
        let body = json!({ "jsonrpc":"2.0","id":1,"method":method,"params":params });
        let v: Value = self.client.post(&self.url).json(&body).send().await?.json().await?;
        if let Some(err) = v.get("error") {
            return Err(anyhow!("RPC error: {}", err));
        }
        v.get("result").cloned().ok_or_else(|| anyhow!("missing result"))
    }
    async fn get_latest_blockhash(&self) -> Result<Hash> {
        let r = self.call("getLatestBlockhash", json!([{"commitment":"confirmed"}])).await?;
        let s = r["value"]["blockhash"].as_str().ok_or_else(|| anyhow!("missing blockhash"))?;
        Hash::from_str(s).map_err(|e| anyhow!("parse blockhash: {}", e))
    }
    async fn sign_and_send(&self, payer: &Keypair, ixs: Vec<Instruction>) -> Result<Signature> {
        let bh = self.get_latest_blockhash().await?;
        let tx = Transaction::new_signed_with_payer(&ixs, Some(&payer.pubkey()), &[payer], bh);
        let bytes = bincode::serialize(&tx)?;
        let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
        let r = self.call("sendTransaction",
            json!([encoded, {"encoding":"base64","skipPreflight":false,"preflightCommitment":"confirmed"}])).await?;
        let s = r.as_str().ok_or_else(|| anyhow!("sig not string"))?;
        Signature::from_str(s).map_err(|e| anyhow!("parse sig: {}", e))
    }
    async fn get_account_data(&self, pk: &Pubkey) -> Result<Option<Vec<u8>>> {
        let r = self.call("getAccountInfo",
            json!([pk.to_string(), {"encoding":"base64","commitment":"confirmed"}])).await?;
        let val = match r.get("value") { Some(v) => v, None => return Ok(None) };
        if val.is_null() { return Ok(None); }
        let arr = val.get("data").and_then(|d| d.as_array())
            .ok_or_else(|| anyhow!("missing data array"))?;
        let b64 = arr[0].as_str().ok_or_else(|| anyhow!("missing data b64"))?;
        Ok(Some(base64::engine::general_purpose::STANDARD.decode(b64)?))
    }
}

async fn wait_confirmed(rpc: &Rpc, sig: &Signature) -> Result<()> {
    for _ in 0..40 {
        tokio::time::sleep(Duration::from_millis(500)).await;
        let r = rpc.call("getSignatureStatuses", json!([[sig.to_string()]])).await?;
        if let Some(s) = r["value"].get(0) {
            if !s.is_null() {
                if let Some(err) = s.get("err") {
                    if !err.is_null() {
                        return Err(anyhow!("tx failed: {}", err));
                    }
                }
                if let Some(c) = s.get("confirmationStatus").and_then(|v| v.as_str()) {
                    if c == "confirmed" || c == "finalized" { return Ok(()); }
                }
            }
        }
    }
    Err(anyhow!("confirmation timeout"))
}

fn load_keypair(path: &std::path::Path) -> Result<Keypair> {
    let data = std::fs::read_to_string(path)?;
    let bytes: Vec<u8> = serde_json::from_str(&data)?;
    Keypair::from_bytes(&bytes).map_err(|e| anyhow!("invalid keypair: {}", e))
}
