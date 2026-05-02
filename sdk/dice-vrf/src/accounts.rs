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

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::pubkey::Pubkey;

    #[test]
    fn test_resolve_produces_valid_pdas() {
        let requester = Pubkey::new_unique();
        let program_id = Pubkey::new_unique();
        let accounts = DiceVrfAccounts::resolve(&requester, 1, &program_id);

        // All derived PDAs must differ from Pubkey::default()
        assert_ne!(accounts.randomness_request, Pubkey::default());
        assert_ne!(accounts.randomness_result, Pubkey::default());
        assert_ne!(accounts.escrow, Pubkey::default());

        // All three derived PDAs must be distinct from each other
        assert_ne!(accounts.randomness_request, accounts.randomness_result);
        assert_ne!(accounts.randomness_request, accounts.escrow);
        assert_ne!(accounts.randomness_result, accounts.escrow);

        // Stored fields should match inputs
        assert_eq!(accounts.program_id, program_id);
        assert_eq!(accounts.requester, requester);
        assert_eq!(accounts.sequence, 1);
    }

    #[test]
    fn test_resolve_deterministic() {
        let requester = Pubkey::new_unique();
        let program_id = Pubkey::new_unique();
        let a = DiceVrfAccounts::resolve(&requester, 42, &program_id);
        let b = DiceVrfAccounts::resolve(&requester, 42, &program_id);

        assert_eq!(a.randomness_request, b.randomness_request);
        assert_eq!(a.randomness_result, b.randomness_result);
        assert_eq!(a.escrow, b.escrow);
        assert_eq!(a.request_bump, b.request_bump);
        assert_eq!(a.result_bump, b.result_bump);
        assert_eq!(a.escrow_bump, b.escrow_bump);
    }

    #[test]
    fn test_resolve_differs_by_sequence() {
        let requester = Pubkey::new_unique();
        let program_id = Pubkey::new_unique();
        let a = DiceVrfAccounts::resolve(&requester, 1, &program_id);
        let b = DiceVrfAccounts::resolve(&requester, 2, &program_id);

        assert_ne!(a.randomness_request, b.randomness_request,
            "different sequences must produce different request PDAs");
        assert_ne!(a.randomness_result, b.randomness_result,
            "different sequences must produce different result PDAs");
        assert_ne!(a.escrow, b.escrow,
            "different sequences must produce different escrow PDAs");
    }

    #[test]
    fn test_resolve_differs_by_requester() {
        let req_a = Pubkey::new_unique();
        let req_b = Pubkey::new_unique();
        let program_id = Pubkey::new_unique();
        let a = DiceVrfAccounts::resolve(&req_a, 1, &program_id);
        let b = DiceVrfAccounts::resolve(&req_b, 1, &program_id);

        assert_ne!(a.randomness_request, b.randomness_request,
            "different requesters must produce different request PDAs");
        assert_ne!(a.randomness_result, b.randomness_result,
            "different requesters must produce different result PDAs");
        assert_ne!(a.escrow, b.escrow,
            "different requesters must produce different escrow PDAs");
    }

    #[test]
    fn test_to_account_metas_count() {
        let requester = Pubkey::new_unique();
        let program_id = Pubkey::new_unique();
        let accounts = DiceVrfAccounts::resolve(&requester, 1, &program_id);
        let metas = accounts.to_account_metas();
        assert_eq!(metas.len(), 5, "to_account_metas should produce exactly 5 account metas");
    }

    #[test]
    fn test_to_account_metas_signer() {
        let requester = Pubkey::new_unique();
        let program_id = Pubkey::new_unique();
        let accounts = DiceVrfAccounts::resolve(&requester, 1, &program_id);
        let metas = accounts.to_account_metas();

        // Index 0: randomness_request — writable, not signer
        assert!(metas[0].is_writable);
        assert!(!metas[0].is_signer);
        assert_eq!(metas[0].pubkey, accounts.randomness_request);

        // Index 1: randomness_result — writable, not signer
        assert!(metas[1].is_writable);
        assert!(!metas[1].is_signer);
        assert_eq!(metas[1].pubkey, accounts.randomness_result);

        // Index 2: escrow — writable, not signer
        assert!(metas[2].is_writable);
        assert!(!metas[2].is_signer);
        assert_eq!(metas[2].pubkey, accounts.escrow);

        // Index 3: requester — writable signer
        assert!(metas[3].is_writable);
        assert!(metas[3].is_signer, "requester must be a signer");
        assert_eq!(metas[3].pubkey, requester);

        // Index 4: program_id — read-only, not signer
        assert!(!metas[4].is_writable);
        assert!(!metas[4].is_signer);
        assert_eq!(metas[4].pubkey, program_id);
    }

    #[test]
    fn test_bumps_are_canonical() {
        let requester = Pubkey::new_unique();
        let program_id = Pubkey::new_unique();

        // Test across several sequences to increase confidence
        for seq in [1u64, 100, 999, u64::MAX] {
            let accounts = DiceVrfAccounts::resolve(&requester, seq, &program_id);

            // Bumps from find_program_address are canonical (highest valid),
            // and must be < 256 (they are u8, so always true, but we verify
            // they match re-derivation).
            let (_, expected_req_bump) =
                pda::randomness_request_pda(&requester, seq, &program_id);
            let (_, expected_res_bump) =
                pda::randomness_result_pda(&requester, seq, &program_id);
            let (_, expected_esc_bump) =
                pda::escrow_pda(&requester, seq, &program_id);

            assert_eq!(accounts.request_bump, expected_req_bump,
                "request bump mismatch for seq={}", seq);
            assert_eq!(accounts.result_bump, expected_res_bump,
                "result bump mismatch for seq={}", seq);
            assert_eq!(accounts.escrow_bump, expected_esc_bump,
                "escrow bump mismatch for seq={}", seq);
        }
    }
}
