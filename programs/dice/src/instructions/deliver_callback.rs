use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction as SolInstruction};
use anchor_lang::solana_program::program::invoke;

use crate::constants::SEED_CHANNEL;
use crate::error::DiceError;
use crate::state::{ChannelStatus, DiceChannel};

/// SHA-256("global:dice_callback")[0..8]
const DICE_CALLBACK_DISCRIMINATOR: [u8; 8] = [128, 131, 129, 45, 53, 113, 215, 151];

#[derive(Accounts)]
pub struct DeliverCallback<'info> {
    /// The protocol coordinator.
    pub coordinator: Signer<'info>,

    #[account(
        mut,
        seeds = [SEED_CHANNEL, channel.authority.as_ref(), &channel.channel_index.to_le_bytes()],
        bump,
    )]
    pub channel: Account<'info, DiceChannel>,

    // remaining_accounts:
    //   [0]    = callback_program
    //   [1..]  = additional accounts forwarded to callback
}

pub fn handler<'info>(
    ctx: Context<'_, '_, 'info, 'info, DeliverCallback<'info>>,
    round_id: u64,
) -> Result<()> {
    let channel = &mut ctx.accounts.channel;

    // Validate coordinator is authorized for this channel
    require!(
        ctx.accounts.coordinator.key() == channel.coordinator,
        DiceError::UnauthorizedCoordinator
    );

    // Validate round_id
    require!(channel.round_id == round_id, DiceError::RoundAlreadyFinalized);

    // Must be finalized
    require!(
        channel.status == ChannelStatus::Finalized,
        DiceError::RoundNotComplete
    );

    let callback_pid = channel.callback_program_id;
    let channel_key = channel.key();
    let randomness = channel.randomness;

    // If no callback program set, just transition to Idle
    if callback_pid == Pubkey::default() {
        channel.status = ChannelStatus::Idle;
        msg!("No callback configured — channel set to Idle");
        return Ok(());
    }

    // REENTRANCY FIX: Set status to Idle BEFORE the CPI call.
    // This prevents the callback program from calling back into DICE
    // while the channel is still in Finalized state.
    channel.status = ChannelStatus::Idle;

    // Verify callback program is in remaining_accounts
    let remaining = ctx.remaining_accounts;
    require!(!remaining.is_empty(), DiceError::CallbackProgramMissing);

    let callback_program = &remaining[0];
    require!(
        callback_program.key() == callback_pid,
        DiceError::CallbackProgramMismatch
    );

    // Validate callback program is executable
    require!(callback_program.executable, DiceError::CallbackProgramMissing);

    // Build callback instruction data:
    //   [0..8]   discriminator
    //   [8..40]  channel_key (Pubkey)
    //   [40..72] randomness ([u8; 32])
    let mut callback_data = Vec::with_capacity(72);
    callback_data.extend_from_slice(&DICE_CALLBACK_DISCRIMINATOR);
    callback_data.extend_from_slice(channel_key.as_ref());
    callback_data.extend_from_slice(&randomness);

    // Build account metas — forward remaining_accounts[1..] to callback
    let mut account_metas = Vec::new();
    for acc in &remaining[1..] {
        if acc.is_writable {
            account_metas.push(AccountMeta::new(*acc.key, acc.is_signer));
        } else {
            account_metas.push(AccountMeta::new_readonly(*acc.key, acc.is_signer));
        }
    }

    let ix = SolInstruction {
        program_id: callback_pid,
        accounts: account_metas,
        data: callback_data,
    };

    // Gather account infos for CPI
    let mut account_infos: Vec<AccountInfo<'info>> = Vec::new();
    for acc in &remaining[1..] {
        account_infos.push(acc.clone());
    }
    account_infos.push(callback_program.clone());

    // If CPI fails, revert status back to Finalized so it can be retried
    match invoke(&ix, &account_infos) {
        Ok(()) => {
            msg!("Callback delivered, channel is Idle");
            Ok(())
        }
        Err(_) => {
            // Revert status so callback can be retried
            let channel = &mut ctx.accounts.channel;
            channel.status = ChannelStatus::Finalized;
            err!(DiceError::CallbackFailed)
        }
    }
}
