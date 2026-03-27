use anchor_lang::prelude::*;

use crate::constants::SEED_DEVICE;
use crate::error::DiceError;
use crate::state::DeviceRegistry;

#[derive(Accounts)]
#[instruction(device_id: [u8; 32], device_pubkey: [u8; 33])]
pub struct RegisterDevice<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        init,
        payer = payer,
        space = DeviceRegistry::LEN,
        seeds = [SEED_DEVICE, &device_id],
        bump,
    )]
    pub device_registry: Account<'info, DeviceRegistry>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<RegisterDevice>, device_id: [u8; 32], device_pubkey: [u8; 33]) -> Result<()> {
    require!(device_id == crate::constants::device_id(&device_pubkey), DiceError::InvalidDeviceId);

    let clock = Clock::get()?;
    let registry = &mut ctx.accounts.device_registry;

    registry.device_pubkey = device_pubkey;
    registry.registered_at = clock.unix_timestamp;
    registry.jobs_completed = 0;
    registry.is_active = true;

    msg!(
        "Device registered: {:?} at slot {}",
        &device_pubkey[..4],
        clock.slot
    );
    Ok(())
}
