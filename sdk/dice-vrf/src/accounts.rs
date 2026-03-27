// DiceVrfAccounts — account resolution helper.
// Agent: SDK Agent

use solana_sdk::{
    instruction::AccountMeta,
    pubkey::Pubkey,
};
use crate::pda;

/// Resolves all account addresses needed for a DICE randomness request.
///
/// Add one field to your instruction's `Accounts` context and all PDAs are
/// derived automatically — no manual account management needed.
///
/// # Example
///
/// ```ignore
/// #[derive(Accounts)]
/// pub struct MyGame<'info> {
///     #[account(mut)]
///     pub player: Signer<'info>,
///     pub dice: DiceVrfAccounts,
///     pub system_program: Program<'info, System>,
/// }
///
/// pub fn play(ctx: Context<MyGame>) -> Result<()> {
///     let dice = DiceVrfAccounts::resolve(
///         ctx.accounts.player.key,
///         1,
///         &dice_vrf::DICE_PROGRAM_ID.parse().unwrap(),
///     );
///     let ix = dice_vrf::cpi::request_randomness_ix(&dice, 1);
///     solana_program::program::invoke(&ix, &[...])?;
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone)]
pub struct DiceVrfAccounts {
    /// The DICE program that owns all derived PDAs.
    pub program_id: Pubkey,
    /// The wallet (or program) that is paying for this request.
    pub requester: Pubkey,
    /// Monotonically increasing request counter scoped to `requester`.
    pub sequence: u64,

    // ── Derived PDAs ─────────────────────────────────────────────────────────
    /// The `RandomnessRequest` PDA — tracks request lifecycle.
    pub randomness_request: Pubkey,
    /// The `RandomnessResult` PDA — written once the request is finalized.
    pub randomness_result: Pubkey,
    /// The `EscrowAccount` PDA — holds the 0.002 SOL fee.
    pub escrow: Pubkey,

    // ── Canonical bumps (needed for PDA signing in CPI calls) ────────────────
    /// Canonical bump for `randomness_request`.
    pub request_bump: u8,
    /// Canonical bump for `randomness_result`.
    pub result_bump: u8,
    /// Canonical bump for `escrow`.
    pub escrow_bump: u8,
}

impl DiceVrfAccounts {
    /// Resolve all PDAs from the minimal set of inputs.
    ///
    /// Calls [`Pubkey::find_program_address`] for each account so bumps are
    /// always canonical. This is the only constructor you need in normal use.
    ///
    /// # Arguments
    /// * `requester`  — the signer paying for the request.
    /// * `sequence`   — per-requester request counter (start at 1, increment
    ///                  each time you call `request_randomness`).
    /// * `program_id` — deployed DICE program ID.
    pub fn resolve(requester: &Pubkey, sequence: u64, program_id: &Pubkey) -> Self {
        let (randomness_request, request_bump) =
            pda::randomness_request_pda(requester, sequence, program_id);
        let (randomness_result, result_bump) =
            pda::randomness_result_pda(requester, sequence, program_id);
        let (escrow, escrow_bump) =
            pda::escrow_pda(requester, sequence, program_id);

        Self {
            program_id: *program_id,
            requester: *requester,
            sequence,
            randomness_request,
            randomness_result,
            escrow,
            request_bump,
            result_bump,
            escrow_bump,
        }
    }

    /// Convert to a list of [`AccountMeta`] suitable for a CPI instruction.
    ///
    /// The ordering matches the `request_randomness` instruction's account
    /// layout as defined in the DICE Anchor program:
    ///
    /// 1. `randomness_request` — writable (will be initialized / mutated)
    /// 2. `randomness_result`  — writable (written on finalization)
    /// 3. `escrow`             — writable (fee is transferred out)
    /// 4. `requester`          — writable signer (pays rent + fee)
    /// 5. `program_id`         — executable, non-writable
    pub fn to_account_metas(&self) -> Vec<AccountMeta> {
        vec![
            AccountMeta::new(self.randomness_request, false),
            AccountMeta::new(self.randomness_result, false),
            AccountMeta::new(self.escrow, false),
            AccountMeta::new(self.requester, true),
            AccountMeta::new_readonly(self.program_id, false),
        ]
    }
}
