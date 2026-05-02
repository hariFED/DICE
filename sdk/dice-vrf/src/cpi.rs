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
/// [8..40]    authority (32)
/// [40..72]   coordinator (32)
/// [72..74]   channel_index (u16)
/// [74]       max_nodes (u8)
/// [75]       status (0=Idle, 1=Pending, 2=CommitPhase, 3=RevealPhase, 4=Finalized, 5=Failed)
/// [76..84]   round_id (u64)
/// [84]       node_count (u8)
/// [85]       commits_received (u8)
/// [86]       reveals_received (u8)
/// [87..95]   created_slot (u64)
/// [95..103]  commit_deadline_slot (u64)
/// [103..111] reveal_deadline_slot (u64)
/// [111..119] balance (u64)
/// [119..151] callback_program_id (32)
/// [151..183] randomness [u8; 32]
/// ```
pub fn decode_channel_randomness(account_data: &[u8]) -> Option<[u8; 32]> {
    if account_data.len() < 183 {
        return None;
    }
    // Status byte at offset 75
    let status = account_data[75];
    // Status 4 = Finalized, Status 0 = Idle (callback delivered, result available)
    if status != 4 && status != 0 {
        return None;
    }
    let randomness: [u8; 32] = account_data[151..183].try_into().ok()?;
    if randomness == [0u8; 32] {
        return None;
    }
    Some(randomness)
}

/// Build the `request_randomness_auto` CPI instruction.
///
/// Auto-funds from developer wallet if channel balance is insufficient.
/// Channel must already exist (call `init_channel` first).
///
/// # Arguments
/// * `program_id` — the DICE program ID
/// * `authority` — developer's wallet (signer, pays fee if balance insufficient)
/// * `channel_index` — channel slot (use 0 for single-channel integrations)
/// * `node_count` — how many nodes for this round (4 to max_nodes)
pub fn request_randomness_auto_ix(
    program_id: &Pubkey,
    authority: &Pubkey,
    channel_index: u16,
    node_count: u8,
) -> Instruction {
    let (channel, _) = crate::pda::channel_pda(authority, channel_index, program_id);
    let mut data = Vec::with_capacity(9);
    data.extend_from_slice(&instruction_discriminator("request_randomness_auto"));
    data.push(node_count);

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

// ── Streaming VRF Feed CPI helpers (v7) ─────────────────────────────────────

/// Build the `init_feed` CPI instruction.
///
/// Creates a streaming `RandomnessFeed` PDA bound to an existing `DiceChannel`.
/// The authority must own both the channel and the feed (same signer).
///
/// # Arguments
/// * `program_id`              — the DICE program ID
/// * `authority`               — developer wallet (signer, pays rent)
/// * `channel_index`           — index of the bound DiceChannel
/// * `feed_index`              — new feed index (unique per authority)
/// * `publish_interval_slots`  — target cadence between publishes (in slots, ~400ms each)
/// * `name`                    — 32-byte human-readable feed name (zero-padded)
pub fn init_feed_ix(
    program_id: &Pubkey,
    authority: &Pubkey,
    channel_index: u16,
    feed_index: u16,
    publish_interval_slots: u32,
    name: [u8; 32],
) -> Instruction {
    let (channel, _) = crate::pda::channel_pda(authority, channel_index, program_id);
    let (feed, _) = crate::pda::feed_pda(authority, feed_index, program_id);

    // Data: 8 disc + 2 feed_index + 4 publish_interval_slots + 32 name = 46 bytes
    let mut data = Vec::with_capacity(46);
    data.extend_from_slice(&instruction_discriminator("init_feed"));
    data.extend_from_slice(&feed_index.to_le_bytes());
    data.extend_from_slice(&publish_interval_slots.to_le_bytes());
    data.extend_from_slice(&name);

    Instruction {
        program_id: *program_id,
        accounts: vec![
            solana_sdk::instruction::AccountMeta::new(*authority, true),
            solana_sdk::instruction::AccountMeta::new_readonly(channel, false),
            solana_sdk::instruction::AccountMeta::new(feed, false),
            solana_sdk::instruction::AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
        ],
        data,
    }
}

/// Build the `publish_feed_value` CPI instruction.
///
/// Called by the feed's bound coordinator on a cadence. The provided
/// `randomness` and `round_id` must match the current state of the bound
/// `DiceChannel` (which must be in Finalized status).
pub fn publish_feed_value_ix(
    program_id: &Pubkey,
    coordinator: &Pubkey,
    feed_authority: &Pubkey,
    feed_index: u16,
    bound_channel_authority: &Pubkey,
    bound_channel_index: u16,
    randomness: [u8; 32],
    round_id: u64,
) -> Instruction {
    let (feed, _) = crate::pda::feed_pda(feed_authority, feed_index, program_id);
    let (channel, _) = crate::pda::channel_pda(bound_channel_authority, bound_channel_index, program_id);

    // Data: 8 disc + 32 randomness + 8 round_id = 48 bytes
    let mut data = Vec::with_capacity(48);
    data.extend_from_slice(&instruction_discriminator("publish_feed_value"));
    data.extend_from_slice(&randomness);
    data.extend_from_slice(&round_id.to_le_bytes());

    Instruction {
        program_id: *program_id,
        accounts: vec![
            solana_sdk::instruction::AccountMeta::new_readonly(*coordinator, true),
            solana_sdk::instruction::AccountMeta::new(feed, false),
            solana_sdk::instruction::AccountMeta::new_readonly(channel, false),
        ],
        data,
    }
}

/// Build the `close_feed` CPI instruction. Refunds rent to the authority.
pub fn close_feed_ix(
    program_id: &Pubkey,
    authority: &Pubkey,
    feed_index: u16,
) -> Instruction {
    let (feed, _) = crate::pda::feed_pda(authority, feed_index, program_id);

    let mut data = Vec::with_capacity(8);
    data.extend_from_slice(&instruction_discriminator("close_feed"));

    Instruction {
        program_id: *program_id,
        accounts: vec![
            solana_sdk::instruction::AccountMeta::new(*authority, true),
            solana_sdk::instruction::AccountMeta::new(feed, false),
        ],
        data,
    }
}

/// One snapshot of a feed's current state, decoded from raw account bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeedSnapshot {
    pub randomness: [u8; 32],
    pub sequence: u64,
    pub slot: u64,
    pub round_id: u64,
    pub status: u8,
}

/// Decode the current randomness value from the raw bytes of a `RandomnessFeed`
/// account.
///
/// This is the primary helper for subscriber programs: call it inside your
/// instruction handler after borrowing the feed account data, and use the
/// returned `randomness` directly — no callback, no per-request TX.
///
/// Returns `None` if the account is too short, has never been published to
/// (sequence == 0), or has a zeroed randomness value.
///
/// RandomnessFeed layout (relevant offsets):
/// ```text
/// [0..8]      discriminator
/// [8..40]     authority (32)
/// [40..72]    coordinator (32)
/// [72..104]   bound_channel (32)
/// [104..106]  feed_index (u16)
/// [106..138]  name (32)
/// [138..142]  publish_interval_slots (u32)
/// [142]       status (0=Active, 1=Closed)
/// [143..175]  current_randomness [u8; 32]
/// [175..183]  current_sequence (u64)
/// [183..191]  current_slot (u64)
/// [191..199]  current_round_id (u64)
/// ```
pub fn decode_feed_current_randomness(account_data: &[u8]) -> Option<FeedSnapshot> {
    if account_data.len() < 199 {
        return None;
    }
    let status = account_data[142];
    let randomness: [u8; 32] = account_data[143..175].try_into().ok()?;
    let sequence = u64::from_le_bytes(account_data[175..183].try_into().ok()?);
    let slot = u64::from_le_bytes(account_data[183..191].try_into().ok()?);
    let round_id = u64::from_le_bytes(account_data[191..199].try_into().ok()?);

    // Sequence 0 = never published, still at genesis
    if sequence == 0 || randomness == [0u8; 32] {
        return None;
    }

    Some(FeedSnapshot {
        randomness,
        sequence,
        slot,
        round_id,
        status,
    })
}

// ── Universal Payout (NodeVault) CPI helpers (v7) ───────────────────────────

/// Build the `register_node_vault` CPI instruction.
///
/// First-time binding of a payout wallet to a device. The binding must be
/// ECDSA-signed by the device's hardware key; the signature is verified
/// on-chain via `secp256k1_recover`. The `payer` is whoever funds the vault
/// PDA rent (typically the coordinator).
pub fn register_node_vault_ix(
    program_id: &Pubkey,
    payer: &Pubkey,
    device_pubkey: [u8; 33],
    payout_wallet: &Pubkey,
    timestamp: i64,
    nonce: [u8; 32],
    signature: [u8; 64],
) -> Instruction {
    let (vault, _) = crate::pda::node_vault_pda(&device_pubkey, program_id);

    // Data: 8 disc + 33 device_pubkey + 32 payout_wallet + 8 timestamp
    //       + 32 nonce + 64 signature = 177 bytes
    let mut data = Vec::with_capacity(177);
    data.extend_from_slice(&instruction_discriminator("register_node_vault"));
    data.extend_from_slice(&device_pubkey);
    data.extend_from_slice(payout_wallet.as_ref());
    data.extend_from_slice(&timestamp.to_le_bytes());
    data.extend_from_slice(&nonce);
    data.extend_from_slice(&signature);

    Instruction {
        program_id: *program_id,
        accounts: vec![
            solana_sdk::instruction::AccountMeta::new(*payer, true),
            solana_sdk::instruction::AccountMeta::new(vault, false),
            solana_sdk::instruction::AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
        ],
        data,
    }
}

/// Build the `rotate_payout_wallet` CPI instruction.
///
/// Dual-signed: the current payout wallet must sign the TX, AND the device
/// must produce a fresh ECDSA binding over the new wallet. A cooldown
/// prevents rapid rotation.
pub fn rotate_payout_wallet_ix(
    program_id: &Pubkey,
    current_payout_wallet: &Pubkey,
    device_pubkey: &[u8; 33],
    new_payout_wallet: &Pubkey,
    timestamp: i64,
    nonce: [u8; 32],
    device_signature: [u8; 64],
) -> Instruction {
    let (vault, _) = crate::pda::node_vault_pda(device_pubkey, program_id);

    // Data: 8 disc + 32 new_payout_wallet + 8 timestamp + 32 nonce + 64 sig = 144
    let mut data = Vec::with_capacity(144);
    data.extend_from_slice(&instruction_discriminator("rotate_payout_wallet"));
    data.extend_from_slice(new_payout_wallet.as_ref());
    data.extend_from_slice(&timestamp.to_le_bytes());
    data.extend_from_slice(&nonce);
    data.extend_from_slice(&device_signature);

    Instruction {
        program_id: *program_id,
        accounts: vec![
            solana_sdk::instruction::AccountMeta::new_readonly(*current_payout_wallet, true),
            solana_sdk::instruction::AccountMeta::new(vault, false),
        ],
        data,
    }
}

/// Build the `withdraw_from_vault` CPI instruction.
///
/// Only the operator's payout wallet can sign. The vault must be `Bound`.
/// Amount must be <= (total_earned - total_withdrawn), and the withdrawal
/// must leave enough lamports in the vault to remain rent-exempt.
pub fn withdraw_from_vault_ix(
    program_id: &Pubkey,
    payout_wallet: &Pubkey,
    device_pubkey: &[u8; 33],
    amount: u64,
) -> Instruction {
    let (vault, _) = crate::pda::node_vault_pda(device_pubkey, program_id);

    // Data: 8 disc + 8 amount = 16 bytes
    let mut data = Vec::with_capacity(16);
    data.extend_from_slice(&instruction_discriminator("withdraw_from_vault"));
    data.extend_from_slice(&amount.to_le_bytes());

    Instruction {
        program_id: *program_id,
        accounts: vec![
            solana_sdk::instruction::AccountMeta::new(*payout_wallet, true),
            solana_sdk::instruction::AccountMeta::new(vault, false),
        ],
        data,
    }
}

/// Build the `claim_rewards_v2` CPI instruction.
///
/// Distributes fee rewards for a finalized v2 channel round:
///   - 70% split across contributing nodes' NodeVaults
///   - 20% to treasury
///   - 10% to reserve
///
/// Transitions the channel `Finalized -> Idle`. Only the channel's bound
/// coordinator can sign. Pass one NodeVault PDA per contributing device
/// via `remaining_vault_pdas`, in the same order as the channel's
/// `device_pubkeys[0..reveals_received]`.
pub fn claim_rewards_v2_ix(
    program_id: &Pubkey,
    coordinator: &Pubkey,
    channel_authority: &Pubkey,
    channel_index: u16,
    treasury: &Pubkey,
    reserve: &Pubkey,
    remaining_vault_pdas: &[Pubkey],
) -> Instruction {
    let (channel, _) = crate::pda::channel_pda(channel_authority, channel_index, program_id);

    // Data: 8 disc only
    let mut data = Vec::with_capacity(8);
    data.extend_from_slice(&instruction_discriminator("claim_rewards_v2"));

    let mut accounts = vec![
        solana_sdk::instruction::AccountMeta::new_readonly(*coordinator, true),
        solana_sdk::instruction::AccountMeta::new(channel, false),
        solana_sdk::instruction::AccountMeta::new(*treasury, false),
        solana_sdk::instruction::AccountMeta::new(*reserve, false),
    ];
    for vault in remaining_vault_pdas {
        accounts.push(solana_sdk::instruction::AccountMeta::new(*vault, false));
    }

    Instruction {
        program_id: *program_id,
        accounts,
        data,
    }
}

/// Off-chain snapshot of a NodeVault decoded from raw account bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeVaultSnapshot {
    pub device_pubkey: [u8; 33],
    pub payout_wallet: Pubkey,
    pub total_earned: u64,
    pub total_withdrawn: u64,
    pub status: u8,
}

/// Decode the current state of a `NodeVault` account.
///
/// Returns `None` if the buffer is too short. Returns a snapshot otherwise,
/// including unbound (zero payout wallet) vaults — callers can inspect
/// `status` to distinguish states.
///
/// Layout offsets (must match `programs/dice/src/state/node_vault.rs`):
/// ```text
/// [0..8]     discriminator
/// [8..41]    device_pubkey [u8; 33]
/// [41..73]   payout_wallet  Pubkey (32)
/// [73..81]   binding_slot   u64
/// [81..145]  binding_signature [u8; 64]
/// [145..177] binding_nonce [u8; 32]
/// [177..185] binding_timestamp i64
/// [185]      status u8
/// [186..194] created_slot u64
/// [194..202] total_earned u64
/// [202..210] total_withdrawn u64
/// [210..218] last_credit_slot u64
/// [218..282] service_counts [u64; 8]  (8 * 8 = 64)
/// [282..346] reserved [u8; 64]
/// ```
pub fn decode_node_vault(account_data: &[u8]) -> Option<NodeVaultSnapshot> {
    if account_data.len() < 210 {
        return None;
    }
    let device_pubkey: [u8; 33] = account_data[8..41].try_into().ok()?;
    let payout_wallet = Pubkey::new_from_array(account_data[41..73].try_into().ok()?);
    let status = account_data[185];
    let total_earned = u64::from_le_bytes(account_data[194..202].try_into().ok()?);
    let total_withdrawn = u64::from_le_bytes(account_data[202..210].try_into().ok()?);

    Some(NodeVaultSnapshot {
        device_pubkey,
        payout_wallet,
        total_earned,
        total_withdrawn,
        status,
    })
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

    // ── v2.0 instruction data layout tests ──────────────────────────────────

    #[test]
    fn test_init_channel_ix_data_layout() {
        let program_id = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let callback = Pubkey::new_unique();
        let ix = init_channel_ix(&program_id, &authority, 7, 10, &callback);

        // 8 discriminator + 2 channel_index(u16 LE) + 1 max_nodes + 32 callback_program_id = 43 bytes
        assert_eq!(ix.data.len(), 43, "init_channel data should be 43 bytes");
        // First 8 bytes: discriminator
        let disc = instruction_discriminator("init_channel");
        assert_eq!(&ix.data[0..8], &disc);
        // Bytes 8..10: channel_index in LE
        assert_eq!(&ix.data[8..10], &7u16.to_le_bytes());
        // Byte 10: max_nodes
        assert_eq!(ix.data[10], 10);
        // Bytes 11..43: callback_program_id
        assert_eq!(&ix.data[11..43], callback.as_ref());
    }

    #[test]
    fn test_fund_channel_ix_data_layout() {
        let program_id = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let amount: u64 = 2_000_000; // 0.002 SOL in lamports
        let ix = fund_channel_ix(&program_id, &authority, 0, amount);

        // 8 discriminator + 8 amount(u64 LE) = 16 bytes
        assert_eq!(ix.data.len(), 16, "fund_channel data should be 16 bytes");
        let disc = instruction_discriminator("fund_channel");
        assert_eq!(&ix.data[0..8], &disc);
        assert_eq!(&ix.data[8..16], &amount.to_le_bytes());
    }

    #[test]
    fn test_request_randomness_v2_ix_data_layout() {
        let program_id = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let ix = request_randomness_v2_ix(&program_id, &authority, 3, 5);

        // 8 discriminator + 1 node_count = 9 bytes
        assert_eq!(ix.data.len(), 9, "request_randomness_v2 data should be 9 bytes");
        let disc = instruction_discriminator("request_randomness_v2");
        assert_eq!(&ix.data[0..8], &disc);
        assert_eq!(ix.data[8], 5); // node_count
    }

    #[test]
    fn test_request_randomness_auto_ix_data_layout() {
        let program_id = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let ix = request_randomness_auto_ix(&program_id, &authority, 1, 4);

        // 8 discriminator + 1 node_count = 9 bytes
        assert_eq!(ix.data.len(), 9, "request_randomness_auto data should be 9 bytes");
        let disc = instruction_discriminator("request_randomness_auto");
        assert_eq!(&ix.data[0..8], &disc);
        assert_eq!(ix.data[8], 4); // node_count
    }

    // ── decode_channel_randomness tests ─────────────────────────────────────

    #[test]
    fn test_decode_channel_randomness_too_short() {
        // Anything less than 183 bytes should return None
        assert_eq!(decode_channel_randomness(&[0u8; 0]), None);
        assert_eq!(decode_channel_randomness(&[0u8; 42]), None);
        assert_eq!(decode_channel_randomness(&[0u8; 182]), None);
    }

    #[test]
    fn test_decode_channel_randomness_not_finalized() {
        // Build a 183-byte buffer with non-zero randomness at [151..183]
        let mut data = vec![0u8; 183];
        data[151..183].fill(0xBB);

        // status=1 (Pending) should return None
        data[75] = 1;
        assert_eq!(decode_channel_randomness(&data), None);

        // status=2 (CommitPhase) should return None
        data[75] = 2;
        assert_eq!(decode_channel_randomness(&data), None);

        // status=3 (RevealPhase) should return None
        data[75] = 3;
        assert_eq!(decode_channel_randomness(&data), None);

        // status=5 (Failed) should return None
        data[75] = 5;
        assert_eq!(decode_channel_randomness(&data), None);
    }

    #[test]
    fn test_decode_channel_randomness_zeroed() {
        // status=4 (Finalized) but randomness is all zeros => None
        let mut data = vec![0u8; 183];
        data[75] = 4; // Finalized
        assert_eq!(decode_channel_randomness(&data), None);
    }

    #[test]
    fn test_decode_channel_randomness_valid() {
        let mut data = vec![0u8; 183];
        data[75] = 4; // status = Finalized
        data[151..183].fill(0xCD);
        let result = decode_channel_randomness(&data);
        assert!(result.is_some(), "Finalized with non-zero randomness should return Some");
        assert_eq!(result.unwrap(), [0xCDu8; 32]);
    }

    #[test]
    fn test_decode_channel_randomness_idle_with_result() {
        // status=0 (Idle) means callback delivered, result still readable
        let mut data = vec![0u8; 183];
        data[75] = 0; // Idle
        data[151..183].fill(0xEF);
        let result = decode_channel_randomness(&data);
        assert!(result.is_some(), "Idle with non-zero randomness should return Some");
        assert_eq!(result.unwrap(), [0xEFu8; 32]);
    }

    // ── discriminator uniqueness tests ──────────────────────────────────────

    #[test]
    fn test_all_discriminators_are_8_bytes() {
        let names = [
            "request_randomness",
            "finalize_randomness",
            "dice_callback",
            "init_channel",
            "fund_channel",
            "request_randomness_v2",
            "request_randomness_auto",
            "init_feed",
            "publish_feed_value",
            "close_feed",
            "register_node_vault",
            "rotate_payout_wallet",
            "withdraw_from_vault",
            "claim_rewards_v2",
        ];
        for name in names {
            let disc = instruction_discriminator(name);
            assert_eq!(disc.len(), 8, "discriminator for '{}' must be 8 bytes", name);
        }
    }

    #[test]
    fn test_discriminators_unique() {
        let names = [
            "request_randomness",
            "finalize_randomness",
            "dice_callback",
            "init_channel",
            "fund_channel",
            "request_randomness_v2",
            "request_randomness_auto",
            "init_feed",
            "publish_feed_value",
            "close_feed",
            "register_node_vault",
            "rotate_payout_wallet",
            "withdraw_from_vault",
            "claim_rewards_v2",
        ];
        let discs: Vec<[u8; 8]> = names.iter().map(|n| instruction_discriminator(n)).collect();
        for i in 0..discs.len() {
            for j in (i + 1)..discs.len() {
                assert_ne!(
                    discs[i], discs[j],
                    "discriminators for '{}' and '{}' must not collide",
                    names[i], names[j]
                );
            }
        }
    }

    // ── Streaming VRF feed tests ────────────────────────────────────────────

    #[test]
    fn test_init_feed_ix_data_layout() {
        let program_id = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let mut name = [0u8; 32];
        name[..5].copy_from_slice(b"dice1");

        let ix = init_feed_ix(&program_id, &authority, 0, 7, 10, name);

        // 8 disc + 2 feed_index + 4 publish_interval_slots + 32 name = 46 bytes
        assert_eq!(ix.data.len(), 46);
        let disc = instruction_discriminator("init_feed");
        assert_eq!(&ix.data[0..8], &disc);
        assert_eq!(&ix.data[8..10], &7u16.to_le_bytes());
        assert_eq!(&ix.data[10..14], &10u32.to_le_bytes());
        assert_eq!(&ix.data[14..46], &name[..]);

        // Account metas: authority (signer, mut), bound_channel (ro), feed (mut), system
        assert_eq!(ix.accounts.len(), 4);
        assert!(ix.accounts[0].is_signer);
        assert!(ix.accounts[0].is_writable);
        assert!(!ix.accounts[1].is_writable, "bound_channel is readonly");
        assert!(ix.accounts[2].is_writable, "feed is init-writable");
    }

    #[test]
    fn test_publish_feed_value_ix_data_layout() {
        let program_id = Pubkey::new_unique();
        let coordinator = Pubkey::new_unique();
        let feed_authority = Pubkey::new_unique();
        let channel_authority = Pubkey::new_unique();
        let randomness = [0xAB; 32];
        let round_id: u64 = 12345;

        let ix = publish_feed_value_ix(
            &program_id,
            &coordinator,
            &feed_authority,
            3,
            &channel_authority,
            1,
            randomness,
            round_id,
        );

        // 8 disc + 32 randomness + 8 round_id = 48 bytes
        assert_eq!(ix.data.len(), 48);
        let disc = instruction_discriminator("publish_feed_value");
        assert_eq!(&ix.data[0..8], &disc);
        assert_eq!(&ix.data[8..40], &randomness[..]);
        assert_eq!(&ix.data[40..48], &round_id.to_le_bytes());

        // Account metas: coordinator (signer, ro), feed (mut), channel (ro)
        assert_eq!(ix.accounts.len(), 3);
        assert!(ix.accounts[0].is_signer);
        assert!(!ix.accounts[0].is_writable, "coordinator is signer-only");
        assert!(ix.accounts[1].is_writable, "feed must be writable");
        assert!(!ix.accounts[2].is_writable, "channel is readonly");
    }

    #[test]
    fn test_close_feed_ix_data_layout() {
        let program_id = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let ix = close_feed_ix(&program_id, &authority, 0);

        // 8 disc only
        assert_eq!(ix.data.len(), 8);
        let disc = instruction_discriminator("close_feed");
        assert_eq!(&ix.data[0..8], &disc);

        assert_eq!(ix.accounts.len(), 2);
        assert!(ix.accounts[0].is_signer);
        assert!(ix.accounts[1].is_writable, "feed must be writable for close");
    }

    // ── decode_feed_current_randomness tests ────────────────────────────────
    //
    // These tests encode the on-chain RandomnessFeed layout as a byte buffer
    // and verify the SDK reader extracts the same values. If the on-chain
    // struct field order ever changes, these tests will catch the drift.

    fn build_feed_bytes(
        status: u8,
        randomness: [u8; 32],
        sequence: u64,
        slot: u64,
        round_id: u64,
    ) -> Vec<u8> {
        // Layout matches programs/dice/src/state/randomness_feed.rs field order:
        //   disc(8) + authority(32) + coordinator(32) + bound_channel(32)
        //   + feed_index(2) + name(32) + publish_interval_slots(4)
        //   + status(1)
        //   + current_randomness(32) + current_sequence(8) + current_slot(8) + current_round_id(8)
        //   + prev_randomness(32) + prev_sequence(8)
        //   + history_head(1) + 16 * 48 history entries
        //   + total_published(8) + created_slot(8)
        let mut data = vec![0u8; 1024];
        // [0..8] discriminator (leave zero, not checked)
        // [8..40] authority (zero)
        // [40..72] coordinator (zero)
        // [72..104] bound_channel (zero)
        // [104..106] feed_index (zero)
        // [106..138] name (zero)
        // [138..142] publish_interval_slots (zero)
        data[142] = status;
        data[143..175].copy_from_slice(&randomness);
        data[175..183].copy_from_slice(&sequence.to_le_bytes());
        data[183..191].copy_from_slice(&slot.to_le_bytes());
        data[191..199].copy_from_slice(&round_id.to_le_bytes());
        data
    }

    #[test]
    fn test_decode_feed_too_short() {
        assert!(decode_feed_current_randomness(&[0u8; 0]).is_none());
        assert!(decode_feed_current_randomness(&[0u8; 198]).is_none());
    }

    #[test]
    fn test_decode_feed_not_yet_published() {
        // Sequence 0 + zero randomness → None
        let data = build_feed_bytes(0, [0u8; 32], 0, 0, 0);
        assert!(decode_feed_current_randomness(&data).is_none());
    }

    #[test]
    fn test_decode_feed_zero_randomness_rejected() {
        // Sequence > 0 but randomness is zero — still None (defensive)
        let data = build_feed_bytes(0, [0u8; 32], 5, 100, 42);
        assert!(decode_feed_current_randomness(&data).is_none());
    }

    #[test]
    fn test_decode_feed_valid_snapshot() {
        let r = [0xCD; 32];
        let data = build_feed_bytes(0, r, 42, 5000, 100);
        let snap = decode_feed_current_randomness(&data).expect("should decode");
        assert_eq!(snap.randomness, r);
        assert_eq!(snap.sequence, 42);
        assert_eq!(snap.slot, 5000);
        assert_eq!(snap.round_id, 100);
        assert_eq!(snap.status, 0); // Active
    }

    #[test]
    fn test_decode_feed_closed_status() {
        let r = [0xEE; 32];
        let data = build_feed_bytes(1, r, 99, 9000, 77);
        let snap = decode_feed_current_randomness(&data).expect("closed feeds still decode");
        assert_eq!(snap.status, 1); // Closed
        assert_eq!(snap.randomness, r);
    }

    #[test]
    fn test_feed_snapshot_equality() {
        let a = FeedSnapshot {
            randomness: [1u8; 32],
            sequence: 1,
            slot: 1,
            round_id: 1,
            status: 0,
        };
        let b = a;
        assert_eq!(a, b);
    }

    // ── Universal Payout (NodeVault) tests ──────────────────────────────────

    /// Build a realistic-looking 33-byte compressed secp256k1 pubkey from a
    /// randomly-chosen Ed25519 Pubkey. All-`0xAA` byte patterns can fail to
    /// produce a valid PDA bump, so we need actually-random seed bytes for
    /// PDA-derivation tests.
    fn test_device_pubkey() -> [u8; 33] {
        let mut out = [0u8; 33];
        out[0] = 0x02; // even-y compressed secp256k1 prefix
        out[1..33].copy_from_slice(&Pubkey::new_unique().to_bytes());
        out
    }

    #[test]
    fn test_register_node_vault_ix_data_layout() {
        let program_id = Pubkey::new_unique();
        let payer = Pubkey::new_unique();
        let payout_wallet = Pubkey::new_unique();
        let device_pubkey = test_device_pubkey();
        let timestamp: i64 = 1_700_000_000;
        let nonce = [0xAB; 32];
        let signature = [0xCD; 64];

        let ix = register_node_vault_ix(
            &program_id,
            &payer,
            device_pubkey,
            &payout_wallet,
            timestamp,
            nonce,
            signature,
        );

        // 8 disc + 33 device_pubkey + 32 payout_wallet + 8 ts + 32 nonce + 64 sig = 177
        assert_eq!(ix.data.len(), 177);
        let disc = instruction_discriminator("register_node_vault");
        assert_eq!(&ix.data[0..8], &disc);
        assert_eq!(&ix.data[8..41], &device_pubkey[..]);
        assert_eq!(&ix.data[41..73], payout_wallet.as_ref());
        assert_eq!(&ix.data[73..81], &timestamp.to_le_bytes());
        assert_eq!(&ix.data[81..113], &nonce[..]);
        assert_eq!(&ix.data[113..177], &signature[..]);

        // Metas: payer (signer/mut), vault (mut), system
        assert_eq!(ix.accounts.len(), 3);
        assert!(ix.accounts[0].is_signer);
        assert!(ix.accounts[0].is_writable);
        assert!(ix.accounts[1].is_writable);
    }

    #[test]
    fn test_rotate_payout_wallet_ix_data_layout() {
        let program_id = Pubkey::new_unique();
        let current_wallet = Pubkey::new_unique();
        let device_pubkey = test_device_pubkey();
        let new_wallet = Pubkey::new_unique();
        let timestamp: i64 = 1_800_000_000;
        let nonce = [0xEE; 32];
        let device_sig = [0x55; 64];

        let ix = rotate_payout_wallet_ix(
            &program_id,
            &current_wallet,
            &device_pubkey,
            &new_wallet,
            timestamp,
            nonce,
            device_sig,
        );

        // 8 disc + 32 new_wallet + 8 ts + 32 nonce + 64 sig = 144
        assert_eq!(ix.data.len(), 144);
        let disc = instruction_discriminator("rotate_payout_wallet");
        assert_eq!(&ix.data[0..8], &disc);
        assert_eq!(&ix.data[8..40], new_wallet.as_ref());
        assert_eq!(&ix.data[40..48], &timestamp.to_le_bytes());
        assert_eq!(&ix.data[48..80], &nonce[..]);
        assert_eq!(&ix.data[80..144], &device_sig[..]);

        assert_eq!(ix.accounts.len(), 2);
        assert!(ix.accounts[0].is_signer);
        assert!(!ix.accounts[0].is_writable, "current wallet is signer-only");
        assert!(ix.accounts[1].is_writable, "vault must be writable");
    }

    #[test]
    fn test_withdraw_from_vault_ix_data_layout() {
        let program_id = Pubkey::new_unique();
        let payout_wallet = Pubkey::new_unique();
        let device_pubkey = test_device_pubkey();
        let amount: u64 = 1_000_000;

        let ix = withdraw_from_vault_ix(&program_id, &payout_wallet, &device_pubkey, amount);

        // 8 disc + 8 amount = 16
        assert_eq!(ix.data.len(), 16);
        let disc = instruction_discriminator("withdraw_from_vault");
        assert_eq!(&ix.data[0..8], &disc);
        assert_eq!(&ix.data[8..16], &amount.to_le_bytes());

        assert_eq!(ix.accounts.len(), 2);
        assert!(ix.accounts[0].is_signer);
        assert!(ix.accounts[0].is_writable, "payout wallet needs mut for lamport receipt");
        assert!(ix.accounts[1].is_writable, "vault must be writable");
    }

    #[test]
    fn test_claim_rewards_v2_ix_data_layout_four_nodes() {
        let program_id = Pubkey::new_unique();
        let coordinator = Pubkey::new_unique();
        let channel_authority = Pubkey::new_unique();
        let treasury = Pubkey::new_unique();
        let reserve = Pubkey::new_unique();
        let vaults: Vec<Pubkey> = (0..4).map(|_| Pubkey::new_unique()).collect();

        let ix = claim_rewards_v2_ix(
            &program_id,
            &coordinator,
            &channel_authority,
            0,
            &treasury,
            &reserve,
            &vaults,
        );

        // 8 disc only
        assert_eq!(ix.data.len(), 8);
        let disc = instruction_discriminator("claim_rewards_v2");
        assert_eq!(&ix.data[0..8], &disc);

        // 4 fixed + 4 vaults = 8 accounts
        assert_eq!(ix.accounts.len(), 8);
        assert!(ix.accounts[0].is_signer);
        assert!(!ix.accounts[0].is_writable, "coordinator is signer-only");
        assert!(ix.accounts[1].is_writable, "channel must be writable");
        assert!(ix.accounts[2].is_writable, "treasury writable");
        assert!(ix.accounts[3].is_writable, "reserve writable");
        for i in 4..8 {
            assert!(ix.accounts[i].is_writable, "vault {i} writable");
            assert!(!ix.accounts[i].is_signer, "vault {i} not a signer");
        }
    }

    #[test]
    fn test_claim_rewards_v2_ix_zero_vaults() {
        // 0-vault case is valid at the byte level — the on-chain handler
        // rejects it via `require!(num_contributors > 0)`. The SDK should
        // still accept it so test suites can cover the rejection path.
        let ix = claim_rewards_v2_ix(
            &Pubkey::new_unique(),
            &Pubkey::new_unique(),
            &Pubkey::new_unique(),
            0,
            &Pubkey::new_unique(),
            &Pubkey::new_unique(),
            &[],
        );
        assert_eq!(ix.accounts.len(), 4, "4 fixed + 0 vaults");
    }

    // ── decode_node_vault tests ──────────────────────────────────────────────
    //
    // The byte offsets MUST stay in lockstep with
    // programs/dice/src/state/node_vault.rs. If the on-chain struct ever
    // reorders a field, these tests fail loudly.

    fn build_node_vault_bytes(
        device_pubkey: [u8; 33],
        payout_wallet: Pubkey,
        status: u8,
        total_earned: u64,
        total_withdrawn: u64,
    ) -> Vec<u8> {
        let mut data = vec![0u8; 346]; // NodeVault::space()
        // [0..8] discriminator (zero)
        data[8..41].copy_from_slice(&device_pubkey);
        data[41..73].copy_from_slice(payout_wallet.as_ref());
        // [73..81] binding_slot (zero)
        // [81..145] binding_signature (zero)
        // [145..177] binding_nonce (zero)
        // [177..185] binding_timestamp (zero)
        data[185] = status;
        // [186..194] created_slot (zero)
        data[194..202].copy_from_slice(&total_earned.to_le_bytes());
        data[202..210].copy_from_slice(&total_withdrawn.to_le_bytes());
        data
    }

    #[test]
    fn test_decode_node_vault_too_short() {
        assert!(decode_node_vault(&[0u8; 200]).is_none());
    }

    #[test]
    fn test_decode_node_vault_unbound_empty() {
        let device_pubkey = test_device_pubkey();
        let data = build_node_vault_bytes(device_pubkey, Pubkey::default(), 0, 0, 0);
        let snap = decode_node_vault(&data).expect("should decode");
        assert_eq!(snap.device_pubkey, device_pubkey);
        assert_eq!(snap.payout_wallet, Pubkey::default());
        assert_eq!(snap.status, 0); // Unbound
        assert_eq!(snap.total_earned, 0);
        assert_eq!(snap.total_withdrawn, 0);
    }

    #[test]
    fn test_decode_node_vault_bound_with_earnings() {
        let device_pubkey = test_device_pubkey();
        let wallet = Pubkey::new_unique();
        let data = build_node_vault_bytes(device_pubkey, wallet, 1, 5_000_000, 1_200_000);
        let snap = decode_node_vault(&data).expect("should decode");
        assert_eq!(snap.payout_wallet, wallet);
        assert_eq!(snap.status, 1); // Bound
        assert_eq!(snap.total_earned, 5_000_000);
        assert_eq!(snap.total_withdrawn, 1_200_000);
        // Available balance = earned - withdrawn = 3_800_000 (caller computes)
    }

    #[test]
    fn test_node_vault_pda_deterministic() {
        let device = test_device_pubkey();
        let program_id = Pubkey::new_unique();
        let (a, bump_a) = crate::pda::node_vault_pda(&device, &program_id);
        let (b, bump_b) = crate::pda::node_vault_pda(&device, &program_id);
        assert_eq!(a, b);
        assert_eq!(bump_a, bump_b);
    }

    #[test]
    fn test_node_vault_pda_differs_from_feed_and_channel() {
        // feed + channel + node_vault should all produce distinct PDAs for
        // the same program_id — different seed prefixes guarantee it.
        let program_id = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let device = test_device_pubkey();

        let (feed, _) = crate::pda::feed_pda(&authority, 0, &program_id);
        let (channel, _) = crate::pda::channel_pda(&authority, 0, &program_id);
        let (vault, _) = crate::pda::node_vault_pda(&device, &program_id);
        assert_ne!(feed, channel);
        assert_ne!(feed, vault);
        assert_ne!(channel, vault);
    }
}
