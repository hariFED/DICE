//! API key authentication and rate limiting for coordinator endpoints.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use subtle::ConstantTimeEq;

// ── API Key Authentication ─────────────────────────────────────────────────

/// Shared state for auth middleware.
#[derive(Clone)]
pub struct AuthState {
    /// Expected API key. If None, auth is disabled (simulation mode).
    pub api_key: Option<String>,
}

/// Middleware: validates `Authorization: Bearer <token>` on protected routes.
pub async fn require_api_key(
    State(auth): State<AuthState>,
    req: Request<Body>,
    next: Next,
) -> Response {
    // No key configured → allow all
    let expected = match &auth.api_key {
        Some(k) if !k.is_empty() => k,
        _ => return next.run(req).await,
    };

    let auth_header = req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let provided = auth_header.strip_prefix("Bearer ").unwrap_or("");

    // Constant-time comparison
    let expected_bytes = expected.as_bytes();
    let provided_bytes = provided.as_bytes();

    if expected_bytes.len() == provided_bytes.len()
        && expected_bytes.ct_eq(provided_bytes).into()
    {
        next.run(req).await
    } else {
        (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "unauthorized — provide Authorization: Bearer <api_key>" })),
        )
            .into_response()
    }
}

// ── Rate Limiting ──────────────────────────────────────────────────────────

/// Simple token bucket rate limiter using atomics.
pub struct RateLimiter {
    /// Tokens available (replenished at `rps` per second).
    tokens: AtomicU64,
    /// Max tokens (= rps).
    max_tokens: u64,
    /// Last refill timestamp (unix millis).
    last_refill: AtomicU64,
    /// Refill rate (tokens per second).
    rps: u64,
}

impl RateLimiter {
    pub fn new(rps: u32) -> Self {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        Self {
            tokens: AtomicU64::new(rps as u64),
            max_tokens: rps as u64,
            last_refill: AtomicU64::new(now_ms),
            rps: rps as u64,
        }
    }

    /// Try to consume one token. Returns true if allowed, false if rate-limited.
    pub fn try_acquire(&self) -> bool {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        // Refill tokens based on elapsed time
        let last = self.last_refill.load(Ordering::Relaxed);
        let elapsed_ms = now_ms.saturating_sub(last);

        if elapsed_ms >= 1000 {
            // Refill: add rps tokens per second elapsed
            let seconds = elapsed_ms / 1000;
            let refill = seconds * self.rps;
            let current = self.tokens.load(Ordering::Relaxed);
            let new_tokens = (current + refill).min(self.max_tokens);
            self.tokens.store(new_tokens, Ordering::Relaxed);
            self.last_refill.store(now_ms, Ordering::Relaxed);
        }

        // Try to take a token
        loop {
            let current = self.tokens.load(Ordering::Relaxed);
            if current == 0 {
                return false;
            }
            if self
                .tokens
                .compare_exchange(current, current - 1, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
            {
                return true;
            }
        }
    }
}
