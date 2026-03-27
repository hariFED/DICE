//! Example: request randomness and poll for a finalized result.
//!
//! This example shows the minimal off-chain integration pattern:
//!
//! 1. Build a `DiceVrfClient` pointing at devnet.
//! 2. Call `await_result` — it polls the `RandomnessResult` PDA until the
//!    commit-reveal protocol completes or the timeout elapses.
//! 3. Print the hex-encoded 32-byte randomness value.
//!
//! Run:
//! ```bash
//! cargo run --example request_and_poll --features client
//! ```
//!
//! Note: In a real integration you would first submit a `request_randomness`
//! transaction (funding the escrow PDA with 0.002 SOL) and record the
//! sequence number before calling `await_result`.

use dice_vrf::DiceVrfClient;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use std::time::Duration;

#[tokio::main]
async fn main() {
    let program_id = Pubkey::from_str(dice_vrf::DICE_PROGRAM_ID)
        .expect("DICE_PROGRAM_ID is a valid base58 pubkey");

    let client = DiceVrfClient::new("https://api.devnet.solana.com", program_id);

    // In production these values come from your application state — the
    // requester is the wallet that signed `request_randomness`, and the
    // sequence is the counter value passed to that instruction.
    let requester = Pubkey::new_unique();
    let sequence: u64 = 1;

    println!(
        "Polling RandomnessResult PDA for requester={} sequence={}",
        requester, sequence
    );
    println!("Waiting up to 60 s (polling every 2 s)...");

    match client
        .await_result(
            &requester,
            sequence,
            Duration::from_secs(60),
            Duration::from_secs(2),
        )
        .await
    {
        Ok(randomness) => println!("Randomness: {}", hex::encode(randomness)),
        Err(e) => eprintln!("Error: {e}"),
    }
}
