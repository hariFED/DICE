use anchor_lang::prelude::*;

use crate::constants::{ROTATION_COOLDOWN_SLOTS, SEED_NODE_VAULT};
use crate::error::DiceError;
use crate::state::{NodeVault, NodeVaultStatus};

/// Rotate a `NodeVault`'s payout wallet.
///
/// This instruction replaces the vault's current `payout_wallet` with a new
/// one. It is deliberately harder to invoke than `register_node_vault`:
///
///   1. The current payout wallet must sign (proves the attacker isn't
///      someone who stole the device without also having the wallet key).
///   2. The device must sign a fresh binding message over the new wallet
///      (proves the attacker isn't someone who phished the wallet without
///      also having physical access to press factory-reset).
///   3. The previous binding's slot + `ROTATION_COOLDOWN_SLOTS` must have
///      elapsed (prevents rapid-fire rotation abuse).
///
/// Both signatures are verified independently:
///   - The device signature uses the same `verify_binding_signature` helper
///     as first-time registration.
///   - The current payout wallet signature is enforced structurally — the
///     instruction requires it as an `AccountMeta { is_signer: true }` and
///     constrains its key to equal `vault.payout_wallet`.
#[derive(Accounts)]
pub struct RotatePayoutWallet<'info> {
    /// Current payout wallet. Must sign the TX and must match
    /// `vault.payout_wallet`.
    #[account(
        constraint = current_payout_wallet.key() == vault.payout_wallet
            @ DiceError::VaultWrongPayoutWallet
    )]
    pub current_payout_wallet: Signer<'info>,

    #[account(
        mut,
        seeds = [SEED_NODE_VAULT, &crate::constants::device_id(&vault.device_pubkey)],
        bump,
    )]
    pub vault: Account<'info, NodeVault>,
}

pub fn handler(
    ctx: Context<RotatePayoutWallet>,
    new_payout_wallet: Pubkey,
    timestamp: i64,
    nonce: [u8; 32],
    device_signature: [u8; 64],
) -> Result<()> {
    let clock = Clock::get()?;
    let vault = &mut ctx.accounts.vault;

    // Vault must be in a rotatable state.
    require!(
        vault.status == NodeVaultStatus::Bound,
        DiceError::VaultNotBound
    );

    // Cooldown check — prevents rapid churn.
    let earliest_allowed = vault
        .binding_slot
        .saturating_add(ROTATION_COOLDOWN_SLOTS);
    require!(
        clock.slot >= earliest_allowed,
        DiceError::VaultRotationCooldown
    );

    // Verify the device signed over the NEW wallet. Reuses the exact same
    // signature format as first-time registration.
    crate::instructions::register_node_vault::verify_binding_signature(
        &vault.device_pubkey,
        &new_payout_wallet,
        timestamp,
        &nonce,
        &device_signature,
    )?;

    // Apply the rotation.
    let prev = vault.payout_wallet;
    vault.payout_wallet = new_payout_wallet;
    vault.binding_slot = clock.slot;
    vault.binding_signature = device_signature;
    vault.binding_nonce = nonce;
    vault.binding_timestamp = timestamp;

    msg!(
        "Vault rotated: device={:?}, prev={}, new={}, slot={}",
        &vault.device_pubkey[..4],
        prev,
        new_payout_wallet,
        clock.slot
    );
    Ok(())
}
