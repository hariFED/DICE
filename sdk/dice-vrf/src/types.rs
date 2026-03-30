// Shared types mirroring the on-chain program's account structs.
// Agent: SDK Agent — keep in sync with programs/dice/src/state/

use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

pub use crate::error::DiceVrfError;

/// The lifecycle status of a randomness request on-chain.
///
/// Mirrors `programs/dice/src/state/randomness_request.rs::RequestStatus`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RequestStatus {
    /// Request account created; waiting for commit phase to begin.
    Pending,
    /// Hardware nodes are posting commitments.
    CommitPhase,
    /// All commitments received; nodes are revealing entropy.
    RevealPhase,
    /// All reveals verified and randomness written to `RandomnessResult`.
    Finalized,
    /// The protocol failed (e.g. insufficient nodes, timeout).
    Failed,
}

/// A view of a `RandomnessRequest` account fetched from the chain.
///
/// Returned by [`crate::client::DiceVrfClient::get_request_status`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RandomnessRequestInfo {
    /// The PDA address of this request.
    pub request_id: Pubkey,
    /// The wallet that called `request_randomness`.
    pub requester: Pubkey,
    /// Monotonically increasing counter scoped to the requester (starts at 1).
    pub sequence: u64,
    /// Current lifecycle status.
    pub status: RequestStatus,
    /// The 32-byte randomness output. `Some` only when `status == Finalized`.
    pub randomness: Option<[u8; 32]>,
    /// Slot at which the request account was created.
    pub created_slot: u64,
    /// Slot at which the result was finalized, if applicable.
    pub finalized_slot: Option<u64>,
}

/// Configuration for a DICE integration.
///
/// Create one of the preset configs with [`DiceConfig::mainnet`],
/// [`DiceConfig::devnet`], or [`DiceConfig::localnet`], then pass it when
/// constructing a [`crate::accounts::DiceVrfAccounts`] or
/// [`crate::client::DiceVrfClient`].
#[derive(Debug, Clone)]
pub struct DiceConfig {
    /// The deployed DICE program ID.
    pub program_id: Pubkey,
    /// Treasury pubkey that receives 20 % of each request fee.
    pub treasury: Pubkey,
    /// Reserve pubkey that receives 10 % of each request fee.
    pub reserve: Pubkey,
    /// Who pays the 0.002 SOL per-request fee.
    pub payment_model: PaymentModel,
}

/// The lifecycle status of a DiceChannel (v2.0).
///
/// Mirrors `programs/dice/src/state/dice_channel.rs::ChannelStatus`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ChannelStatus {
    /// Ready for a new request.
    Idle,
    /// Request submitted, waiting for commits.
    Pending,
    /// At least one commit received.
    CommitPhase,
    /// All commits received, waiting for reveals.
    RevealPhase,
    /// Randomness computed and written. Awaiting callback delivery.
    Finalized,
    /// Round failed (timeout or insufficient nodes).
    Failed,
}

/// A view of a `DiceChannel` account fetched from the chain (v2.0).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiceChannelInfo {
    /// The PDA address of this channel.
    pub channel_address: Pubkey,
    /// Channel owner.
    pub authority: Pubkey,
    /// Index for multiple channels per developer.
    pub channel_index: u16,
    /// Maximum nodes this channel supports.
    pub max_nodes: u8,
    /// Current lifecycle status.
    pub status: ChannelStatus,
    /// Current round ID.
    pub round_id: u64,
    /// Prepaid fee balance in lamports.
    pub balance: u64,
    /// The 32-byte randomness from the last finalized round.
    pub randomness: Option<[u8; 32]>,
}

/// Controls who funds the 0.002 SOL per-request fee.
#[derive(Debug, Clone)]
pub enum PaymentModel {
    /// The developer pre-funds an escrow PDA before calling
    /// `request_randomness` (default). The end user's transaction does not
    /// need to carry extra lamports.
    DeveloperPays,
    /// The 0.002 SOL fee is deducted from the end user's wallet at the time
    /// the `request_randomness` instruction is executed.
    UserPays,
}

/// Mainnet treasury and reserve addresses for the DICE program.
const MAINNET_TREASURY: &str = "DiceTreasury1111111111111111111111111111111";
const MAINNET_RESERVE:  &str = "DiceReserve11111111111111111111111111111111";

/// Devnet treasury and reserve addresses (same key material, devnet deploy).
const DEVNET_TREASURY: &str = "DiceTreasury1111111111111111111111111111111";
const DEVNET_RESERVE:  &str = "DiceReserve11111111111111111111111111111111";

impl DiceConfig {
    /// Create a [`DiceConfig`] pointing at the mainnet DICE program.
    ///
    /// Uses the canonical mainnet program ID
    /// `78Qv6cyKkRZN2YngiLSSBCe2iyRc6jgtCs3incCaMRcv`.
    pub fn mainnet() -> Self {
        Self {
            program_id: crate::DICE_PROGRAM_ID.parse().unwrap(),
            treasury: MAINNET_TREASURY.parse().unwrap(),
            reserve: MAINNET_RESERVE.parse().unwrap(),
            payment_model: PaymentModel::DeveloperPays,
        }
    }

    /// Create a [`DiceConfig`] pointing at the devnet DICE program.
    ///
    /// Identical program ID as mainnet — the same binary is deployed to both
    /// networks. Useful for integration testing against real RPC endpoints
    /// without spending real SOL.
    pub fn devnet() -> Self {
        Self {
            program_id: crate::DICE_PROGRAM_ID.parse().unwrap(),
            treasury: DEVNET_TREASURY.parse().unwrap(),
            reserve: DEVNET_RESERVE.parse().unwrap(),
            payment_model: PaymentModel::DeveloperPays,
        }
    }

    /// Create a [`DiceConfig`] suitable for a local `solana-test-validator`
    /// session.
    ///
    /// Uses `Pubkey::default()` for treasury and reserve so that no real
    /// key material is required during tests.
    pub fn localnet() -> Self {
        Self {
            program_id: crate::DICE_PROGRAM_ID.parse().unwrap(),
            treasury: Pubkey::default(),
            reserve: Pubkey::default(),
            payment_model: PaymentModel::DeveloperPays,
        }
    }
}
