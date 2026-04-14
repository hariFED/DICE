#include "payout_binding.h"
#include "crypto.h"
#include "base58.h"
#include "websocket_client.h"
#include "dice_protocol.h"

#include <string.h>
#include <time.h>
#include "esp_log.h"
#include "esp_random.h"
#include "nvs.h"
#include "nvs_flash.h"

static const char *TAG = "dice_payout";

#define NVS_NS             "dice"
#define NVS_SOL_WALLET     "sol_wallet"
#define NVS_BINDING_SENT   "binding_sent"
#define NVS_BINDING_WALLET "binding_wallet"

bool dice_payout_binding_maybe_send(void)
{
    /* ─── Step 1: read sol_wallet from NVS ──────────────────────────── */
    nvs_handle_t nvs;
    esp_err_t    err = nvs_open(NVS_NS, NVS_READWRITE, &nvs);
    if (err != ESP_OK) {
        ESP_LOGE(TAG, "nvs_open failed: %s", esp_err_to_name(err));
        return false;
    }

    char   wallet_b58[48] = {0};
    size_t wlen           = sizeof(wallet_b58);
    err = nvs_get_str(nvs, NVS_SOL_WALLET, wallet_b58, &wlen);
    if (err != ESP_OK || strlen(wallet_b58) == 0) {
        ESP_LOGI(TAG, "no sol_wallet set — skipping binding send");
        nvs_close(nvs);
        return true; /* not an error — operator hasn't set a wallet */
    }

    /* ─── Step 2: check if we've already bound this wallet ─────────── */
    char   prev_wallet[48] = {0};
    size_t pwlen           = sizeof(prev_wallet);
    esp_err_t pw_err       = nvs_get_str(nvs, NVS_BINDING_WALLET, prev_wallet, &pwlen);

    uint8_t sent_flag = 0;
    (void)nvs_get_u8(nvs, NVS_BINDING_SENT, &sent_flag);

    if (pw_err == ESP_OK && strcmp(prev_wallet, wallet_b58) == 0 && sent_flag == 1) {
        ESP_LOGI(TAG, "binding already sent for this wallet — skipping");
        nvs_close(nvs);
        return true;
    }

    /* ─── Step 3: decode base58 → 32 raw bytes ──────────────────────── */
    uint8_t wallet_raw[32];
    size_t  raw_len = 0;
    if (!dice_base58_decode(wallet_b58,
                             strlen(wallet_b58),
                             wallet_raw,
                             sizeof(wallet_raw),
                             &raw_len)
        || raw_len != 32) {
        ESP_LOGE(TAG, "sol_wallet base58 decode failed (len=%u)", (unsigned)raw_len);
        nvs_close(nvs);
        return false;
    }

    /* ─── Step 4: generate a fresh random nonce + timestamp ────────── */
    uint8_t nonce[32];
    esp_fill_random(nonce, sizeof(nonce));

    int64_t timestamp = (int64_t)time(NULL);
    if (timestamp <= 0) {
        /* time() may not be set yet if SNTP hasn't synced — use uptime
         * as a fallback. The coordinator accepts any positive timestamp
         * and the on-chain instruction only uses it as part of the
         * signed message, not for validation against clock.slot. */
        timestamp = (int64_t)(esp_log_timestamp() / 1000);
        ESP_LOGW(TAG, "system time not set, using uptime %lld as timestamp", timestamp);
    }

    /* ─── Step 5: sign the binding message ──────────────────────────── */
    uint8_t signature[64];
    if (!dice_crypto_sign_payout_binding(wallet_raw, timestamp, nonce, signature)) {
        ESP_LOGE(TAG, "dice_crypto_sign_payout_binding failed");
        memset(wallet_raw, 0, sizeof(wallet_raw));
        nvs_close(nvs);
        return false;
    }

    /* ─── Step 6: assemble and send PayoutBindingRequest ────────────── */
    dice_message_t msg = {0};
    msg.type           = DICE_MSG_PAYOUT_BINDING;

    /* Fill in payout_binding fields. node_id is our own pubkey. */
    if (!dice_crypto_get_pubkey(msg.payout_binding.node_id)) {
        ESP_LOGE(TAG, "could not read device pubkey");
        memset(wallet_raw, 0, sizeof(wallet_raw));
        nvs_close(nvs);
        return false;
    }
    memcpy(msg.payout_binding.payout_wallet, wallet_raw, 32);
    msg.payout_binding.timestamp = timestamp;
    memcpy(msg.payout_binding.nonce, nonce, 32);
    memcpy(msg.payout_binding.signature, signature, 64);

    /* Scrub sensitive stack data as soon as the message struct has it. */
    memset(wallet_raw, 0, sizeof(wallet_raw));
    memset(nonce,      0, sizeof(nonce));
    memset(signature,  0, sizeof(signature));

    if (!dice_ws_is_connected()) {
        ESP_LOGW(TAG, "WebSocket not connected — binding send deferred");
        nvs_close(nvs);
        return false;
    }

    if (!dice_ws_send(&msg)) {
        ESP_LOGE(TAG, "dice_ws_send(payout_binding) failed");
        nvs_close(nvs);
        return false;
    }

    ESP_LOGI(TAG,
        "PayoutBindingRequest sent: wallet=%s, ts=%lld",
        wallet_b58, (long long)timestamp);

    /* ─── Step 7: persist the "already sent" marker ─────────────────── */
    esp_err_t e1 = nvs_set_u8(nvs,  NVS_BINDING_SENT,   1);
    esp_err_t e2 = nvs_set_str(nvs, NVS_BINDING_WALLET, wallet_b58);
    esp_err_t e3 = nvs_commit(nvs);
    nvs_close(nvs);

    if (e1 != ESP_OK || e2 != ESP_OK || e3 != ESP_OK) {
        ESP_LOGW(TAG, "NVS flag write failed — will resend on next boot");
        /* Non-fatal: the binding was sent successfully, just the local
         * bookkeeping failed. The on-chain handler is idempotent-ish
         * (register_node_vault rejects re-binding with VaultAlreadyBound),
         * so worst case the next attempt fails harmlessly. */
    }

    return true;
}

void dice_payout_binding_clear_flag(void)
{
    nvs_handle_t nvs;
    if (nvs_open(NVS_NS, NVS_READWRITE, &nvs) != ESP_OK) return;
    nvs_erase_key(nvs, NVS_BINDING_SENT);
    nvs_erase_key(nvs, NVS_BINDING_WALLET);
    nvs_commit(nvs);
    nvs_close(nvs);
}
