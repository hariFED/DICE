use clap::Parser;
use std::path::PathBuf;

/// DICE Coordinator configuration — parsed from CLI flags and environment variables.
#[derive(Debug, Clone, Parser)]
#[command(
    name = "dice-coordinator",
    about = "DICE hardware-backed VRF oracle coordinator",
    version
)]
pub struct Config {
    /// mTLS WebSocket port for ESP32-S3 node connections
    #[arg(long, env = "DICE_WS_PORT", default_value_t = 8443)]
    pub ws_port: u16,

    /// REST API port
    #[arg(long, env = "DICE_API_PORT", default_value_t = 8080)]
    pub api_port: u16,

    /// Prometheus metrics port
    #[arg(long, env = "DICE_METRICS_PORT", default_value_t = 9090)]
    pub metrics_port: u16,

    /// Run in simulation mode: plain WebSocket, no DB required, no Solana RPC calls.
    /// Use with mock-firmware-node --insecure for local end-to-end testing.
    #[arg(long, env = "DICE_SIMULATION", default_value_t = false)]
    pub simulation: bool,

    /// Enable mTLS WebSocket even in simulation mode.
    /// Requires --tls-cert-path, --tls-key-path, and --ca-cert-path.
    #[arg(long, env = "DICE_TLS", default_value_t = false)]
    pub tls: bool,

    /// API key for protecting coordinator endpoints (Bearer token).
    /// If set, all endpoints except /health and /metrics require
    /// Authorization: Bearer <key>. If unset, all endpoints are public.
    #[arg(long, env = "DICE_API_KEY")]
    pub api_key: Option<String>,

    /// Maximum requests per second for the /simulate endpoint.
    #[arg(long, env = "DICE_RATE_LIMIT_RPS", default_value_t = 10)]
    pub rate_limit_rps: u32,

    /// PostgreSQL connection URL (required unless --simulation)
    #[arg(long, env = "DATABASE_URL", default_value = "postgres://dice:dice@localhost/dice")]
    pub database_url: String,

    /// Solana RPC endpoint URL (unused in simulation mode)
    #[arg(long, env = "SOLANA_RPC_URL", default_value = "https://api.devnet.solana.com")]
    pub solana_rpc_url: String,

    /// Path to coordinator Solana keypair JSON file (unused in simulation mode)
    #[arg(long, env = "DICE_KEYPAIR_PATH", default_value = "coordinator-keypair.json")]
    pub coordinator_keypair_path: PathBuf,

    /// Path to coordinator TLS certificate (PEM; unused in simulation mode)
    #[arg(long, env = "DICE_TLS_CERT_PATH", default_value = "certs/coordinator.crt")]
    pub tls_cert_path: PathBuf,

    /// Path to coordinator TLS private key (PEM; unused in simulation mode)
    #[arg(long, env = "DICE_TLS_KEY_PATH", default_value = "certs/coordinator.key")]
    pub tls_key_path: PathBuf,

    /// Path to CA certificate for verifying device mTLS certs (PEM; unused in simulation mode)
    #[arg(long, env = "DICE_CA_CERT_PATH", default_value = "certs/ca.crt")]
    pub ca_cert_path: PathBuf,

    /// Minimum number of nodes required to run a round
    #[arg(long, env = "DICE_MIN_NODES", default_value_t = 4)]
    pub min_nodes: u8,

    /// Maximum number of nodes to select per round
    #[arg(long, env = "DICE_MAX_NODES", default_value_t = 7)]
    pub max_nodes: u8,

    /// Seconds before the commit phase times out
    #[arg(long, env = "DICE_COMMIT_TIMEOUT_SECS", default_value_t = 60)]
    pub commit_timeout_secs: u64,

    /// Seconds before the reveal phase times out
    #[arg(long, env = "DICE_REVEAL_TIMEOUT_SECS", default_value_t = 60)]
    pub reveal_timeout_secs: u64,

    /// Protocol treasury wallet — receives 20% of every fee.
    /// Must be a base58 Solana pubkey. If unset, claim_rewards_v2 TXs are
    /// NOT submitted after finalization (v7 operators earn nothing until
    /// this is configured). For local simulation you can pass any valid
    /// pubkey; only mainnet / real-economics deployments need this wired.
    #[arg(long, env = "DICE_TREASURY")]
    pub treasury: Option<String>,

    /// Protocol reserve wallet — receives 10% of every fee.
    /// Same semantics as --treasury.
    #[arg(long, env = "DICE_RESERVE")]
    pub reserve: Option<String>,
}
