use anchor_lang::prelude::*;

/// Tracks a registered hardware node (ESP32-S3) by its secp256k1 compressed public key.
///
/// Seeds: [SEED_DEVICE, device_pubkey]
/// Space: 8 (discriminator) + 33 (device_pubkey) + 8 (registered_at) + 8 (jobs_completed) + 1 (is_active) = 58
#[account]
pub struct DeviceRegistry {
    /// Compressed secp256k1 public key (33 bytes)
    pub device_pubkey: [u8; 33],
    /// Unix timestamp when the device was registered
    pub registered_at: i64,
    /// Number of randomness jobs this device has completed
    pub jobs_completed: u64,
    /// Whether the device is currently active
    pub is_active: bool,
}

impl DeviceRegistry {
    pub const LEN: usize = 8 + 33 + 8 + 8 + 1; // 58
}
