//! Minimal Solana JSON-RPC client.
//!
//! Uses `reqwest` instead of `solana-client` to avoid the
//! `spl-token-2022 1.0.0 → solana-program =1.17.6` transitive conflict
//! with `solana-sdk =1.18.26`.

use anyhow::{anyhow, Context, Result};
use base64::Engine;
use serde_json::{json, Value};
use solana_sdk::{
    hash::Hash,
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
    transaction::Transaction,
};
use std::str::FromStr;
use tracing::{debug, info, warn};

/// Lightweight Solana RPC client backed by `reqwest`.
pub struct SolanaRpc {
    url: String,
    client: reqwest::Client,
}

impl SolanaRpc {
    pub fn new(url: &str) -> Self {
        SolanaRpc {
            url: url.to_string(),
            client: reqwest::Client::new(),
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

    /// Sign and send a transaction built from the given instructions.
    pub async fn sign_and_send(
        &self,
        keypair: &Keypair,
        instructions: Vec<solana_sdk::instruction::Instruction>,
    ) -> Result<Signature> {
        let blockhash = self.get_latest_blockhash().await?;
        let tx = Transaction::new_signed_with_payer(
            &instructions,
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
            warn!(error = %err, "transaction failed on-chain");
            return Ok(false);
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
