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
pub use types::{ChannelStatus, DiceChannelInfo, DiceConfig, DiceVrfError, PaymentModel, RandomnessRequestInfo, RequestStatus};
pub use pda::*;

#[cfg(feature = "client")]
pub use client::DiceVrfClient;

// ── Constants ────────────────────────────────────────────────────────────────

/// The canonical DICE program ID on mainnet (and devnet), as a base58 string.
///
/// Prefer [`DICE_PROGRAM_ID_PUBKEY`] when you need a `Pubkey` — that avoids
/// the `.parse().unwrap()` pattern that shows up at every call site and
/// eliminates a potential runtime panic if the constant ever drifts.
pub const DICE_PROGRAM_ID: &str = "FMwPuCjkfZXN2MuNJQiUzZC3hnxHcD8mrTuntsqA84XD";

/// The canonical DICE program ID as a compile-time `Pubkey`.
///
/// Declared via `solana_sdk::pubkey!` so the base58 string is parsed at
/// compile time — no runtime panic, no `.parse().unwrap()` at call sites.
/// Keep in lockstep with [`DICE_PROGRAM_ID`]; the unit test below enforces
/// that they resolve to the same value.
pub const DICE_PROGRAM_ID_PUBKEY: solana_sdk::pubkey::Pubkey =
    solana_sdk::pubkey!("FMwPuCjkfZXN2MuNJQiUzZC3hnxHcD8mrTuntsqA84XD");

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn dice_program_id_pubkey_matches_string() {
        let parsed = solana_sdk::pubkey::Pubkey::from_str(DICE_PROGRAM_ID)
            .expect("DICE_PROGRAM_ID must be valid base58");
        assert_eq!(parsed, DICE_PROGRAM_ID_PUBKEY);
    }
}
