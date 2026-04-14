use anchor_lang::prelude::*;

use crate::constants::SEED_NODE_VAULT;
use crate::error::DiceError;
use crate::instructions::credit_vault_helper::transfer_lamports_program_owned;
use crate::state::{NodeVault, NodeVaultStatus};

/// Withdraw `amount` lamports from a `NodeVault` to its bound payout wallet.
///
/// This is the operator's one-button cash-out — the same instruction works
/// regardless of which services (VRF, PoL, sensor, HSM) credited the vault.
///
/// Only the vault's currently bound payout wallet can sign. The vault must
/// be `Bound` (not `Unbound` or `Frozen`). The amount must be non-zero and
/// must not exceed the vault's credited balance
/// (`total_earned - total_withdrawn`).
///
/// The withdraw also verifies that draining the account won't fall below
/// rent-exempt minimum — the vault PDA needs to remain valid for future
/// credits.
#[derive(Accounts)]
pub struct WithdrawFromVault<'info> {
    /// The operator's Solana wallet. Must match `vault.payout_wallet`.
    #[account(
        mut,
        constraint = payout_wallet.key() == vault.payout_wallet
            @ DiceError::VaultWrongPayoutWallet
    )]
    pub payout_wallet: Signer<'info>,

    #[account(
        mut,
        seeds = [SEED_NODE_VAULT, &crate::constants::device_id(&vault.device_pubkey)],
        bump,
    )]
    pub vault: Account<'info, NodeVault>,
}

pub fn handler(ctx: Context<WithdrawFromVault>, amount: u64) -> Result<()> {
    let vault_account_info = ctx.accounts.vault.to_account_info();
    let payout_account_info = ctx.accounts.payout_wallet.to_account_info();
    let vault = &mut ctx.accounts.vault;

    // Must be bound to withdraw. Unbound vaults can accumulate but cannot pay out.
    require!(
        vault.status == NodeVaultStatus::Bound,
        DiceError::VaultNotBound
    );

    // Update bookkeeping first — this also range-checks amount vs balance.
    vault.record_withdraw(amount)?;

    // Compute rent-exempt minimum so we never drain below it (the account
    // must remain valid for future credits).
    let rent = Rent::get()?;
    let min_rent = rent.minimum_balance(NodeVault::space());
    let vault_lamports = vault_account_info.lamports();
    require!(
        vault_lamports.saturating_sub(amount) >= min_rent,
        DiceError::VaultInsufficientBalance
    );

    // Physical lamport transfer. The vault PDA is program-owned, so we use
    // direct lamport manipulation instead of a system_program::transfer CPI
    // (which can't move lamports out of a data-bearing account).
    transfer_lamports_program_owned(
        &vault_account_info,
        &payout_account_info,
        amount,
    )?;

    msg!(
        "Vault withdraw: device={:?}, amount={}, balance_remaining={}",
        &vault.device_pubkey[..4],
        amount,
        vault.balance()
    );
    Ok(())
}
