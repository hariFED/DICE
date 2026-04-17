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
/* Pending-job queue (ring buffer)                                       */
/* ------------------------------------------------------------------ */

#define MAX_PENDING_JOBS 16  /* Max concurrent commit-reveal rounds */

typedef struct {
    bool     active;               /**< True when this slot holds a pending reveal */
    uint8_t  request_id[32];
    uint32_t round_seq;
    uint64_t deadline_ts;
    uint8_t  entropy[32];          /**< Stored for reveal phase */
    uint8_t  commit_hash[32];      /**< SHA-256(entropy) */
} pending_job_t;

static pending_job_t s_jobs[MAX_PENDING_JOBS];
static SemaphoreHandle_t s_mutex = NULL;

/* ------------------------------------------------------------------ */
/* Internal helpers                                                      */
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

/** Find a free slot or return -1. Caller must hold mutex. */
static int find_free_slot(void)
{
    for (int i = 0; i < MAX_PENDING_JOBS; i++) {
        if (!s_jobs[i].active) return i;
    }
    return -1;
}

/**
 * Find the active slot for `request_id` with the **highest** round_seq.
 *
 * In v2, the on-chain `request_id` is the DiceChannel pubkey and stays
 * constant across every round on that channel. The coordinator bumps
 * `round_seq` on each round. If a previous round on the same channel
 * timed out (e.g. one device was slow and missed the commit deadline),
 * its slot may still be `active` when the NEXT JobAssignment arrives —
 * producing TWO active slots with the same request_id but different
 * round_seq. Returning the first-matching slot (as the original code
 * did) picked up the stale one, and on reveal the device sent entropy
 * from the OLD round while the coordinator held a commit_hash for the
 * NEW round — hash mismatch, reveal rejected, round failed.
 *
 * Fix: walk ALL matching slots and return the one with the largest
 * `round_seq`. Caller must hold mutex.
 */
static int find_slot_by_request(const uint8_t request_id[32])
{
    int best = -1;
    uint32_t best_seq = 0;
    for (int i = 0; i < MAX_PENDING_JOBS; i++) {
        if (!s_jobs[i].active) continue;
        if (memcmp(s_jobs[i].request_id, request_id, 32) != 0) continue;
        if (best < 0 || s_jobs[i].round_seq > best_seq) {
            best = i;
            best_seq = s_jobs[i].round_seq;
        }
    }
    return best;
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

    /* --- Store in pending job queue --- */
    if (xSemaphoreTake(s_mutex, pdMS_TO_TICKS(1000)) != pdTRUE) {
        ESP_LOGE(TAG, "Could not acquire job mutex");
        memset(entropy, 0, sizeof(entropy));
        return;
    }

    /* v7.1 fix: if we already have an active slot for this request_id
     * (common in v2 where request_id == channel PDA is reused across
     * rounds), evict it before inserting the new round's data. This
     * prevents a stale round_N slot from shadowing round_{N+1}'s
     * reveal lookup, and also stops the per-channel slot count from
     * growing every time a round times out. */
    for (int i = 0; i < MAX_PENDING_JOBS; i++) {
        if (s_jobs[i].active
            && memcmp(s_jobs[i].request_id, request_id, 32) == 0)
        {
            ESP_LOGW(TAG, "Evicting stale slot %d (round_seq=%lu) for same request_id",
                     i, (unsigned long)s_jobs[i].round_seq);
            memset(s_jobs[i].entropy, 0, sizeof(s_jobs[i].entropy));
            memset(&s_jobs[i], 0, sizeof(s_jobs[i]));
        }
    }

    int slot = find_free_slot();
    if (slot < 0) {
        ESP_LOGW(TAG, "Job queue full (%d slots) — discarding oldest job", MAX_PENDING_JOBS);
        /* Evict slot 0, shift everything down */
        memset(s_jobs[0].entropy, 0, sizeof(s_jobs[0].entropy));
        for (int i = 0; i < MAX_PENDING_JOBS - 1; i++) {
            s_jobs[i] = s_jobs[i + 1];
        }
        slot = MAX_PENDING_JOBS - 1;
        memset(&s_jobs[slot], 0, sizeof(s_jobs[slot]));
    }

    s_jobs[slot].active      = true;
    s_jobs[slot].round_seq   = round_seq;
    s_jobs[slot].deadline_ts = deadline_ts;
    memcpy(s_jobs[slot].request_id,  request_id,  32);
    memcpy(s_jobs[slot].entropy,     entropy,      32);
    memcpy(s_jobs[slot].commit_hash, commit_hash,  32);

    ESP_LOGI(TAG, "Job queued in slot %d (round_seq=%lu)", slot, (unsigned long)round_seq);

    xSemaphoreGive(s_mutex);

    /* Build the CommitSubmission message */
    dice_message_t msg;
    memset(&msg, 0, sizeof(msg));
    msg.type = DICE_MSG_COMMIT;
    memcpy(msg.commit.request_id,  request_id,  32);
    memcpy(msg.commit.node_id,     node_id,     33);
    memcpy(msg.commit.commit_hash, commit_hash, 32);
    memcpy(msg.commit.signature,   signature,   64);

    /* Scrub local sensitive copies */
    memset(entropy,     0, sizeof(entropy));
    memset(commit_hash, 0, sizeof(commit_hash));
    memset(signature,   0, sizeof(signature));

    if (!dice_ws_send(&msg)) {
        ESP_LOGE(TAG, "Failed to send CommitSubmission for round %lu",
                 (unsigned long)round_seq);
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

    int slot = find_slot_by_request(request_id);
    if (slot < 0) {
        ESP_LOGW(TAG, "cr_do_reveal: no matching job in queue (id=%02x%02x...)",
                 request_id[0], request_id[1]);
        xSemaphoreGive(s_mutex);
        return false;
    }

    /* Copy entropy out of the locked region before signing */
    uint8_t entropy[32];
    memcpy(entropy, s_jobs[slot].entropy, 32);
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

    /* --- Clear this job from the queue --- */
    if (xSemaphoreTake(s_mutex, pdMS_TO_TICKS(1000)) == pdTRUE) {
        int s = find_slot_by_request(request_id);
        if (s >= 0) {
            memset(s_jobs[s].entropy, 0, sizeof(s_jobs[s].entropy));
            s_jobs[s].active = false;
        }
        xSemaphoreGive(s_mutex);
    }

    ESP_LOGI(TAG, "RevealSubmission sent");

    /* Notify heartbeat module */
    dice_heartbeat_job_completed();

    return true;
}
