use std::sync::Arc;

use prometheus_client::{
    encoding::text::encode,
    metrics::{counter::Counter, gauge::Gauge, histogram::Histogram},
    registry::Registry,
};

/// Signed integer gauge (supports decrement).
pub type IntGauge = Gauge<i64, std::sync::atomic::AtomicI64>;

/// All application-level Prometheus metrics in one place.
#[derive(Clone)]
pub struct Metrics {
    /// Number of nodes currently connected via mTLS WebSocket.
    pub nodes_connected: IntGauge,
    /// Total number of randomness rounds started since process boot.
    pub rounds_total: Counter,
    /// Total number of rounds that ended in `Failed` state.
    pub rounds_failed_total: Counter,
    /// Histogram of wall-clock time from round start to finalization (seconds).
    pub round_duration_seconds: Histogram,
    /// Total Solana transaction submission failures.
    pub solana_tx_failed_total: Counter,
    /// Total mTLS handshake failures on the WebSocket listener.
    pub mtls_handshake_failed_total: Counter,
    /// The Prometheus registry — kept so we can render `/metrics`.
    registry: Arc<Registry>,
}

impl Metrics {
    /// Create and register all metrics.  Call once at startup.
    pub fn new() -> Self {
        let mut registry = Registry::default();

        let nodes_connected = IntGauge::default();
        let rounds_total = Counter::default();
        let rounds_failed_total = Counter::default();
        let round_duration_seconds = Histogram::new([1.0, 5.0, 10.0, 20.0, 30.0, 60.0].into_iter());
        let solana_tx_failed_total = Counter::default();
        let mtls_handshake_failed_total = Counter::default();

        registry.register(
            "dice_nodes_connected",
            "Currently connected hardware nodes",
            nodes_connected.clone(),
        );
        registry.register(
            "dice_rounds_total",
            "Total randomness rounds started",
            rounds_total.clone(),
        );
        registry.register(
            "dice_rounds_failed_total",
            "Total randomness rounds that failed",
            rounds_failed_total.clone(),
        );
        registry.register(
            "dice_round_duration_seconds",
            "Wall-clock time from round start to finalization (seconds)",
            round_duration_seconds.clone(),
        );
        registry.register(
            "dice_solana_tx_failed_total",
            "Total Solana transaction submission failures",
            solana_tx_failed_total.clone(),
        );
        registry.register(
            "dice_mtls_handshake_failed_total",
            "Total mTLS WebSocket handshake failures",
            mtls_handshake_failed_total.clone(),
        );

        Metrics {
            nodes_connected,
            rounds_total,
            rounds_failed_total,
            round_duration_seconds,
            solana_tx_failed_total,
            mtls_handshake_failed_total,
            registry: Arc::new(registry),
        }
    }

    /// Render all metrics in the Prometheus text exposition format.
    pub fn render(&self) -> String {
        let mut buf = String::new();
        encode(&mut buf, &self.registry).expect("metrics encoding is infallible");
        buf
    }
}
