// dice-vrf-macros — proc-macro crate
// Agent: SDK Agent

extern crate proc_macro;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

/// Derive macro that adds a `resolve_dice` associated method to any struct
/// that participates in a DICE randomness request.
///
/// The generated method delegates to [`dice_vrf::DiceVrfAccounts::resolve`]
/// so callers never have to construct PDAs by hand.
///
/// # Example
///
/// ```ignore
/// use dice_vrf_macros::DiceVrfAccounts;
///
/// #[derive(Accounts, DiceVrfAccounts)]
/// pub struct MyGameInstruction<'info> {
///     #[account(mut)]
///     pub player: Signer<'info>,
///     pub system_program: Program<'info, System>,
/// }
///
/// // In your instruction handler:
/// let dice = MyGameInstruction::resolve_dice(
///     ctx.accounts.player.key,
///     sequence,
///     &dice_vrf::DICE_PROGRAM_ID.parse().unwrap(),
/// );
/// ```
///
/// The `#[dice_vrf(...)]` attribute is accepted on fields but currently
/// reserved for future configuration (e.g. custom treasury override).
#[proc_macro_derive(DiceVrfAccounts, attributes(dice_vrf))]
pub fn derive_dice_vrf_accounts(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let expanded = quote! {
        impl #name {
            /// Resolve DICE VRF accounts for this instruction context.
            ///
            /// Returns a fully populated [`dice_vrf::DiceVrfAccounts`] with all
            /// PDAs derived from `requester` and `sequence`.
            pub fn resolve_dice(
                requester: &solana_sdk::pubkey::Pubkey,
                sequence: u64,
                program_id: &solana_sdk::pubkey::Pubkey,
            ) -> dice_vrf::DiceVrfAccounts {
                dice_vrf::DiceVrfAccounts::resolve(requester, sequence, program_id)
            }
        }
    };

    TokenStream::from(expanded)
}
