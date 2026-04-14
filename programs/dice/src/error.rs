use anchor_lang::prelude::*;

#[error_code]
pub enum DiceError {
    #[msg("Insufficient nodes: minimum 4 required")]
    InsufficientNodes,

    #[msg("Round has timed out")]
    RoundTimedOut,

    #[msg("Invalid ECDSA signature")]
    InvalidSignature,

    #[msg("Node has already committed for this round")]
    AlreadyCommitted,

    #[msg("Reveal mismatch: hash(entropy) does not match commit")]
    RevealMismatch,

    #[msg("Escrow has insufficient funds")]
    EscrowInsufficient,

    #[msg("Round is not yet complete")]
    RoundNotComplete,

    #[msg("Node is not authorized for this round")]
    UnauthorizedNode,

    #[msg("Invalid node count: must be between 4 and 50")]
    InvalidNodeCount,

    #[msg("Round has already been finalized")]
    RoundAlreadyFinalized,

    #[msg("Callback program missing from remaining accounts")]
    CallbackProgramMissing,

    #[msg("Callback program ID does not match request")]
    CallbackProgramMismatch,

    #[msg("CPI callback to developer program failed")]
    CallbackFailed,

    #[msg("Device ID does not match SHA-256(device_pubkey)")]
    InvalidDeviceId,

    #[msg("Signer is not the authorized coordinator for this channel")]
    UnauthorizedCoordinator,

    #[msg("Node has already revealed for this round")]
    AlreadyRevealed,

    // ── Streaming VRF feed errors ────────────────────────────────────
    #[msg("Feed publish attempted before the configured interval has elapsed")]
    FeedPublishTooSoon,

    #[msg("Caller is not the coordinator bound to this feed")]
    FeedWrongCoordinator,

    #[msg("Publish interval must be between MIN_PUBLISH_INTERVAL_SLOTS and MAX_PUBLISH_INTERVAL_SLOTS")]
    FeedInvalidInterval,

    #[msg("Feed is not active — it was closed or paused")]
    FeedNotActive,

    #[msg("Bound channel does not match the feed's configured bound_channel")]
    FeedChannelMismatch,

    #[msg("Bound channel is not in Finalized status — cannot publish its randomness")]
    FeedChannelNotFinalized,

    #[msg("Bound channel round_id does not match provided round_id")]
    FeedRoundIdMismatch,

    #[msg("Bound channel randomness does not match provided randomness")]
    FeedRandomnessMismatch,

    // ── Universal payout (NodeVault) errors ──────────────────────────
    #[msg("Vault is already bound to a payout wallet")]
    VaultAlreadyBound,

    #[msg("Vault has no payout wallet bound yet")]
    VaultNotBound,

    #[msg("Signer is not the vault's current payout wallet")]
    VaultWrongPayoutWallet,

    #[msg("Withdraw amount exceeds vault credited balance")]
    VaultInsufficientBalance,

    #[msg("Withdraw amount must be greater than zero")]
    VaultZeroAmount,

    #[msg("Payout wallet rotation attempted before cooldown elapsed")]
    VaultRotationCooldown,

    #[msg("Payout-binding signature failed secp256k1 verification")]
    VaultBindingSignatureInvalid,

    #[msg("Credit amount would overflow vault totals")]
    VaultCreditOverflow,

    #[msg("Unknown service ID passed to credit_vault")]
    VaultUnknownServiceId,

    #[msg("Vault is frozen — credits and withdrawals are paused")]
    VaultFrozen,
}
