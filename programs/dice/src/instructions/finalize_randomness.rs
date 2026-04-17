use anchor_lang::prelude::*;
use solana_program::hash::hashv;
use solana_program::instruction::{AccountMeta, Instruction as SolInstruction};
use solana_program::program::invoke;

use crate::constants::{MIN_NODES_REQUIRED, SEED_REQUEST, SEED_RESULT};
use crate::error::DiceError;
use crate::state::{RandomnessRequest, RandomnessResult, RequestStatus, RevealRecord};

#[derive(Accounts)]
pub struct FinalizeRandomness<'info> {
    #[account(mut)]
    pub coordinator: Signer<'info>,

    #[account(
        mut,
        seeds = [
            SEED_REQUEST,
            randomness_request.requester.as_ref(),
            &randomness_request.sequence.to_le_bytes(),
        ],
        bump,
    )]
    pub randomness_request: Account<'info, RandomnessRequest>,

    #[account(
        init,
        payer = coordinator,
        space = RandomnessResult::LEN,
        seeds = [
            SEED_RESULT,
            randomness_request.requester.as_ref(),
            &randomness_request.sequence.to_le_bytes(),
        ],
        bump,
    )]
    pub randomness_result: Account<'info, RandomnessResult>,

    pub system_program: Program<'info, System>,
    // remaining_accounts layout:
    //   [0 .. reveals_received]   — RevealRecord PDAs
    //   [reveals_received]        — callback program (only if callback_program_id != default)
    //   [reveals_received+1 ..]   — additional accounts forwarded to callback
}

/// Well-known 8-byte discriminator for the developer's `dice_callback` instruction.
///
/// Computed as `SHA-256("global:dice_callback")[0..8]`.
/// Developer programs MUST use this exact instruction name for the callback to land.
const DICE_CALLBACK_DISCRIMINATOR: [u8; 8] = {
    // Precomputed: SHA-256("global:dice_callback")[0..8]
    // Verified by the verify_callback_discriminator unit test.
    [128, 131, 129, 45, 53, 113, 215, 151]
};

pub fn handler<'info>(ctx: Context<'info, FinalizeRandomness<'info>>) -> Result<()> {
    let clock = Clock::get()?;

    // Capture keys before mutable borrows
    let request_key = ctx.accounts.randomness_request.key();
    let req_sequence = ctx.accounts.randomness_request.sequence;
    let callback_program_id = ctx.accounts.randomness_request.callback_program_id;
    let reveals_received = ctx.accounts.randomness_request.reveals_received;

    // Only finalize once
    require!(
        ctx.accounts.randomness_request.status != RequestStatus::Finalized,
        DiceError::RoundAlreadyFinalized
    );

    // Check if we have enough reveals.
    let reveal_deadline = ctx.accounts.randomness_request.reveal_deadline_slot;
    let timed_out = reveal_deadline > 0 && clock.slot > reveal_deadline;

    if reveals_received < MIN_NODES_REQUIRED {
        if timed_out {
            ctx.accounts.randomness_request.status = RequestStatus::Failed;
            return err!(DiceError::InsufficientNodes);
        }
        return err!(DiceError::RoundNotComplete);
    }

    // Collect entropy values from the remaining_accounts (RevealRecord PDAs)
    let remaining = ctx.remaining_accounts;
    let reveal_count = reveals_received as usize;
    require!(remaining.len() >= reveal_count, DiceError::InsufficientNodes);

    let mut entropy_slices: Vec<[u8; 32]> = Vec::with_capacity(reveal_count);
    let mut contributing_nodes: [[u8; 33]; 7] = [[0u8; 33]; 7];
    let mut contributing_count: u8 = 0;

    for account_info in remaining[..reveal_count].iter() {
        let data = account_info.try_borrow_data()?;
        let reveal: RevealRecord = RevealRecord::try_deserialize(&mut &data[..])?;

        // Verify this reveal belongs to the current request
        require!(reveal.request == request_key, DiceError::UnauthorizedNode);

        entropy_slices.push(reveal.entropy);
        if contributing_count < 7 {
            contributing_nodes[contributing_count as usize] = reveal.device_pubkey;
            contributing_count += 1;
        }
    }

    require!(
        contributing_count >= MIN_NODES_REQUIRED,
        DiceError::InsufficientNodes
    );

    // Compute final randomness: SHA-256(entropy1 || entropy2 || ... || entropyN)
    let refs: Vec<&[u8]> = entropy_slices.iter().map(|e| e.as_ref()).collect();
    let final_hash = hashv(&refs);
    let final_bytes = final_hash.to_bytes();

    // Write the result
    {
        let result = &mut ctx.accounts.randomness_result;
        result.request = request_key;
        result.randomness = final_bytes;
        result.contributing_nodes = contributing_nodes;
        result.contributing_count = contributing_count;
        result.finalized_slot = clock.slot;
    }

    // Mark request as finalized
    ctx.accounts.randomness_request.status = RequestStatus::Finalized;

    msg!(
        "Randomness finalized for seq={} with {} nodes at slot {}. Result: {:?}",
        req_sequence,
        contributing_count,
        clock.slot,
        &final_bytes[..8]
    );

    // ---- CPI callback to developer's program --------------------------------
    //
    // If the developer provided a callback_program_id when requesting randomness,
    // we CPI-invoke their `dice_callback` instruction with the randomness value.
    //
    // The developer's program MUST implement:
    //   pub fn dice_callback(ctx: Context<...>, request_key: Pubkey, randomness: [u8; 32]) -> Result<()>
    //
    // The callback receives the RandomnessResult PDA as its first account (read-only,
    // owned by DICE program) so it can independently verify the randomness is authentic.
    // Additional accounts needed by the callback are forwarded from remaining_accounts.
    let has_callback = callback_program_id != Pubkey::default();

    if has_callback {
        // The callback program must be the next remaining_account after the reveal PDAs
        let callback_start = reveal_count;
        require!(
            remaining.len() > callback_start,
            DiceError::CallbackProgramMissing
        );

        let callback_program_info = &remaining[callback_start];
        require!(
            callback_program_info.key() == callback_program_id,
            DiceError::CallbackProgramMismatch
        );

        // Build callback instruction data:
        //   [0..8]   DICE_CALLBACK_DISCRIMINATOR
        //   [8..40]  request_key (Pubkey)
        //   [40..72] randomness ([u8; 32])
        let mut callback_data = Vec::with_capacity(72);
        callback_data.extend_from_slice(&DICE_CALLBACK_DISCRIMINATOR);
        callback_data.extend_from_slice(request_key.as_ref());
        callback_data.extend_from_slice(&final_bytes);

        // Accounts forwarded to callback:
        //   [0] = RandomnessResult PDA (read-only, for verification)
        //   [1..] = any additional accounts the coordinator passed
        let result_key = ctx.accounts.randomness_result.key();
        let mut account_metas = vec![AccountMeta::new_readonly(result_key, false)];

        let extra_accounts = &remaining[callback_start + 1..];
        for acc in extra_accounts {
            if acc.is_writable {
                account_metas.push(AccountMeta::new(*acc.key, acc.is_signer));
            } else {
                account_metas.push(AccountMeta::new_readonly(*acc.key, acc.is_signer));
            }
        }

        let ix = SolInstruction {
            program_id: callback_program_id,
            accounts: account_metas,
            data: callback_data,
        };

        // Gather account_infos for the CPI
        let mut account_infos = vec![ctx.accounts.randomness_result.to_account_info()];
        for acc in extra_accounts {
            account_infos.push(acc.clone());
        }
        // The callback program itself must also be in the account_infos
        account_infos.push(callback_program_info.clone());

        invoke(&ix, &account_infos).map_err(|_| error!(DiceError::CallbackFailed))?;

        msg!("CPI callback invoked: program={}", callback_program_id);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_callback_discriminator() {
        // Ensure the hardcoded discriminator matches the SHA-256 computation.
        let hash = solana_program::hash::hashv(&[b"global:dice_callback"]);
        let expected: [u8; 8] = hash.to_bytes()[..8].try_into().unwrap();
        assert_eq!(DICE_CALLBACK_DISCRIMINATOR, expected);
    }
}
