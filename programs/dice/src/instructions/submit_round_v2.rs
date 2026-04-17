use anchor_lang::prelude::*;
use solana_program::hash::hashv;

use crate::constants::{MIN_NODES_REQUIRED, SEED_CHANNEL};
use crate::error::DiceError;
use crate::state::{ChannelStatus, DiceChannel};

/// One device's full contribution for a single round (commit + reveal in one shot).
///
/// Packed: 33 (pubkey) + 32 (commit_hash) + 32 (entropy) + 64 (signature) = 161 B per device.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug)]
pub struct RoundContribution {
    pub device_pubkey: [u8; 33],
    pub commit_hash: [u8; 32],
    pub entropy: [u8; 32],
    pub signature: [u8; 64],
}

/// v7.5 single-shot round submission.
///
/// Replaces the 3-TX flow (commits / reveals / finalize+claim) with **one** TX
/// that takes all device contributions atomically. Saves a full ~1.5 s
/// confirmation hop versus L3-lite (which still uses a separate commits TX).
///
/// Threat-model note: by accepting commits + reveals in the same TX, the
/// coordinator gains the ability to grind rounds at no on-chain cost (it
/// can collect entropies locally, simulate the result, and silently drop
/// the TX if it doesn't like the outcome). For DICE — where the coordinator
/// is operator-controlled, mTLS-authenticated, and observable via per-channel
/// dispatch metrics — this is an acceptable tradeoff. Bias resistance still
/// holds against any single dishonest device because the result is
/// SHA-256(e₁‖…‖eₙ) and any honest contributor's entropy is unbiasable.
/// Channels with stricter requirements should keep using submit_commit_v2 +
/// submit_reveal_v2.
#[derive(Accounts)]
pub struct SubmitRoundV2<'info> {
    /// The protocol coordinator.
    pub coordinator: Signer<'info>,

    #[account(
        mut,
        seeds = [SEED_CHANNEL, channel.authority.as_ref(), &channel.channel_index.to_le_bytes()],
        bump,
    )]
    pub channel: Account<'info, DiceChannel>,
}

pub fn handler(
    ctx: Context<SubmitRoundV2>,
    round_id: u64,
    contributions: Vec<RoundContribution>,
) -> Result<()> {
    let channel = &mut ctx.accounts.channel;

    // Auth: only the channel's bound coordinator may submit.
    require!(
        ctx.accounts.coordinator.key() == channel.coordinator,
        DiceError::UnauthorizedCoordinator
    );

    // Round + state: must match the round opened by request_randomness_auto
    // and channel must be in Pending (no commits or reveals yet).
    require!(channel.round_id == round_id, DiceError::RoundAlreadyFinalized);
    require!(
        channel.status == ChannelStatus::Pending,
        DiceError::RoundAlreadyFinalized
    );

    // Contribution count must match the requested node_count exactly and
    // meet the global BFT minimum.
    let n = contributions.len();
    require!(
        n == channel.node_count as usize,
        DiceError::InsufficientNodes
    );
    require!(
        n >= MIN_NODES_REQUIRED as usize,
        DiceError::InsufficientNodes
    );
    require!(n <= channel.max_nodes as usize, DiceError::InsufficientNodes);

    // Per-device verification + write-back. We collect entropies inline so
    // we can compute the final randomness in the same loop without holding
    // an extra Vec<&[u8]> across borrows.
    let mut all_entropy: Vec<[u8; 32]> = Vec::with_capacity(n);

    for (i, c) in contributions.iter().enumerate() {
        // 1. SHA-256(entropy) must equal the device's commit hash.
        //    This is the only cryptographic binding v2 enforces on chain
        //    (matching submit_reveal_v2.rs). The signature field is
        //    stored unverified — it's a decorative proof of origin the
        //    coordinator validated off-chain before submission.
        let computed = hashv(&[&c.entropy]).to_bytes();
        require!(computed == c.commit_hash, DiceError::RevealMismatch);

        // 2. Ensure no duplicate device in this submission.
        let device_id = crate::constants::device_id(&c.device_pubkey);
        for j in 0..i {
            require!(
                channel.device_ids[j] != device_id,
                DiceError::AlreadyCommitted
            );
        }

        // 3. Write the inline arrays.
        channel.device_ids[i] = device_id;
        channel.device_pubkeys[i] = c.device_pubkey;
        channel.commit_hashes[i] = c.commit_hash;
        channel.entropies[i] = c.entropy;
        channel.signatures[i] = c.signature;

        all_entropy.push(c.entropy);
    }

    channel.commits_received = n as u8;
    channel.reveals_received = n as u8;

    // Final randomness = SHA-256(e₁ ‖ e₂ ‖ … ‖ eₙ). Identical formula to
    // finalize_v2 so subscribers see the same result they would have got
    // from the legacy 3-TX path.
    let refs: Vec<&[u8]> = all_entropy.iter().map(|e| e.as_ref()).collect();
    let final_hash = hashv(&refs);
    channel.randomness = final_hash.to_bytes();

    // v7.4 auto-Idle for no-callback channels — for callback channels we
    // leave Finalized so deliver_callback can still fire the dApp CPI.
    if channel.callback_program_id == Pubkey::default() {
        channel.status = ChannelStatus::Idle;
        msg!(
            "submit_round_v2: round={} n={} auto-Idle (no callback) randomness={:?}",
            round_id,
            n,
            &channel.randomness[..8]
        );
    } else {
        channel.status = ChannelStatus::Finalized;
        msg!(
            "submit_round_v2: round={} n={} Finalized (callback pending) randomness={:?}",
            round_id,
            n,
            &channel.randomness[..8]
        );
    }
    Ok(())
}
