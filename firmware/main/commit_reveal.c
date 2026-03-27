#include "commit_reveal.h"
#include "entropy.h"
#include "crypto.h"
#include "websocket_client.h"
#include "heartbeat.h"
#include "dice_protocol.h"

#include <string.h>

#include "esp_log.h"
#include "esp_timer.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "freertos/semphr.h"

static const char *TAG = "dice_cr";

/* ------------------------------------------------------------------ */
/* Pending-job state                                                     */
/* ------------------------------------------------------------------ */

typedef struct {
    bool     active;               /**< True when a job is pending reveal */
    uint8_t  request_id[32];
    uint32_t round_seq;
    uint64_t deadline_ts;
    uint8_t  entropy[32];          /**< Stored for reveal phase */
    uint8_t  commit_hash[32];      /**< SHA-256(entropy) */
} pending_job_t;

static pending_job_t s_pending;
static SemaphoreHandle_t s_mutex = NULL;  /* protects s_pending */

/* ------------------------------------------------------------------ */
/* Internal init (called lazily)                                        */
/* ------------------------------------------------------------------ */

static void ensure_mutex(void)
{
    if (!s_mutex) {
        s_mutex = xSemaphoreCreateMutex();
        if (!s_mutex) {
            ESP_LOGE(TAG, "Failed to create job mutex — aborting");
            abort();
        }
    }
}

/* ------------------------------------------------------------------ */
/* Public API                                                            */
/* ------------------------------------------------------------------ */

void dice_cr_handle_job(const uint8_t request_id[32],
                         uint32_t      round_seq,
                         uint64_t      deadline_ts)
{
    ensure_mutex();

    ESP_LOGI(TAG, "Handling job assignment (round_seq=%lu)", (unsigned long)round_seq);

    /* --- Generate entropy --- */
    uint8_t entropy[32];
    if (!dice_entropy_generate(entropy)) {
        ESP_LOGE(TAG, "Entropy generation failed — discarding job");
        return;
    }

    /* --- Compute commit hash: SHA-256(entropy) --- */
    uint8_t commit_hash[32];
    dice_crypto_sha256(entropy, 32, commit_hash);

    /* --- Sign commit_hash with device key --- */
    uint8_t signature[64];
    if (!dice_crypto_sign(commit_hash, 32, signature)) {
        ESP_LOGE(TAG, "Signing commit hash failed — discarding job");
        memset(entropy, 0, sizeof(entropy));
        return;
    }

    /* --- Get node ID (compressed pubkey) --- */
    uint8_t node_id[33];
    if (!dice_crypto_get_pubkey(node_id)) {
        ESP_LOGE(TAG, "Failed to get pubkey — discarding job");
        memset(entropy, 0, sizeof(entropy));
        memset(signature, 0, sizeof(signature));
        return;
    }

    /* --- Store pending job (hold mutex while updating) --- */
    if (xSemaphoreTake(s_mutex, pdMS_TO_TICKS(1000)) != pdTRUE) {
        ESP_LOGE(TAG, "Could not acquire job mutex");
        memset(entropy, 0, sizeof(entropy));
        return;
    }

    if (s_pending.active) {
        ESP_LOGW(TAG, "Overwriting previous pending job — old entropy discarded");
        memset(s_pending.entropy, 0, sizeof(s_pending.entropy));
    }

    s_pending.active      = true;
    s_pending.round_seq   = round_seq;
    s_pending.deadline_ts = deadline_ts;
    memcpy(s_pending.request_id,  request_id,  32);
    memcpy(s_pending.entropy,     entropy,      32);
    memcpy(s_pending.commit_hash, commit_hash,  32);

    /* Build the CommitSubmission message while still holding the mutex-protected
     * values in local variables (commit_hash has not yet been zeroed). */
    dice_message_t msg;
    memset(&msg, 0, sizeof(msg));
    msg.type = DICE_MSG_COMMIT;
    memcpy(msg.commit.request_id,  request_id,  32);
    memcpy(msg.commit.node_id,     node_id,     33);
    memcpy(msg.commit.commit_hash, commit_hash, 32);  /* use local copy, not s_pending */
    memcpy(msg.commit.signature,   signature,   64);

    xSemaphoreGive(s_mutex);

    /* Scrub local sensitive copies now that they are in the message struct */
    memset(entropy,     0, sizeof(entropy));
    memset(commit_hash, 0, sizeof(commit_hash));
    memset(signature,   0, sizeof(signature));

    if (!dice_ws_send(&msg)) {
        ESP_LOGE(TAG, "Failed to send CommitSubmission for round %lu",
                 (unsigned long)round_seq);
        /* Leave s_pending.active=true so reveal can still be attempted */
    } else {
        ESP_LOGI(TAG, "CommitSubmission sent for round %lu", (unsigned long)round_seq);
    }
}

bool dice_cr_do_reveal(const uint8_t request_id[32])
{
    ensure_mutex();

    if (xSemaphoreTake(s_mutex, pdMS_TO_TICKS(1000)) != pdTRUE) {
        ESP_LOGE(TAG, "cr_do_reveal: could not acquire mutex");
        return false;
    }

    if (!s_pending.active) {
        ESP_LOGW(TAG, "cr_do_reveal: no pending job");
        xSemaphoreGive(s_mutex);
        return false;
    }

    if (memcmp(s_pending.request_id, request_id, 32) != 0) {
        ESP_LOGE(TAG, "cr_do_reveal: request_id mismatch — ignoring");
        xSemaphoreGive(s_mutex);
        return false;
    }

    /* Copy entropy out of the locked region before signing */
    uint8_t entropy[32];
    memcpy(entropy, s_pending.entropy, 32);
    xSemaphoreGive(s_mutex);

    /* --- Sign entropy with device key --- */
    uint8_t signature[64];
    if (!dice_crypto_sign(entropy, 32, signature)) {
        ESP_LOGE(TAG, "cr_do_reveal: signing entropy failed");
        memset(entropy, 0, sizeof(entropy));
        return false;
    }

    /* --- Get node ID --- */
    uint8_t node_id[33];
    if (!dice_crypto_get_pubkey(node_id)) {
        ESP_LOGE(TAG, "cr_do_reveal: failed to get pubkey");
        memset(entropy,   0, sizeof(entropy));
        memset(signature, 0, sizeof(signature));
        return false;
    }

    /* --- Build RevealSubmission --- */
    dice_message_t msg;
    memset(&msg, 0, sizeof(msg));
    msg.type = DICE_MSG_REVEAL;
    memcpy(msg.reveal.request_id, request_id, 32);
    memcpy(msg.reveal.node_id,    node_id,    33);
    memcpy(msg.reveal.entropy,    entropy,    32);
    memcpy(msg.reveal.signature,  signature,  64);

    /* Scrub local copies before sending */
    memset(entropy,   0, sizeof(entropy));
    memset(signature, 0, sizeof(signature));

    if (!dice_ws_send(&msg)) {
        ESP_LOGE(TAG, "cr_do_reveal: failed to send RevealSubmission");
        return false;
    }

    /* --- Clear pending job state --- */
    if (xSemaphoreTake(s_mutex, pdMS_TO_TICKS(1000)) == pdTRUE) {
        memset(s_pending.entropy, 0, sizeof(s_pending.entropy));
        s_pending.active = false;
        xSemaphoreGive(s_mutex);
    }

    ESP_LOGI(TAG, "RevealSubmission sent");

    /* Notify heartbeat module */
    dice_heartbeat_job_completed();

    return true;
}
