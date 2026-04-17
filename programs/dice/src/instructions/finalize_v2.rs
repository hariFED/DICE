use anchor_lang::prelude::*;
use solana_program::hash::hashv;

use crate::constants::{MIN_NODES_REQUIRED, SEED_CHANNEL};
use crate::error::DiceError;
use crate::state::{ChannelStatus, DiceChannel};

#[derive(Accounts)]
pub struct FinalizeV2<'info> {
    /// The protocol coordinator.
    pub coordinator: Signer<'info>,

    #[account(
        mut,
        seeds = [SEED_CHANNEL, channel.authority.as_ref(), &channel.channel_index.to_le_bytes()],
        bump,
    )]
    pub channel: Account<'info, DiceChannel>,
}

pub fn handler(ctx: Context<FinalizeV2>, round_id: u64) -> Result<()> {
    let _clock = Clock::get()?;
    let channel = &mut ctx.accounts.channel;

    // Validate coordinator is authorized for this channel
    require!(
        ctx.accounts.coordinator.key() == channel.coordinator,
        DiceError::UnauthorizedCoordinator
    );

    // Validate round_id
    require!(channel.round_id == round_id, DiceError::RoundAlreadyFinalized);

    // Must be in RevealPhase (all commits received, reveals in progress)
    require!(
        channel.status == ChannelStatus::RevealPhase,
        DiceError::RoundAlreadyFinalized
    );

    // Need minimum reveals
    require!(
        channel.reveals_received >= MIN_NODES_REQUIRED,
        DiceError::InsufficientNodes
    );

    // Collect entropy from revealed nodes (copy to owned values to avoid borrow issues)
    let mut entropy_values: Vec<[u8; 32]> = Vec::with_capacity(channel.reveals_received as usize);
    for i in 0..channel.commits_received as usize {
        if channel.entropies[i] != [0u8; 32] {
            entropy_values.push(channel.entropies[i]);
        }
    }

    require!(
        entropy_values.len() >= MIN_NODES_REQUIRED as usize,
        DiceError::InsufficientNodes
    );

    // Final randomness = SHA-256(entropy1 || entropy2 || ... || entropyN)
    let refs: Vec<&[u8]> = entropy_values.iter().map(|e| e.as_ref()).collect();
    let final_hash = hashv(&refs);
    channel.randomness = final_hash.to_bytes();

    // L4 latency optimization (v7.4+):
    //
    // For channels with NO callback configured (`callback_program_id ==
    // Pubkey::default()`), skip the Finalized → Idle hand-off via a
    // separate `deliver_callback` TX and transition straight to Idle here.
    // This eliminates an entire round-trip TX for the streaming-VRF
    // pattern and the stress driver, saving ~1.5 s per round.
    //
    // Channels WITH a real callback program still go to Finalized — they
    // need `deliver_callback` to fire the dApp CPI before transitioning.
    let auto_idle = channel.callback_program_id == Pubkey::default();
    if auto_idle {
        channel.status = ChannelStatus::Idle;
        msg!(
            "Randomness finalized + auto-Idle (no callback): round_id={}, reveals={}, randomness={:?}",
            round_id,
            entropy_values.len(),
            &channel.randomness[..8]
        );
    } else {
        channel.status = ChannelStatus::Finalized;
        msg!(
            "Randomness finalized: round_id={}, reveals={}, randomness={:?}",
            round_id,
            entropy_values.len(),
            &channel.randomness[..8]
        );
    }
    Ok(())
}
