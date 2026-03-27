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

    #[msg("Invalid node count: must be between 5 and 7")]
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
}
