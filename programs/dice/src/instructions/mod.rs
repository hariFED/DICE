// v2.0 channel-based instructions
pub mod close_channel;
pub mod deliver_callback;
pub mod fail_round;
pub mod finalize_v2;
pub mod fund_channel;
pub mod init_channel;
pub mod request_randomness_auto;
pub mod request_randomness_v2;
pub mod resize_channel;
pub mod select_nodes;
pub mod submit_commit_v2;
pub mod submit_reveal_v2;
pub mod submit_round_v2;
pub mod withdraw_balance;

// Streaming VRF (v7)
pub mod close_feed;
pub mod init_feed;
pub mod publish_feed_value;

// Universal payout / NodeVault (v7)
pub mod claim_rewards_v2;
pub mod credit_vault_helper;
pub mod register_node_vault;
pub mod rotate_payout_wallet;
pub mod withdraw_from_vault;

// v1.0 legacy (kept for backwards compatibility)
pub mod claim_rewards;
pub mod finalize_randomness;
pub mod fund_escrow;
pub mod init_escrow;
pub mod register_device;
pub mod request_randomness;
pub mod submit_commit;
pub mod submit_reveal;

// v2.0 exports
pub use close_channel::*;
pub use deliver_callback::*;
pub use fail_round::*;
pub use finalize_v2::*;
pub use fund_channel::*;
pub use init_channel::*;
pub use request_randomness_auto::*;
pub use request_randomness_v2::*;
pub use resize_channel::*;
pub use select_nodes::*;
pub use submit_commit_v2::*;
pub use submit_reveal_v2::*;
pub use submit_round_v2::*;
pub use withdraw_balance::*;

// Streaming VRF exports (v7)
pub use close_feed::*;
pub use init_feed::*;
pub use publish_feed_value::*;

// Universal payout / NodeVault exports (v7)
// credit_vault_helper is intentionally NOT glob-reexported — callers use it
// via `crate::instructions::credit_vault_helper::...` to avoid namespace
// clashes with the identically-named `credit_vault_state` helper fn.
pub use claim_rewards_v2::*;
pub use register_node_vault::*;
pub use rotate_payout_wallet::*;
pub use withdraw_from_vault::*;

// v1.0 exports
pub use claim_rewards::*;
pub use finalize_randomness::*;
pub use fund_escrow::*;
pub use init_escrow::*;
pub use register_device::*;
pub use request_randomness::*;
pub use submit_commit::*;
pub use submit_reveal::*;
