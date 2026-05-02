use anchor_lang::prelude::*;

use crate::constants::VAULT_SERVICE_COUNT;
use crate::error::DiceError;

/// Lifecycle status of a NodeVault.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum NodeVaultStatus {
    /// Vault exists but no payout wallet has been bound yet.
    /// Credits accumulate. Withdraws are forbidden.
    Unbound,
    /// Vault is bound to a payout wallet. Credits and withdraws both allowed.
    Bound,
    /// Emergency freeze. Credits and withdraws both forbidden.
    /// Reserved for v7.5 — no instruction sets this in Phase 1.
    Frozen,
}

/// The universal per-device payout vault.
///
/// One `NodeVault` exists per physical ESP32-S3 node, keyed by the device's
/// compressed secp256k1 public key. Every DICE service (VRF v1, VRF v2, PoL,
/// sensor attestation, HSM signing, future services) credits the same vault
/// for each round the device contributed to. The operator withdraws from a
/// single place.
///
/// The `payout_wallet` is bound via a hardware-signed attestation (see
/// `register_node_vault` instruction). The device's secp256k1 signing key —
/// the same key that signs commit-reveal entropy — produces a binding
/// signature over `(DOMAIN || device_pubkey || payout_wallet || timestamp ||
/// nonce)`. Anchor verifies the signature on-chain using `secp256k1_recover`,
/// which guarantees that the coordinator cannot forge or redirect bindings.
///
/// Seeds: `["node_vault", device_pubkey]`
#[account]
pub struct NodeVault {
    // ── Identity ──────────────────────────────────────────────────────────
    /// The 33-byte compressed secp256k1 public key identifying this device.
    pub device_pubkey: [u8; 33],

    // ── Binding ───────────────────────────────────────────────────────────
    /// Operator-owned Solana wallet that receives withdraws.
    /// `Pubkey::default()` until the first `register_node_vault` call.
    pub payout_wallet: Pubkey,
    /// Slot at which the current binding was registered (used as rotation
    /// cooldown anchor). Zero until first binding.
    pub binding_slot: u64,
    /// Last ECDSA binding signature accepted on-chain. Stored for audit only.
    pub binding_signature: [u8; 64],
    /// Coordinator-supplied nonce embedded in the last binding message.
    /// Prevents replay of old binding messages.
    pub binding_nonce: [u8; 32],
    /// Unix timestamp embedded in the last binding message.
    pub binding_timestamp: i64,

    // ── Lifecycle ─────────────────────────────────────────────────────────
    /// Vault lifecycle status.
    pub status: NodeVaultStatus,
    /// Slot at which the vault account was first created.
    pub created_slot: u64,

    // ── Totals ────────────────────────────────────────────────────────────
    /// Lifetime lamports credited to this vault across all services.
    pub total_earned: u64,
    /// Lifetime lamports withdrawn by the payout wallet.
    pub total_withdrawn: u64,
    /// Slot of the most recent credit (informational, for operator UX).
    pub last_credit_slot: u64,
    /// Per-service credit counters. Indexed by service ID.
    /// 0=VRF_v1, 1=VRF_v2, 2=PoL, 3=Sensor, 4=HSM, 5-7 reserved.
    pub service_counts: [u64; VAULT_SERVICE_COUNT],

    // ── Reserved ──────────────────────────────────────────────────────────
    /// Zero-filled padding reserved for future fields without requiring a
    /// vault migration. 64 bytes gives ~8 new u64s of headroom.
    pub reserved: [u8; 64],
}

impl NodeVault {
    /// Fixed on-wire size. No Vec fields so this is a compile-time constant.
    ///
    /// 8 (discriminator)
    /// + 33 (device_pubkey)
    /// + 32 (payout_wallet) + 8 (binding_slot) + 64 (binding_signature)
    ///   + 32 (binding_nonce) + 8 (binding_timestamp)
    /// + 1 (status) + 8 (created_slot)
    /// + 8 (total_earned) + 8 (total_withdrawn) + 8 (last_credit_slot)
    /// + VAULT_SERVICE_COUNT * 8 (service_counts)
    /// + 64 (reserved)
    pub const fn space() -> usize {
        8  // discriminator
        + 33 // device_pubkey
        + 32 // payout_wallet
        + 8  // binding_slot
        + 64 // binding_signature
        + 32 // binding_nonce
        + 8  // binding_timestamp
        + 1  // status
        + 8  // created_slot
        + 8  // total_earned
        + 8  // total_withdrawn
        + 8  // last_credit_slot
        + VAULT_SERVICE_COUNT * 8 // service_counts
        + 64 // reserved
    }

    /// Return the lamports currently credited to this vault (earned − withdrawn).
    pub fn balance(&self) -> u64 {
        self.total_earned.saturating_sub(self.total_withdrawn)
    }

    /// Record a successful credit. Caller is responsible for physically
    /// moving lamports into the vault account before or after this call.
    pub fn record_credit(
        &mut self,
        amount: u64,
        service_id: u8,
        slot: u64,
    ) -> Result<()> {
        require!(
            (service_id as usize) < VAULT_SERVICE_COUNT,
            DiceError::VaultUnknownServiceId
        );
        self.total_earned = self
            .total_earned
            .checked_add(amount)
            .ok_or(error!(DiceError::VaultCreditOverflow))?;
        self.service_counts[service_id as usize] = self.service_counts[service_id as usize]
            .checked_add(1)
            .ok_or(error!(DiceError::VaultCreditOverflow))?;
        self.last_credit_slot = slot;
        Ok(())
    }

    /// Record a withdrawal. Caller is responsible for the lamport transfer.
    pub fn record_withdraw(&mut self, amount: u64) -> Result<()> {
        require!(amount > 0, DiceError::VaultZeroAmount);
        require!(
            self.balance() >= amount,
            DiceError::VaultInsufficientBalance
        );
        self.total_withdrawn = self
            .total_withdrawn
            .checked_add(amount)
            .ok_or(error!(DiceError::VaultCreditOverflow))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_vault() -> NodeVault {
        NodeVault {
            device_pubkey: [0x02; 33],
            payout_wallet: Pubkey::default(),
            binding_slot: 0,
            binding_signature: [0u8; 64],
            binding_nonce: [0u8; 32],
            binding_timestamp: 0,
            status: NodeVaultStatus::Unbound,
            created_slot: 100,
            total_earned: 0,
            total_withdrawn: 0,
            last_credit_slot: 0,
            service_counts: [0u64; VAULT_SERVICE_COUNT],
            reserved: [0u8; 64],
        }
    }

    #[test]
    fn space_is_expected_constant_and_under_limit() {
        let s = NodeVault::space();
        // 8 + 33 + 32 + 8 + 64 + 32 + 8 + 1 + 8 + 8 + 8 + 8 + 8*8 + 64
        // = 8 + 33 + 32 + 8 + 64 + 32 + 8 + 1 + 8 + 8 + 8 + 8 + 64 + 64
        assert_eq!(s, 346);
        assert!(s <= 10_240, "vault must fit in init limit");
    }

    #[test]
    fn balance_is_earned_minus_withdrawn() {
        let mut v = fresh_vault();
        v.total_earned = 1_000_000;
        v.total_withdrawn = 250_000;
        assert_eq!(v.balance(), 750_000);
    }

    #[test]
    fn balance_saturates_to_zero_if_inconsistent() {
        // Defensive: even if state was corrupted, balance() returns 0 rather
        // than panicking.
        let mut v = fresh_vault();
        v.total_earned = 100;
        v.total_withdrawn = 500;
        assert_eq!(v.balance(), 0);
    }

    #[test]
    fn record_credit_updates_totals_and_counters() {
        let mut v = fresh_vault();
        v.record_credit(1_400_000, 0, 5000).unwrap(); // VRF v1
        v.record_credit(1_400_000, 1, 5500).unwrap(); // VRF v2
        v.record_credit(1_400_000, 1, 6000).unwrap(); // VRF v2 again

        assert_eq!(v.total_earned, 4_200_000);
        assert_eq!(v.last_credit_slot, 6000);
        assert_eq!(v.service_counts[0], 1, "VRF v1 count");
        assert_eq!(v.service_counts[1], 2, "VRF v2 count");
        assert_eq!(v.service_counts[2], 0, "PoL untouched");
    }

    #[test]
    fn record_credit_rejects_unknown_service() {
        let mut v = fresh_vault();
        let r = v.record_credit(100, 99, 1);
        assert!(r.is_err(), "service_id 99 must be rejected");
    }

    #[test]
    fn record_credit_detects_overflow() {
        let mut v = fresh_vault();
        v.total_earned = u64::MAX - 10;
        let r = v.record_credit(100, 0, 1);
        assert!(r.is_err(), "overflow must be caught");
    }

    #[test]
    fn record_withdraw_respects_balance() {
        let mut v = fresh_vault();
        v.total_earned = 1_000;
        // Withdraw exactly the balance — should succeed
        v.record_withdraw(1_000).unwrap();
        assert_eq!(v.total_withdrawn, 1_000);
        assert_eq!(v.balance(), 0);
    }

    #[test]
    fn record_withdraw_rejects_over_balance() {
        let mut v = fresh_vault();
        v.total_earned = 100;
        let r = v.record_withdraw(200);
        assert!(r.is_err(), "over-withdraw must fail");
    }

    #[test]
    fn record_withdraw_rejects_zero() {
        let mut v = fresh_vault();
        v.total_earned = 100;
        let r = v.record_withdraw(0);
        assert!(r.is_err(), "zero amount must fail");
    }

    #[test]
    fn partial_withdraw_leaves_residual_balance() {
        let mut v = fresh_vault();
        v.total_earned = 1_000;
        v.record_withdraw(300).unwrap();
        assert_eq!(v.balance(), 700);
        v.record_withdraw(500).unwrap();
        assert_eq!(v.balance(), 200);
    }

    #[test]
    fn status_round_trips_through_borsh() {
        for variant in [
            NodeVaultStatus::Unbound,
            NodeVaultStatus::Bound,
            NodeVaultStatus::Frozen,
        ] {
            let mut buf = Vec::new();
            variant.serialize(&mut buf).unwrap();
            let decoded = NodeVaultStatus::deserialize(&mut &buf[..]).unwrap();
            assert_eq!(variant, decoded);
        }
    }
}
