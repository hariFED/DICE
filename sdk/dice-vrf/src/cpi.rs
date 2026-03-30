// CPI instruction builders for on-chain programs consuming DICE randomness.
// Agent: SDK Agent

use sha2::{Digest, Sha256};
use solana_sdk::{instruction::Instruction, pubkey::Pubkey};

use crate::accounts::DiceVrfAccounts;

// ── Discriminator helper ─────────────────────────────────────────────────────

/// Compute the 8-byte Anchor instruction discriminator for `name`.
///
/// Anchor derives discriminators as the first 8 bytes of
/// `SHA-256("global:<instruction_name>")`.
fn instruction_discriminator(name: &str) -> [u8; 8] {
    let mut hasher = Sha256::new();
    hasher.update(format!("global:{}", name).as_bytes());
    let result = hasher.finalize();
    result[..8].try_into().unwrap()
}

// ── On-chain CPI helpers ─────────────────────────────────────────────────────

/// Build the `request_randomness` CPI instruction.
///
/// Pass the returned [`Instruction`] to
/// `solana_program::program::invoke` (or `invoke_signed`) from within your
/// Anchor program's instruction handler.
///
/// # Arguments
/// * `accounts`  — resolved via [`DiceVrfAccounts::resolve`].
/// * `sequence`  — the same per-requester counter used to derive the PDAs.
/// * `callback_program_id` — program to CPI-invoke with the randomness result.
///   Pass `Pubkey::default()` if you prefer to poll for results instead.
///
/// # Example
///
/// ```ignore
/// let accounts = DiceVrfAccounts::resolve(
///     ctx.accounts.player.key,
///     sequence,
///     &dice_vrf::DICE_PROGRAM_ID.parse().unwrap(),
/// );
/// // Use Pubkey::default() for poll-based consumption, or your program's ID for CPI callback
/// let ix = dice_vrf::cpi::request_randomness_ix(&accounts, sequence, &my_program_id);
/// solana_program::program::invoke(&ix, account_infos)?;
/// ```
pub fn request_randomness_ix(
    accounts: &DiceVrfAccounts,
    sequence: u64,
    callback_program_id: &Pubkey,
) -> Instruction {
    // 8-byte discriminator || sequence (u64 LE) || callback_program_id (32 bytes)
    let mut data = Vec::with_capacity(48);
    data.extend_from_slice(&instruction_discriminator("request_randomness"));
    data.extend_from_slice(&sequence.to_le_bytes());
    data.extend_from_slice(callback_program_id.as_ref());

    Instruction {
        program_id: accounts.program_id,
        accounts: accounts.to_account_metas(),
        data,
    }
}

/// Well-known 8-byte Anchor discriminator for the `dice_callback` instruction.
///
/// Developer programs receiving CPI callbacks from DICE must name their
/// handler `dice_callback`. The instruction data layout is:
///
/// ```text
/// [0..8]   DICE_CALLBACK_DISCRIMINATOR
/// [8..40]  request_key: Pubkey   (the RandomnessRequest PDA)
/// [40..72] randomness:  [u8; 32] (the final randomness value)
/// ```
pub fn dice_callback_discriminator() -> [u8; 8] {
    instruction_discriminator("dice_callback")
}

// ── v2.0 Channel CPI helpers ────────────────────────────────────────────────

/// Build the `init_channel` CPI instruction (v2.0).
///
/// Creates a reusable DiceChannel PDA. Developer pays rent once.
pub fn init_channel_ix(
    program_id: &Pubkey,
    authority: &Pubkey,
    channel_index: u16,
    max_nodes: u8,
    callback_program_id: &Pubkey,
) -> Instruction {
    let (channel, _) = crate::pda::channel_pda(authority, channel_index, program_id);
    let mut data = Vec::with_capacity(51);
    data.extend_from_slice(&instruction_discriminator("init_channel"));
    data.extend_from_slice(&channel_index.to_le_bytes());
    data.push(max_nodes);
    data.extend_from_slice(callback_program_id.as_ref());

    Instruction {
        program_id: *program_id,
        accounts: vec![
            solana_sdk::instruction::AccountMeta::new(*authority, true),
            solana_sdk::instruction::AccountMeta::new(channel, false),
            solana_sdk::instruction::AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
        ],
        data,
    }
}

/// Build the `fund_channel` CPI instruction (v2.0).
pub fn fund_channel_ix(
    program_id: &Pubkey,
    authority: &Pubkey,
    channel_index: u16,
    amount: u64,
) -> Instruction {
    let (channel, _) = crate::pda::channel_pda(authority, channel_index, program_id);
    let mut data = Vec::with_capacity(16);
    data.extend_from_slice(&instruction_discriminator("fund_channel"));
    data.extend_from_slice(&amount.to_le_bytes());

    Instruction {
        program_id: *program_id,
        accounts: vec![
            solana_sdk::instruction::AccountMeta::new(*authority, true),
            solana_sdk::instruction::AccountMeta::new(channel, false),
            solana_sdk::instruction::AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
        ],
        data,
    }
}

/// Build the `request_randomness_v2` CPI instruction (v2.0).
///
/// Resets the channel, deducts fee from prepaid balance.
pub fn request_randomness_v2_ix(
    program_id: &Pubkey,
    authority: &Pubkey,
    channel_index: u16,
    node_count: u8,
) -> Instruction {
    let (channel, _) = crate::pda::channel_pda(authority, channel_index, program_id);
    let mut data = Vec::with_capacity(9);
    data.extend_from_slice(&instruction_discriminator("request_randomness_v2"));
    data.push(node_count);

    Instruction {
        program_id: *program_id,
        accounts: vec![
            solana_sdk::instruction::AccountMeta::new(*authority, true),
            solana_sdk::instruction::AccountMeta::new(channel, false),
        ],
        data,
    }
}

/// Decode the channel status and randomness from a raw `DiceChannel` account.
///
/// Returns `Some(randomness)` if the channel has been finalized, `None` otherwise.
///
/// DiceChannel layout (relevant offsets):
/// ```text
/// [0..8]     discriminator
/// [8..40]    authority
/// [40..42]   channel_index
/// [42]       max_nodes
/// [43]       status (0=Idle, 1=Pending, 2=CommitPhase, 3=RevealPhase, 4=Finalized, 5=Failed)
/// [44..52]   round_id
/// ...
/// [111..143] randomness [u8; 32]
/// ```
pub fn decode_channel_randomness(account_data: &[u8]) -> Option<[u8; 32]> {
    if account_data.len() < 143 {
        return None;
    }
    // Status byte at offset 43
    let status = account_data[43];
    // Status 4 = Finalized, Status 0 = Idle (callback delivered, result available)
    if status != 4 && status != 0 {
        return None;
    }
    let randomness: [u8; 32] = account_data[111..143].try_into().ok()?;
    if randomness == [0u8; 32] {
        return None;
    }
    Some(randomness)
}

// ── Off-chain decode helper ──────────────────────────────────────────────────

/// Decode the final randomness value from the raw bytes of a
/// `RandomnessResult` account.
///
/// Returns `Some([u8; 32])` when the account has been finalized, `None`
/// otherwise (account not yet written, or the result field is still zeroed).
///
/// The on-chain `RandomnessResult` layout (borsh, field order):
/// ```text
/// [0..8]   8-byte Anchor account discriminator
/// [8..40]  request: Pubkey (32 bytes)
/// [40..72] randomness: [u8; 32]
/// ```
///
/// # Example
///
/// ```ignore
/// let account = rpc_client.get_account(&result_pda).await?;
/// if let Some(randomness) = dice_vrf::cpi::decode_randomness_result(&account.data) {
///     println!("randomness: {}", hex::encode(randomness));
/// }
/// ```
pub fn decode_randomness_result(account_data: &[u8]) -> Option<[u8; 32]> {
    // Minimum: 8 discriminator + 32 request + 32 randomness = 72 bytes
    if account_data.len() < 72 {
        return None;
    }

    // Skip 8-byte discriminator + 32-byte request pubkey
    let randomness_bytes: [u8; 32] = account_data[40..72].try_into().ok()?;

    // A zeroed result means the account exists but hasn't been finalized yet
    if randomness_bytes == [0u8; 32] {
        return None;
    }

    Some(randomness_bytes)
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discriminator_is_deterministic() {
        let a = instruction_discriminator("request_randomness");
        let b = instruction_discriminator("request_randomness");
        assert_eq!(a, b);
    }

    #[test]
    fn discriminator_differs_by_name() {
        let a = instruction_discriminator("request_randomness");
        let b = instruction_discriminator("finalize_randomness");
        assert_ne!(a, b);
    }

    #[test]
    fn decode_randomness_result_too_short() {
        assert_eq!(decode_randomness_result(&[0u8; 71]), None);
    }

    #[test]
    fn decode_randomness_result_zeroed() {
        // All-zero means not yet finalized
        assert_eq!(decode_randomness_result(&[0u8; 72]), None);
    }

    #[test]
    fn decode_randomness_result_valid() {
        let mut data = vec![0u8; 72];
        // [0..8]   discriminator (leave zero)
        // [8..40]  request pubkey (leave zero)
        // [40..72] randomness: fill with 0xAB
        data[40..72].fill(0xAB);
        let result = decode_randomness_result(&data);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), [0xABu8; 32]);
    }

    #[test]
    fn request_randomness_ix_data_layout() {
        let program_id = Pubkey::new_unique();
        let requester = Pubkey::new_unique();
        let callback = Pubkey::new_unique();
        let accounts = DiceVrfAccounts::resolve(&requester, 7, &program_id);
        let ix = request_randomness_ix(&accounts, 7, &callback);

        assert_eq!(ix.program_id, program_id);
        // 8 discriminator + 8 sequence + 32 callback_program_id = 48
        assert_eq!(ix.data.len(), 48);
        // First 8 bytes are the discriminator
        let disc = instruction_discriminator("request_randomness");
        assert_eq!(&ix.data[0..8], &disc);
        // Next 8 bytes are the sequence in LE
        assert_eq!(&ix.data[8..16], &7u64.to_le_bytes());
        // Next 32 bytes are the callback program ID
        assert_eq!(&ix.data[16..48], callback.as_ref());
    }

    #[test]
    fn dice_callback_discriminator_is_stable() {
        let d = dice_callback_discriminator();
        assert_eq!(d, [128, 131, 129, 45, 53, 113, 215, 151]);
    }
}
