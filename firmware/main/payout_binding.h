#pragma once
#include <stdbool.h>

/**
 * @file payout_binding.h
 * @brief Firmware-side payout wallet binding helper.
 *
 * After the operator enters a Solana wallet in the captive portal, the
 * device persists it to NVS under "dice"/"sol_wallet". This helper reads
 * that wallet, assembles the hardware-signed binding message, encodes it
 * as a DICE_MSG_PAYOUT_BINDING over the coordinator WebSocket, and marks
 * the binding as sent so it only fires once per bound wallet.
 *
 * Typical call site: once at startup, AFTER the WebSocket is connected
 * and AFTER crypto init, AFTER any registration handshake has completed.
 */

/**
 * Send a PayoutBindingRequest if one is pending.
 *
 * Reads NVS keys:
 *   - "dice"/"sol_wallet"        : 32-44 base58 chars (operator input)
 *   - "dice"/"binding_sent"      : u8 flag (1 = already sent this wallet)
 *   - "dice"/"binding_wallet"    : last bound wallet (for change detection)
 *
 * Behavior:
 *   - If sol_wallet is missing or empty → no-op (operator hasn't set one)
 *   - If binding_wallet matches sol_wallet AND binding_sent == 1 → no-op
 *     (binding already registered on-chain, no re-send needed)
 *   - Otherwise: decode sol_wallet from base58, generate a fresh nonce +
 *     timestamp, call dice_crypto_sign_payout_binding, encode as CBOR,
 *     and send via dice_ws_send. On success, set binding_sent=1 and
 *     copy sol_wallet → binding_wallet in NVS.
 *
 * Safe to call repeatedly — the NVS flag prevents duplicate submissions.
 *
 * @return true if a binding was sent (or not needed), false on error.
 */
bool dice_payout_binding_maybe_send(void);

/**
 * Forget the binding_sent flag so the next call to
 * dice_payout_binding_maybe_send() re-sends the binding.
 *
 * Called by factory_reset after wiping sol_wallet, so a new binding is
 * sent when the operator re-provisions with a different wallet.
 */
void dice_payout_binding_clear_flag(void);
