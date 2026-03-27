// Off-chain DICE client — gated by the `client` feature flag.
// Agent: SDK Agent

use std::sync::Arc;
use std::time::{Duration, Instant};

use solana_sdk::pubkey::Pubkey;

use crate::{cpi, pda, types};
use crate::error::DiceVrfError;

/// Async off-chain client for interacting with the DICE oracle network.
///
/// Wraps a `solana_client` RPC connection and exposes high-level helpers for
/// polling randomness results and querying request status.
///
/// # Example
///
/// ```ignore
/// use dice_vrf::DiceVrfClient;
/// use std::str::FromStr;
/// use std::time::Duration;
///
/// #[tokio::main]
/// async fn main() {
///     let program_id = dice_vrf::DICE_PROGRAM_ID.parse().unwrap();
///     let client = DiceVrfClient::new("https://api.devnet.solana.com", program_id);
///
///     let requester = solana_sdk::pubkey::Pubkey::new_unique();
///     let randomness = client
///         .await_result(&requester, 1, Duration::from_secs(60), Duration::from_secs(2))
///         .await
///         .unwrap();
///
///     println!("Randomness: {}", hex::encode(randomness));
/// }
/// ```
pub struct DiceVrfClient {
    rpc_client: Arc<solana_client::nonblocking::rpc_client::RpcClient>,
    /// The deployed DICE program ID to query against.
    program_id: Pubkey,
}

impl DiceVrfClient {
    /// Create a new [`DiceVrfClient`] connected to the given `rpc_url`.
    ///
    /// # Arguments
    /// * `rpc_url`    — Solana RPC endpoint, e.g.
    ///                  `"https://api.devnet.solana.com"`.
    /// * `program_id` — Deployed DICE program ID. Use
    ///                  `dice_vrf::DICE_PROGRAM_ID.parse().unwrap()` for the
    ///                  canonical deployment.
    pub fn new(rpc_url: &str, program_id: Pubkey) -> Self {
        Self {
            rpc_client: Arc::new(
                solana_client::nonblocking::rpc_client::RpcClient::new(rpc_url.to_string()),
            ),
            program_id,
        }
    }

    /// Wait for a randomness request to be finalized, returning the 32-byte
    /// output when it is ready.
    ///
    /// Polls the `RandomnessResult` PDA every `poll_interval` until a
    /// non-zero value is present or `timeout` elapses.
    ///
    /// # Errors
    /// * [`DiceVrfError::Timeout`] — result not available within `timeout`.
    /// * [`DiceVrfError::RpcError`] — unrecoverable RPC failure (transient
    ///   failures such as "account not found" are silently retried).
    ///
    /// # Example
    ///
    /// ```ignore
    /// let randomness = client
    ///     .await_result(&requester, sequence, Duration::from_secs(60), Duration::from_secs(2))
    ///     .await?;
    /// ```
    pub async fn await_result(
        &self,
        requester: &Pubkey,
        sequence: u64,
        timeout: Duration,
        poll_interval: Duration,
    ) -> Result<[u8; 32], DiceVrfError> {
        let deadline = Instant::now() + timeout;
        let (result_pda, _) = pda::randomness_result_pda(requester, sequence, &self.program_id);

        loop {
            if Instant::now() > deadline {
                return Err(DiceVrfError::Timeout);
            }

            match self.rpc_client.get_account(&result_pda).await {
                Ok(account) => {
                    if let Some(randomness) = cpi::decode_randomness_result(&account.data) {
                        return Ok(randomness);
                    }
                    // Account exists but result not yet finalized — keep polling
                }
                Err(_) => {
                    // Account doesn't exist yet — keep polling
                }
            }

            tokio::time::sleep(poll_interval).await;
        }
    }

    /// Fetch the current status of a randomness request.
    ///
    /// Returns `Ok(None)` if the `RandomnessRequest` PDA has not been created
    /// yet (i.e. `request_randomness` has not been called).
    ///
    /// # Errors
    /// * [`DiceVrfError::RpcError`] — RPC call failed.
    pub async fn get_request_status(
        &self,
        requester: &Pubkey,
        sequence: u64,
    ) -> Result<Option<types::RandomnessRequestInfo>, DiceVrfError> {
        let (request_pda, _) =
            pda::randomness_request_pda(requester, sequence, &self.program_id);

        let account = match self.rpc_client.get_account(&request_pda).await {
            Ok(a) => a,
            Err(e) => {
                // If the error indicates "account not found" return None
                let msg = e.to_string();
                if msg.contains("AccountNotFound")
                    || msg.contains("could not find account")
                    || msg.contains("Invalid param")
                {
                    return Ok(None);
                }
                return Err(DiceVrfError::RpcError(msg));
            }
        };

        let info = deserialize_request_account(&account.data, request_pda, *requester, sequence)?;
        Ok(Some(info))
    }
}

// ── On-chain account deserializer ────────────────────────────────────────────

/// Parse a raw `RandomnessRequest` account buffer into a
/// [`types::RandomnessRequestInfo`].
///
/// Layout (after the 8-byte Anchor discriminator):
/// ```text
/// requester:      32 bytes  (Pubkey)
/// sequence:        8 bytes  (u64 LE)
/// status:          1 byte   (enum tag 0-4)
/// randomness:     33 bytes  (1-byte Option tag + 32-byte value)
/// created_slot:    8 bytes  (u64 LE)
/// finalized_slot: 9 bytes   (1-byte Option tag + 8-byte u64 LE)
/// ```
///
/// Total minimum: 8 + 32 + 8 + 1 + 33 + 8 + 9 = 99 bytes.
fn deserialize_request_account(
    data: &[u8],
    request_pda: Pubkey,
    requester: Pubkey,
    sequence: u64,
) -> Result<types::RandomnessRequestInfo, DiceVrfError> {
    const MIN_LEN: usize = 99;
    if data.len() < MIN_LEN {
        return Err(DiceVrfError::ProgramError(format!(
            "RandomnessRequest account data too short: {} bytes (expected >= {})",
            data.len(),
            MIN_LEN
        )));
    }

    let mut cursor = 8usize; // skip 8-byte discriminator

    // requester (32 bytes) — we already have it, skip
    cursor += 32;

    // sequence (8 bytes LE)
    let _seq = u64::from_le_bytes(data[cursor..cursor + 8].try_into().unwrap());
    cursor += 8;

    // status (1 byte enum tag)
    let status_tag = data[cursor];
    cursor += 1;
    let status = match status_tag {
        0 => types::RequestStatus::Pending,
        1 => types::RequestStatus::CommitPhase,
        2 => types::RequestStatus::RevealPhase,
        3 => types::RequestStatus::Finalized,
        4 => types::RequestStatus::Failed,
        t => {
            return Err(DiceVrfError::ProgramError(format!(
                "Unknown RequestStatus tag: {t}"
            )))
        }
    };

    // randomness: Option<[u8;32]>  — 1-byte tag + 32-byte value
    let randomness_tag = data[cursor];
    cursor += 1;
    let randomness: Option<[u8; 32]> = if randomness_tag == 1 {
        let bytes: [u8; 32] = data[cursor..cursor + 32].try_into().unwrap();
        cursor += 32;
        Some(bytes)
    } else {
        cursor += 32;
        None
    };

    // created_slot (8 bytes LE)
    let created_slot = u64::from_le_bytes(data[cursor..cursor + 8].try_into().unwrap());
    cursor += 8;

    // finalized_slot: Option<u64> — 1-byte tag + 8-byte value
    let finalized_tag = data[cursor];
    cursor += 1;
    let finalized_slot: Option<u64> = if finalized_tag == 1 {
        Some(u64::from_le_bytes(
            data[cursor..cursor + 8].try_into().unwrap(),
        ))
    } else {
        None
    };

    Ok(types::RandomnessRequestInfo {
        request_id: request_pda,
        requester,
        sequence,
        status,
        randomness,
        created_slot,
        finalized_slot,
    })
}
