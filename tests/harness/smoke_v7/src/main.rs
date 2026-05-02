//! DICE v7 devnet smoke test.
//!
//! Exercises `register_node_vault` end-to-end against a devnet deployment of
//! the DICE program. Proves:
//!   - the new instruction discriminator is reachable on-chain
//!   - the SHA-256 binding verification path runs correctly
//!   - secp256k1_recover finds the right device pubkey
//!   - the NodeVault PDA is created and transitions to `Bound`
//!
//! This is a standalone workspace package so it compiles cleanly regardless
//! of the coordinator's build state. All RPC + TX helpers are inlined (a
//! minimal reqwest-based subset of coordinator/src/solana_rpc.rs).
//!
//! Run:
//!   cd C:\Users\Abcom\DICE
//!   cargo run -p smoke-v7 -- --keypair coordinator-keypair.json

use anyhow::{anyhow, Context, Result};
use base64::Engine;
use clap::Parser;
use k256::ecdsa::{signature::hazmat::PrehashSigner, Signature, SigningKey};
use k256::elliptic_curve::sec1::ToEncodedPoint;
use k256::SecretKey;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use solana_sdk::{
    hash::Hash,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signature as SolSig},
    signer::Signer,
    system_program,
    transaction::Transaction,
};
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

// ─── CLI ────────────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(about = "DICE v7 smoke test — register_node_vault on devnet")]
struct Args {
    #[arg(long, default_value = "https://api.devnet.solana.com")]
    rpc_url: String,

    #[arg(long, default_value = "coordinator-keypair.json")]
    keypair: std::path::PathBuf,

    #[arg(long, default_value = "C2JugYQztp1XDGG1ZCagbqRivqGsmE1vG1uMHaMHPDaQ")]
    payout_wallet: String,

    #[arg(long, default_value = "FMwPuCjkfZXN2MuNJQiUzZC3hnxHcD8mrTuntsqA84XD")]
    program_id: String,
}

// ─── Main ───────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new("info,smoke_v7=debug"))
        .init();

    let args = Args::parse();

    let program_id = Pubkey::from_str(&args.program_id).context("invalid program ID")?;
    let payout_wallet =
        Pubkey::from_str(&args.payout_wallet).context("invalid payout wallet")?;

    let payer = load_keypair(&args.keypair).context("load payer keypair")?;
    println!("Payer (upgrade authority): {}", payer.pubkey());

    let rpc = Rpc::new(args.rpc_url.clone());

    let balance = rpc.get_balance(&payer.pubkey()).await?;
    println!(
        "Payer balance: {} lamports ({:.4} SOL)",
        balance,
        balance as f64 / 1e9
    );
    if balance < 5_000_000 {
        println!("⚠️  Balance low — register_node_vault needs ~0.002 SOL for rent + fees");
    }

    // ── 1. Generate a fresh secp256k1 test device keypair ────────────────
    //
    // Uses the current time as the entropy source so every smoke run creates
    // a DIFFERENT vault — re-running is safe, no vault collision.
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let mut sk_bytes = [0u8; 32];
    for i in 0..32 {
        sk_bytes[i] = ((nanos.wrapping_shr((i as u32 % 16) * 4)) & 0xff) as u8;
    }
    // Guarantee a valid scalar. A few bytes flipped around is enough.
    sk_bytes[0] ^= 0x02;
    sk_bytes[31] ^= 0x42;

    let secret = SecretKey::from_slice(&sk_bytes).context("build secp256k1 secret")?;
    let device_point = secret.public_key().to_encoded_point(true);
    let device_pubkey: [u8; 33] = device_point
        .as_bytes()
        .try_into()
        .context("compressed pubkey should be 33 bytes")?;
    println!(
        "Test device pubkey (hex): {}",
        hex::encode(device_pubkey)
    );

    // ── 2. Build binding message hash ────────────────────────────────────
    //
    // Must match programs/dice/src/instructions/register_node_vault.rs
    // `verify_binding_signature` byte-for-byte:
    //   DOMAIN || device_pubkey || payout_wallet || ts_le || nonce
    //   then SHA-256.
    const DOMAIN: &[u8] = b"DICE_PAYOUT_BINDING_V1";
    let timestamp: i64 = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let mut nonce = [0u8; 32];
    for i in 0..32 {
        nonce[i] = ((nanos.wrapping_shr((i as u32 % 8) * 8)) & 0xff) as u8 ^ 0xA5;
    }

    let mut hasher = Sha256::new();
    hasher.update(DOMAIN);
    hasher.update(&device_pubkey);
    hasher.update(payout_wallet.as_ref());
    hasher.update(&timestamp.to_le_bytes());
    hasher.update(&nonce);
    let msg_hash: [u8; 32] = hasher.finalize().into();

    // ── 3. Sign the hash ─────────────────────────────────────────────────
    let signing_key = SigningKey::from(&secret);
    let signature: Signature = signing_key
        .sign_prehash(&msg_hash)
        .context("ecdsa sign")?;
    let sig_bytes: [u8; 64] = signature
        .to_bytes()
        .as_slice()
        .try_into()
        .context("signature should be 64 bytes")?;

    println!("Binding hash:  {}", hex::encode(msg_hash));
    println!("Signature:     {}", hex::encode(sig_bytes));

    // ── 4. Build the register_node_vault instruction ─────────────────────
    //
    // Seeds: ["node_vault", SHA256(device_pubkey)]
    let device_id: [u8; 32] = Sha256::digest(device_pubkey).into();
    let (vault_pda, _bump) =
        Pubkey::find_program_address(&[b"node_vault", &device_id], &program_id);
    println!("Vault PDA:     {}", vault_pda);

    let ix = build_register_node_vault_ix(
        &program_id,
        &payer.pubkey(),
        &vault_pda,
        &device_pubkey,
        &payout_wallet,
        timestamp,
        &nonce,
        &sig_bytes,
    );

    // ── 5. Submit the TX ─────────────────────────────────────────────────
    println!("Submitting register_node_vault TX...");
    let sig = rpc
        .sign_and_send(&payer, vec![ix])
        .await
        .context("send register_node_vault TX")?;
    println!("TX signature: {}", sig);
    println!(
        "Explorer: https://explorer.solana.com/tx/{}?cluster=devnet",
        sig
    );

    // ── 6. Poll for confirmation ─────────────────────────────────────────
    println!("Waiting for confirmation (up to 60s)...");
    let mut confirmed = false;
    for i in 1..=30 {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        match rpc.confirm_transaction(&sig).await {
            Ok(Some(Ok(()))) => {
                println!("✅ TX confirmed after {}s", i * 2);
                confirmed = true;
                break;
            }
            Ok(Some(Err(msg))) => {
                println!("❌ TX failed on-chain: {}", msg);
                std::process::exit(1);
            }
            Ok(None) => {
                // still processing
            }
            Err(e) => {
                eprintln!("confirm poll error: {}", e);
            }
        }
    }
    if !confirmed {
        println!("⚠️  Confirmation timed out — checking vault state anyway");
    }

    // ── 7. Fetch and decode the vault ────────────────────────────────────
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    let vault_data = rpc
        .get_account_data(&vault_pda)
        .await
        .context("fetch vault account")?;

    match vault_data {
        Some(bytes) => {
            println!("Vault exists: {} bytes", bytes.len());
            if bytes.len() < 200 {
                anyhow::bail!("vault data unexpectedly short: {}", bytes.len());
            }
            // Layout (matches programs/dice/src/state/node_vault.rs):
            //   [0..8]    discriminator
            //   [8..41]   device_pubkey  [u8; 33]
            //   [41..73]  payout_wallet  Pubkey (32)
            //   [73..81]  binding_slot   u64
            //   ...
            //   [185]     status         u8 (0=Unbound, 1=Bound, 2=Frozen)
            let on_chain_device = &bytes[8..41];
            let on_chain_wallet_bytes: [u8; 32] =
                bytes[41..73].try_into().context("slice wallet")?;
            let on_chain_wallet = Pubkey::new_from_array(on_chain_wallet_bytes);
            let status = bytes[185];

            println!("  device_pubkey: {}", hex::encode(on_chain_device));
            println!("  payout_wallet: {}", on_chain_wallet);
            println!(
                "  status:        {} ({})",
                status,
                match status {
                    0 => "Unbound",
                    1 => "Bound",
                    2 => "Frozen",
                    _ => "UNKNOWN",
                }
            );

            let ok_device = on_chain_device == device_pubkey;
            let ok_wallet = on_chain_wallet == payout_wallet;
            let ok_status = status == 1;

            println!();
            println!("RESULT");
            println!("  device pubkey match: {}", if ok_device { "✅" } else { "❌" });
            println!("  payout wallet match: {}", if ok_wallet { "✅" } else { "❌" });
            println!("  status == Bound:     {}", if ok_status { "✅" } else { "❌" });

            if ok_device && ok_wallet && ok_status {
                println!();
                println!("🎉 SMOKE TEST PASSED — register_node_vault works end-to-end on devnet");
            } else {
                println!();
                println!("❌ SMOKE TEST FAILED — one or more assertions failed");
                std::process::exit(1);
            }
        }
        None => {
            anyhow::bail!("vault account not found at {} — did the TX actually land?", vault_pda);
        }
    }

    Ok(())
}

// ─── Instruction builder ────────────────────────────────────────────────────

/// Compute the Anchor global instruction discriminator at runtime.
fn anchor_discriminator(name: &str) -> [u8; 8] {
    let full = format!("global:{name}");
    let hash = Sha256::digest(full.as_bytes());
    let mut out = [0u8; 8];
    out.copy_from_slice(&hash[..8]);
    out
}

/// Build the `register_node_vault` instruction.
/// Layout MUST match programs/dice/src/instructions/register_node_vault.rs:
///   data = discriminator(8) + device_pubkey(33) + payout_wallet(32)
///        + timestamp_le(8) + nonce(32) + signature(64) = 177 bytes
///   accounts = [payer(signer,mut), vault(mut), system_program]
fn build_register_node_vault_ix(
    program_id: &Pubkey,
    payer: &Pubkey,
    vault: &Pubkey,
    device_pubkey: &[u8; 33],
    payout_wallet: &Pubkey,
    timestamp: i64,
    nonce: &[u8; 32],
    signature: &[u8; 64],
) -> Instruction {
    let mut data = Vec::with_capacity(177);
    data.extend_from_slice(&anchor_discriminator("register_node_vault"));
    data.extend_from_slice(device_pubkey);
    data.extend_from_slice(payout_wallet.as_ref());
    data.extend_from_slice(&timestamp.to_le_bytes());
    data.extend_from_slice(nonce);
    data.extend_from_slice(signature);

    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*payer, true),
            AccountMeta::new(*vault, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data,
    }
}

// ─── Minimal reqwest-based Solana RPC client ────────────────────────────────

struct Rpc {
    url: String,
    client: reqwest::Client,
}

impl Rpc {
    fn new(url: String) -> Self {
        Rpc {
            url,
            client: reqwest::Client::new(),
        }
    }

    async fn call(&self, method: &str, params: Value) -> Result<Value> {
        let body = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params,
        });
        let resp = self
            .client
            .post(&self.url)
            .json(&body)
            .send()
            .await
            .context("rpc send")?;
        let json: Value = resp.json().await.context("rpc decode")?;
        if let Some(err) = json.get("error") {
            return Err(anyhow!("RPC error: {}", err));
        }
        json.get("result")
            .cloned()
            .ok_or_else(|| anyhow!("missing result"))
    }

    async fn get_balance(&self, pk: &Pubkey) -> Result<u64> {
        let r = self.call("getBalance", json!([pk.to_string()])).await?;
        r["value"]
            .as_u64()
            .ok_or_else(|| anyhow!("missing balance"))
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

    async fn sign_and_send(
        &self,
        payer: &Keypair,
        ixs: Vec<Instruction>,
    ) -> Result<SolSig> {
        let bh = self.get_latest_blockhash().await?;
        let tx = Transaction::new_signed_with_payer(&ixs, Some(&payer.pubkey()), &[payer], bh);
        let bytes = bincode::serialize(&tx).context("serialize tx")?;
        let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);

        let r = self
            .call(
                "sendTransaction",
                json!([encoded, {"encoding": "base64", "skipPreflight": false, "preflightCommitment": "confirmed"}]),
            )
            .await?;
        let s = r
            .as_str()
            .ok_or_else(|| anyhow!("sendTransaction result not a string"))?;
        SolSig::from_str(s).map_err(|e| anyhow!("parse signature: {}", e))
    }

    /// Returns:
    ///   Ok(None)         — still processing
    ///   Ok(Some(Ok(()))) — confirmed success
    ///   Ok(Some(Err(msg))) — confirmed but failed on-chain
    async fn confirm_transaction(&self, sig: &SolSig) -> Result<Option<Result<(), String>>> {
        let r = self
            .call(
                "getSignatureStatuses",
                json!([[sig.to_string()], {"searchTransactionHistory": false}]),
            )
            .await?;
        let arr = r["value"]
            .as_array()
            .ok_or_else(|| anyhow!("missing status array"))?;
        if arr.is_empty() || arr[0].is_null() {
            return Ok(None);
        }
        let entry = &arr[0];
        let conf = entry["confirmationStatus"].as_str().unwrap_or("");
        if conf != "confirmed" && conf != "finalized" {
            return Ok(None);
        }
        let err = &entry["err"];
        if !err.is_null() {
            return Ok(Some(Err(format!("{}", err))));
        }
        Ok(Some(Ok(())))
    }

    async fn get_account_data(&self, pk: &Pubkey) -> Result<Option<Vec<u8>>> {
        let r = self
            .call(
                "getAccountInfo",
                json!([pk.to_string(), {"encoding": "base64", "commitment": "confirmed"}]),
            )
            .await?;
        if r["value"].is_null() {
            return Ok(None);
        }
        let arr = r["value"]["data"]
            .as_array()
            .ok_or_else(|| anyhow!("missing data array"))?;
        let b64 = arr[0]
            .as_str()
            .ok_or_else(|| anyhow!("data[0] not a string"))?;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(b64)
            .context("base64 decode")?;
        Ok(Some(bytes))
    }
}

fn load_keypair(path: &std::path::Path) -> Result<Keypair> {
    let data = std::fs::read_to_string(path)
        .with_context(|| format!("read keypair file {:?}", path))?;
    let bytes: Vec<u8> = serde_json::from_str(&data)
        .with_context(|| format!("parse keypair JSON {:?}", path))?;
    Keypair::from_bytes(&bytes).map_err(|e| anyhow!("invalid keypair bytes: {}", e))
}
