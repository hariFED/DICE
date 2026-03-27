use thiserror::Error;

#[derive(Debug, Error)]
pub enum DiceVrfError {
    #[error("Request not yet finalized (status: {0})")]
    NotFinalized(String),

    #[error("Request timed out")]
    Timeout,

    #[error("Insufficient escrow balance: need {needed} lamports, have {available}")]
    InsufficientEscrow { needed: u64, available: u64 },

    #[error("Program error: {0}")]
    ProgramError(String),

    #[error("RPC error: {0}")]
    RpcError(String),
}
