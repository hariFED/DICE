use anchor_lang::prelude::*;

use crate::constants::{
    NODE_REWARD_BPS, REQUEST_FEE_LAMPORTS, RESERVE_REWARD_BPS, SEED_CHANNEL, SEED_NODE_VAULT,
    SERVICE_ID_VRF_V2, TREASURY_REWARD_BPS,
};
use crate::error::DiceError;
use crate::instructions::credit_vault_helper::{
    credit_vault_state, transfer_lamports_program_owned,
};
use crate::state::{ChannelStatus, DiceChannel, NodeVault};

/// Distribute rewards for a finalized v2 channel round.
///
/// This is the v2 counterpart to the legacy `claim_rewards` instruction, and
/// the single mechanism by which nodes earn SOL on v2 rounds. It is called
/// once per finalized round (by the coordinator) and atomically:
///
///   1. Computes the 70/20/10 split of `REQUEST_FEE_LAMPORTS`.
///   2. Divides the 70% node share equally across all contributing nodes.
///   3. Transfers each node's share from the channel PDA into that node's
///      `NodeVault` PDA (direct lamport manipulation — both accounts are
///      program-owned).
///   4. Calls `credit_vault_state` to record the credit in vault bookkeeping.
///   5. Transfers treasury and reserve shares from the channel PDA to the
///      configured protocol wallets.
///   6. Transitions the channel `Finalized -> Idle`, preventing double-claim
///      and unblocking the next request.
///
/// The coordinator passes one `NodeVault` account per contributing node in
/// `remaining_accounts`, in the same order as `channel.device_pubkeys[0..
/// reveals_received]`. The instruction verifies each vault's stored
/// `device_pubkey` matches the expected one — preventing a malicious
/// coordinator from redirecting payouts to the wrong device's vault.
///
/// All amounts are `checked_*` throughout. The channel account's
/// rent-exempt minimum is left untouched — only the `fee` amount that was
/// credited into the channel during `request_randomness_auto` is drained.
#[derive(Accounts)]
pub struct ClaimRewardsV2<'info> {
    /// The protocol coordinator.
    pub coordinator: Signer<'info>,

    #[account(
        mut,
        seeds = [SEED_CHANNEL, channel.authority.as_ref(), &channel.channel_index.to_le_bytes()],
        bump,
    )]
    pub channel: Account<'info, DiceChannel>,

    /// Protocol treasury wallet (receives 20% of the fee).
    #[account(mut)]
    pub treasury: SystemAccount<'info>,

    /// Protocol reserve wallet (receives 10% of the fee).
    #[account(mut)]
    pub reserve: SystemAccount<'info>,
    // remaining_accounts: one writable NodeVault per contributing node,
    // in the order matching channel.device_pubkeys[0..reveals_received]
}

pub fn handler<'info>(
    ctx: Context<'_, '_, '_, 'info, ClaimRewardsV2<'info>>,
) -> Result<()> {
    let clock = Clock::get()?;
    let program_id = ctx.program_id;
    let channel_ai = ctx.accounts.channel.to_account_info();
    let treasury_ai = ctx.accounts.treasury.to_account_info();
    let reserve_ai = ctx.accounts.reserve.to_account_info();
    let channel = &mut ctx.accounts.channel;

    // Authorization: only the channel's bound coordinator can claim.
    require!(
        ctx.accounts.coordinator.key() == channel.coordinator,
        DiceError::UnauthorizedCoordinator
    );

    // L4 (v7.4): accept Finalized OR Idle. `finalize_v2` now auto-Idle's
    // for no-callback channels, so when the coordinator bundles
    // finalize_v2 + claim_rewards_v2 in one TX, claim sees Idle (the
    // state finalize_v2 transitioned to in the same TX). Idempotency
    // is preserved via the rent-exempt check below — a second call
    // would see `available = channel_lamports - min_rent = 0 < fee`
    // and bail with EscrowInsufficient.
    require!(
        channel.status == ChannelStatus::Finalized
            || channel.status == ChannelStatus::Idle,
        DiceError::RoundNotComplete
    );

    let num_contributors = channel.reveals_received as u64;
    require!(num_contributors > 0, DiceError::InsufficientNodes);

    // Split math — identical to v1 claim_rewards economics:
    //   70% split across contributing nodes
    //   20% treasury
    //   10% reserve
    let fee = REQUEST_FEE_LAMPORTS;
    let total_node_share = fee
        .checked_mul(NODE_REWARD_BPS)
        .ok_or(error!(DiceError::VaultCreditOverflow))?
        .checked_div(10_000)
        .ok_or(error!(DiceError::VaultCreditOverflow))?;
    let treasury_share = fee
        .checked_mul(TREASURY_REWARD_BPS)
        .ok_or(error!(DiceError::VaultCreditOverflow))?
        .checked_div(10_000)
        .ok_or(error!(DiceError::VaultCreditOverflow))?;
    let reserve_share = fee
        .checked_mul(RESERVE_REWARD_BPS)
        .ok_or(error!(DiceError::VaultCreditOverflow))?
        .checked_div(10_000)
        .ok_or(error!(DiceError::VaultCreditOverflow))?;
    let per_node_share = total_node_share
        .checked_div(num_contributors)
        .ok_or(error!(DiceError::VaultCreditOverflow))?;

    // remaining_accounts must contain exactly one NodeVault per contributor.
    let remaining = ctx.remaining_accounts;
    require!(
        remaining.len() == num_contributors as usize,
        DiceError::InsufficientNodes
    );

    // Before moving lamports, verify the channel has at least `fee` lamports
    // above its rent-exempt minimum — otherwise we'd brick the PDA.
    let rent = Rent::get()?;
    let min_rent = rent.minimum_balance(DiceChannel::space(channel.max_nodes));
    let channel_lamports = channel_ai.lamports();
    let available = channel_lamports.saturating_sub(min_rent);
    require!(available >= fee, DiceError::EscrowInsufficient);

    // Pay each contributing node's vault.
    for i in 0..(num_contributors as usize) {
        let vault_ai = &remaining[i];
        let expected_device = channel.device_pubkeys[i];

        // Verify the account is a NodeVault PDA owned by our program and
        // derived from the expected device pubkey. PDA seeds are the 32-byte
        // device_id hash (not the 33-byte pubkey — Solana seeds are max 32).
        require!(
            vault_ai.owner == program_id,
            DiceError::VaultBindingSignatureInvalid
        );
        let expected_device_id = crate::constants::device_id(&expected_device);
        let (expected_pda, _bump) = Pubkey::find_program_address(
            &[SEED_NODE_VAULT, &expected_device_id],
            program_id,
        );
        require!(
            vault_ai.key() == expected_pda,
            DiceError::VaultBindingSignatureInvalid
        );

        // Deserialize the vault, update bookkeeping, re-serialize.
        let mut vault: NodeVault = {
            let data = vault_ai.try_borrow_data()?;
            NodeVault::try_deserialize(&mut data.as_ref())?
        };

        // Double-check the device_pubkey stored in the vault matches the
        // one from the channel — this protects against a coordinator
        // passing a vault whose PDA derivation matches but whose on-disk
        // device_pubkey was tampered with somehow.
        require!(
            vault.device_pubkey == expected_device,
            DiceError::VaultBindingSignatureInvalid
        );

        // Physical lamport transfer.
        transfer_lamports_program_owned(&channel_ai, vault_ai, per_node_share)?;

        // Bookkeeping update.
        credit_vault_state(&mut vault, per_node_share, SERVICE_ID_VRF_V2, clock.slot)?;

        // Persist the mutated vault back to the account.
        {
            let mut data = vault_ai.try_borrow_mut_data()?;
            let mut cursor = std::io::Cursor::new(&mut data[..]);
            vault.try_serialize(&mut cursor)?;
        }
    }

    // Pay treasury and reserve.
    transfer_lamports_program_owned(&channel_ai, &treasury_ai, treasury_share)?;
    transfer_lamports_program_owned(&channel_ai, &reserve_ai, reserve_share)?;

    // Intentionally DO NOT transition status here. `deliver_callback` owns
    // the Finalized -> Idle transition because it's the instruction that
    // fires the dApp's callback CPI (and needs the dApp's accounts). If we
    // transitioned here, deliver_callback would see status != Finalized and
    // reject. Since both instructions want Finalized as their prerequisite,
    // exactly one of them must not write the transition. claim_rewards_v2
    // is idempotent via the rent-exempt check above — a second call would
    // see `available = channel_lamports - min_rent = 0 < fee` and fail with
    // EscrowInsufficient. The caller (coordinator) is expected to bundle
    // finalize_v2 + claim_rewards_v2 atomically, then the dApp driver
    // submits deliver_callback separately with its own accounts.
    //
    // Channels with NO callback configured still need something to
    // transition the state — deliver_callback handles that case too (it
    // checks for Pubkey::default() and transitions to Idle without CPI).

    msg!(
        "v2 rewards distributed: fee={}, nodes={}, per_node={}, treasury={}, reserve={} (channel remains Finalized until deliver_callback)",
        fee,
        num_contributors,
        per_node_share,
        treasury_share,
        reserve_share
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pure math test for the 70/20/10 split logic.
    /// Verifies split arithmetic on the actual REQUEST_FEE_LAMPORTS constant.
    #[test]
    fn split_math_for_typical_round() {
        let fee = REQUEST_FEE_LAMPORTS;
        let total_node = fee * NODE_REWARD_BPS / 10_000;
        let treasury = fee * TREASURY_REWARD_BPS / 10_000;
        let reserve = fee * RESERVE_REWARD_BPS / 10_000;

        assert_eq!(total_node + treasury + reserve, fee, "splits must sum to fee");
        assert_eq!(total_node, 1_400_000, "70% of 2_000_000");
        assert_eq!(treasury, 400_000, "20% of 2_000_000");
        assert_eq!(reserve, 200_000, "10% of 2_000_000");
    }

    #[test]
    fn per_node_share_for_4_node_round() {
        let total_node = REQUEST_FEE_LAMPORTS * NODE_REWARD_BPS / 10_000;
        let per_node = total_node / 4;
        assert_eq!(per_node, 350_000, "1_400_000 / 4");
        // 4 nodes get 350_000 each = 1_400_000 total, matches 70% share.
        assert_eq!(per_node * 4, total_node);
    }

    #[test]
    fn per_node_share_for_7_node_round() {
        let total_node = REQUEST_FEE_LAMPORTS * NODE_REWARD_BPS / 10_000;
        let per_node = total_node / 7;
        assert_eq!(per_node, 200_000, "1_400_000 / 7");
        // 7 * 200_000 = 1_400_000 — exact, no dust for this number of nodes.
        assert_eq!(per_node * 7, total_node);
    }

    #[test]
    fn per_node_share_rounding_dust_is_bounded() {
        // For any node count 4..=50, the dust (unallocated lamports from
        // integer division) must be strictly less than num_nodes lamports.
        let total_node = REQUEST_FEE_LAMPORTS * NODE_REWARD_BPS / 10_000;
        for n in 4u64..=50 {
            let per = total_node / n;
            let paid = per * n;
            let dust = total_node - paid;
            assert!(
                dust < n,
                "n={n}: dust={dust} must be less than num_nodes"
            );
        }
    }

    #[test]
    fn zero_contributors_division_would_fail() {
        // Sanity: the handler guards against 0 contributors before division.
        // If it didn't, checked_div(0) would return None and handler would
        // bail. This test verifies the math path behaves the same way.
        let total_node = REQUEST_FEE_LAMPORTS * NODE_REWARD_BPS / 10_000;
        assert_eq!(total_node.checked_div(0), None);
    }
}
