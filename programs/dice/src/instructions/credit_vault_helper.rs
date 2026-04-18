use anchor_lang::prelude::*;
use anchor_lang::system_program;

use crate::error::DiceError;
use crate::state::{NodeVault, NodeVaultStatus};

/// Credit a `NodeVault` with `amount` lamports earned from `service_id`.
///
/// This is the single entrypoint every DICE service uses to pay nodes. The
/// pattern is intentionally minimal:
///
///   1. Physically transfer `amount` lamports from the service's escrow
///      account into the vault PDA (caller decides the source).
///   2. Call `credit_vault` to record the credit in the vault's totals and
///      service counters.
///
/// The vault must not be `Frozen`. Unbound vaults are allowed — credits
/// accumulate until the operator binds a wallet (see `register_node_vault`).
///
/// # Arguments
/// * `vault`         — writable `NodeVault` account
/// * `amount`        — lamports being credited (must be > 0)
/// * `service_id`    — logical service identifier (see constants.rs)
/// * `current_slot`  — `Clock::get()?.slot` from the caller
///
/// # Note on lamport movement
/// This helper does NOT transfer lamports. It only records the credit in
/// vault state. The caller must ensure the physical transfer happens in the
/// same instruction context (via direct lamport manipulation on a PDA
/// account owned by the program, or via a `system_program::transfer` CPI
/// from a non-PDA source).
pub fn credit_vault_state(
    vault: &mut NodeVault,
    amount: u64,
    service_id: u8,
    current_slot: u64,
) -> Result<()> {
    require!(amount > 0, DiceError::VaultZeroAmount);
    require!(vault.status != NodeVaultStatus::Frozen, DiceError::VaultFrozen);
    vault.record_credit(amount, service_id, current_slot)?;
    Ok(())
}

/// Transfer `amount` lamports from a service-owned PDA (`from`) into the
/// `vault` PDA using direct lamport manipulation. Both accounts must be
/// owned by the DICE program. After this returns, call `credit_vault_state`
/// to update the vault's bookkeeping.
///
/// This is the standard pattern for PDA-to-PDA transfers when the source
/// account is program-owned (e.g., `EscrowAccount`, `DiceChannel`). The
/// source account's `data_len()` is preserved; only its lamports field
/// changes. A `system_program::transfer` CPI would fail here because the
/// system program cannot transfer from an account that has data.
pub fn transfer_lamports_program_owned(
    from: &AccountInfo,
    to: &AccountInfo,
    amount: u64,
) -> Result<()> {
    let from_lamports = from.lamports();
    require!(
        from_lamports >= amount,
        DiceError::VaultInsufficientBalance
    );

    **from.try_borrow_mut_lamports()? = from_lamports
        .checked_sub(amount)
        .ok_or(error!(DiceError::VaultCreditOverflow))?;
    **to.try_borrow_mut_lamports()? = to
        .lamports()
        .checked_add(amount)
        .ok_or(error!(DiceError::VaultCreditOverflow))?;

    Ok(())
}

/// Transfer `amount` lamports from a system-owned signer (`from`) into a
/// program-owned PDA (`to`) via a `system_program::transfer` CPI.
///
/// Use this variant when the source is a plain wallet (e.g., developer
/// topping up a vault directly). For PDA-to-PDA transfers use
/// `transfer_lamports_program_owned` instead.
// Anchor 1.0: CpiContext::new takes the program *Pubkey*, not its
// AccountInfo. We no longer need the system_program AccountInfo to be
// passed in — `solana_invoke` resolves it from the instruction's program_id.
#[allow(dead_code)]
pub fn transfer_lamports_from_signer<'info>(
    from: AccountInfo<'info>,
    to: AccountInfo<'info>,
    _system_program: AccountInfo<'info>,
    amount: u64,
) -> Result<()> {
    let cpi_ctx = CpiContext::new(
        anchor_lang::system_program::System::id(),
        system_program::Transfer { from, to },
    );
    system_program::transfer(cpi_ctx, amount)
}
