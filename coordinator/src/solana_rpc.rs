//! Minimal Solana JSON-RPC client.
//!
//! Uses `reqwest` instead of `solana-client` to avoid the
//! `spl-token-2022 1.0.0 → solana-program =1.17.6` transitive conflict
//! with `solana-sdk =1.18.26`.

use anyhow::{anyhow, Context, Result};
use base64::Engine;
use serde_json::{json, Value};
use solana_sdk::{
    compute_budget::ComputeBudgetInstruction,
    hash::Hash,
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
    transaction::Transaction,
};

/// Priority fee charged per compute unit, in micro-lamports.
/// Pushes our TXs into earlier blocks during devnet contention.
/// At ~600k CU per round this costs 6_000 lamports = 0.000006 SOL.
const PRIORITY_FEE_MICROLAMPORTS_PER_CU: u64 = 10_000;
use std::str::FromStr;
use tracing::{debug, info, warn};

/// Lightweight Solana RPC client backed by `reqwest`.
pub struct SolanaRpc {
    url: String,
    client: reqwest::Client,
}

impl SolanaRpc {
    pub fn new(url: &str) -> Self {
        // Build a reqwest client with explicit timeouts so a hung devnet
        // RPC can never freeze the poller indefinitely. Without these,
        // a getProgramAccounts call to a stalled endpoint will block the
        // poller's tokio task forever — exactly the symptom we hit twice.
        // 8 s connect, 12 s overall request — well above the 800 ms poll
        // tick but bounded.
        let client = reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(8))
            .timeout(std::time::Duration::from_secs(12))
            .build()
            .expect("reqwest client build");
        SolanaRpc {
            url: url.to_string(),
            client,
        }
    }

    /// Send a JSON-RPC request and return the `result` field.
    /// Retries up to 3 times with exponential backoff on transient failures.
    async fn rpc(&self, method: &str, params: Value) -> Result<Value> {
        let body = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params,
        });

        let delays = [500, 1000, 2000]; // ms backoff
        let mut last_err = anyhow!("RPC request failed");

        for (attempt, delay_ms) in std::iter::once(0).chain(delays.iter().copied()).enumerate() {
            if attempt > 0 {
                debug!(attempt, delay_ms, method, "RPC retry after backoff");
                tokio::time::sleep(std::time::Duration::from_millis(delay_ms as u64)).await;
            }

            let resp = match self.client.post(&self.url).json(&body).send().await {
                Ok(r) => r,
                Err(e) => {
                    last_err = anyhow!("RPC request failed: {}", e);
                    continue;
                }
            };

            // Check for rate limiting (429)
            if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                last_err = anyhow!("RPC rate limited (429)");
                continue;
            }

            let json: Value = match resp.json().await {
                Ok(j) => j,
                Err(e) => {
                    last_err = anyhow!("RPC response parse failed: {}", e);
                    continue;
                }
            };

            if let Some(err) = json.get("error") {
                // Don't retry application-level errors (invalid params, etc.)
                return Err(anyhow!("RPC error: {}", err));
            }

            return json.get("result")
                .cloned()
                .ok_or_else(|| anyhow!("RPC response missing 'result'"));
        }

        Err(last_err.context(format!("RPC {} failed after retries", method)))
    }

    // -----------------------------------------------------------------------
    // Core methods
    // -----------------------------------------------------------------------

    /// Fetch the current cluster slot (confirmed commitment).
    pub async fn get_slot(&self) -> Result<u64> {
        let result = self
            .rpc("getSlot", json!([{"commitment": "confirmed"}]))
            .await?;
        result
            .as_u64()
            .ok_or_else(|| anyhow!("missing slot in response"))
    }

    /// Fetch the latest blockhash for transaction signing.
    pub async fn get_latest_blockhash(&self) -> Result<Hash> {
        let result = self
            .rpc("getLatestBlockhash", json!([{"commitment": "finalized"}]))
            .await?;
        let hash_str = result["value"]["blockhash"]
            .as_str()
            .ok_or_else(|| anyhow!("missing blockhash in response"))?;
        Hash::from_str(hash_str).map_err(|e| anyhow!("parse blockhash: {}", e))
    }

    /// Sign, send, and poll `getSignatureStatuses` until the TX is at
    /// `confirmed` commitment (or an error is surfaced). Required for
    /// dependent-TX chains like submit_commit_v2 → submit_reveal_v2 →
    /// finalize_v2 where each TX reads state written by the previous.
    ///
    /// Polls every 300 ms up to ~15 s total. Returns the signature on
    /// success, an error on TX failure or confirmation timeout.
    pub async fn sign_send_and_confirm(
        &self,
        keypair: &Keypair,
        instructions: Vec<solana_sdk::instruction::Instruction>,
    ) -> Result<Signature> {
        let sig = self.sign_and_send(keypair, instructions).await?;
        for _ in 0..50 {
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
            if self.confirm_transaction(&sig).await? {
                return Ok(sig);
            }
        }
        Err(anyhow!(
            "tx {} did not reach confirmed commitment within 15 s",
            sig
        ))
    }

    /// L1 optimization — sign, send, and wait for `processed` commitment
    /// instead of `confirmed`. Use this for INTERMEDIATE TXs in a chain
    /// where you only need the state update to be visible to the NEXT
    /// TX on the same RPC node, not durably landed against a re-org.
    ///
    /// Typical use: commit_v2 TX (A), reveal_v2 TX (B) — A's state
    /// change must be visible when B lands, and both are invoked on
    /// the same leader slot most of the time. The coordinator's
    /// finalize+claim TX (C) still uses `confirmed` commitment via
    /// `sign_send_and_confirm` because we want durability before the
    /// channel transitions to Idle for the next round.
    ///
    /// Polls every 120 ms up to ~6 s. On devnet, `processed` commitment
    /// is typically reached in 200–600 ms, so this usually returns in
    /// 300–700 ms vs 1200–2000 ms for `confirmed`.
    pub async fn sign_send_and_confirm_processed(
        &self,
        keypair: &Keypair,
        instructions: Vec<solana_sdk::instruction::Instruction>,
    ) -> Result<Signature> {
        let sig = self.sign_and_send(keypair, instructions).await?;
        for _ in 0..50 {
            tokio::time::sleep(std::time::Duration::from_millis(120)).await;
            if self.is_signature_processed(&sig).await? {
                return Ok(sig);
            }
        }
        Err(anyhow!(
            "tx {} did not reach processed commitment within 6 s",
            sig
        ))
    }

    /// Check whether a signature has reached `processed` commitment
    /// (i.e. been executed by the current leader) — which is enough
    /// for a dependent TX to read its state, as long as both run on
    /// the same RPC node.
    ///
    /// Returns `Err` if the TX is known to have failed so the caller
    /// can stop the chain early instead of waiting for a timeout.
    pub async fn is_signature_processed(&self, sig: &Signature) -> Result<bool> {
        let result = self
            .rpc(
                "getSignatureStatuses",
                json!([[sig.to_string()], {"searchTransactionHistory": false}]),
            )
            .await?;

        let statuses = result["value"]
            .as_array()
            .ok_or_else(|| anyhow!("missing statuses array"))?;

        if statuses.is_empty() || statuses[0].is_null() {
            return Ok(false);
        }

        let err = &statuses[0]["err"];
        if !err.is_null() {
            warn!(signature = %sig, error = %err, "transaction failed on-chain");
            return Err(anyhow!("tx {} failed: {}", sig, err));
        }

        // At this point the entry exists and has no error, so it has at
        // least reached `processed` commitment (Solana never returns
        // statuses for slots below processed).
        Ok(true)
    }

    /// Sign and send a transaction built from the given instructions.
    pub async fn sign_and_send(
        &self,
        keypair: &Keypair,
        instructions: Vec<solana_sdk::instruction::Instruction>,
    ) -> Result<Signature> {
        let blockhash = self.get_latest_blockhash().await?;

        // Prepend a compute-unit-price instruction so our TXs land in
        // earlier blocks during devnet contention. Negligible cost
        // (~6_000 lamports per VRF round) for a meaningful latency win.
        let mut all_ixs =
            Vec::with_capacity(instructions.len() + 1);
        all_ixs.push(ComputeBudgetInstruction::set_compute_unit_price(
            PRIORITY_FEE_MICROLAMPORTS_PER_CU,
        ));
        all_ixs.extend(instructions);

        let tx = Transaction::new_signed_with_payer(
            &all_ixs,
            Some(&keypair.pubkey()),
            &[keypair],
            blockhash,
        );

        let serialized = bincode::serialize(&tx).context("serialize transaction")?;
        let encoded = base64::engine::general_purpose::STANDARD.encode(&serialized);

        let result = self
            .rpc(
                "sendTransaction",
                json!([encoded, {"encoding": "base64", "skipPreflight": true, "preflightCommitment": "confirmed"}]),
            )
            .await?;

        let sig_str = result
            .as_str()
            .ok_or_else(|| anyhow!("sendTransaction did not return a signature string"))?;

        let sig = Signature::from_str(sig_str)
            .map_err(|e| anyhow!("parse signature: {}", e))?;

        info!(signature = %sig, "transaction sent");
        Ok(sig)
    }

    /// Fetch raw account data for a pubkey. Returns `None` if account doesn't exist.
    pub async fn get_account_data(&self, pubkey: &Pubkey) -> Result<Option<Vec<u8>>> {
        let result = self
            .rpc(
                "getAccountInfo",
                json!([pubkey.to_string(), {"encoding": "base64", "commitment": "confirmed"}]),
            )
            .await?;

        if result["value"].is_null() {
            return Ok(None);
        }

        let data_arr = result["value"]["data"]
            .as_array()
            .ok_or_else(|| anyhow!("missing data array"))?;

        let b64 = data_arr[0]
            .as_str()
            .ok_or_else(|| anyhow!("data[0] not a string"))?;

        let bytes = base64::engine::general_purpose::STANDARD
            .decode(b64)
            .context("base64 decode account data")?;

        Ok(Some(bytes))
    }

    /// Check if a transaction signature has been confirmed.
    ///
    /// Returns:
    /// - `Ok(true)` if the TX is at `confirmed` or `finalized` commitment
    ///   and landed successfully.
    /// - `Ok(false)` if the status entry is still null (TX not yet processed).
    /// - `Err(..)` if the TX is known to have failed on chain. Callers that
    ///   want to distinguish between failure and not-yet-confirmed must
    ///   surface the error instead of retrying blindly.
    pub async fn confirm_transaction(&self, sig: &Signature) -> Result<bool> {
        let result = self
            .rpc(
                "getSignatureStatuses",
                json!([[sig.to_string()], {"searchTransactionHistory": false}]),
            )
            .await?;

        let statuses = result["value"]
            .as_array()
            .ok_or_else(|| anyhow!("missing statuses array"))?;

        if statuses.is_empty() || statuses[0].is_null() {
            return Ok(false);
        }

        let err = &statuses[0]["err"];
        if !err.is_null() {
            warn!(signature = %sig, error = %err, "transaction failed on-chain");
            return Err(anyhow!("tx {} failed: {}", sig, err));
        }

        let confirmation = statuses[0]["confirmationStatus"]
            .as_str()
            .unwrap_or("unknown");

        Ok(confirmation == "confirmed" || confirmation == "finalized")
    }

    /// Get the SOL balance for a pubkey (in lamports).
    pub async fn get_balance(&self, pubkey: &Pubkey) -> Result<u64> {
        let result = self
            .rpc("getBalance", json!([pubkey.to_string()]))
            .await?;
        result["value"]
            .as_u64()
            .ok_or_else(|| anyhow!("missing balance value"))
    }

    /// Fetch all program accounts matching a set of filters.
    /// Returns `Vec<(Pubkey, Vec<u8>)>` of account addresses and their data.
    pub async fn get_program_accounts(
        &self,
        program_id: &Pubkey,
        data_size: Option<usize>,
        memcmp_filters: &[(usize, &[u8])],
    ) -> Result<Vec<(Pubkey, Vec<u8>)>> {
        let mut filters: Vec<Value> = Vec::new();

        if let Some(size) = data_size {
            filters.push(json!({"dataSize": size}));
        }

        for (offset, bytes) in memcmp_filters {
            filters.push(json!({
                "memcmp": {
                    "offset": offset,
                    "bytes": base64::engine::general_purpose::STANDARD.encode(bytes),
                    "encoding": "base64"
                }
            }));
        }

        let result = self
            .rpc(
                "getProgramAccounts",
                json!([
                    program_id.to_string(),
                    {
                        "encoding": "base64",
                        "commitment": "confirmed",
                        "filters": filters,
                    }
                ]),
            )
            .await?;

        let accounts = result
            .as_array()
            .ok_or_else(|| anyhow!("expected array of accounts"))?;

        let mut out = Vec::with_capacity(accounts.len());
        for entry in accounts {
            let pubkey_str = entry["pubkey"]
                .as_str()
                .ok_or_else(|| anyhow!("missing pubkey"))?;
            let pubkey = Pubkey::from_str(pubkey_str)
                .map_err(|e| anyhow!("parse pubkey: {}", e))?;

            let data_arr = entry["account"]["data"]
                .as_array()
                .ok_or_else(|| anyhow!("missing data array"))?;
            let b64 = data_arr[0]
                .as_str()
                .ok_or_else(|| anyhow!("data[0] not a string"))?;
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(b64)
                .context("base64 decode")?;

            out.push((pubkey, bytes));
        }

        debug!(program = %program_id, count = out.len(), "fetched program accounts");
        Ok(out)
    }
}

/// Load a Solana keypair from a JSON file (the standard `solana-keygen` format).
pub fn load_keypair(path: &std::path::Path) -> Result<Keypair> {
    let data = std::fs::read_to_string(path)
        .with_context(|| format!("read keypair file {:?}", path))?;
    let bytes: Vec<u8> = serde_json::from_str(&data)
        .with_context(|| format!("parse keypair JSON {:?}", path))?;
    Keypair::from_bytes(&bytes)
        .map_err(|e| anyhow!("invalid keypair bytes: {}", e))
}
