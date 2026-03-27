// dice-vrf SDK
// Agent: SDK Agent

pub mod accounts;
pub mod cpi;
pub mod error;
pub mod pda;
pub mod types;

#[cfg(feature = "client")]
pub mod client;

// ── Public re-exports ────────────────────────────────────────────────────────

pub use accounts::DiceVrfAccounts;
pub use types::{DiceConfig, DiceVrfError, PaymentModel, RandomnessRequestInfo, RequestStatus};
pub use pda::*;

#[cfg(feature = "client")]
pub use client::DiceVrfClient;

// ── Constants ────────────────────────────────────────────────────────────────

/// The canonical DICE program ID on mainnet (and devnet).
pub const DICE_PROGRAM_ID: &str = "78Qv6cyKkRZN2YngiLSSBCe2iyRc6jgtCs3incCaMRcv";
