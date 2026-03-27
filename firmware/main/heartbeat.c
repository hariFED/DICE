#include "heartbeat.h"
#include "websocket_client.h"
#include "crypto.h"
#include "dice_protocol.h"

#include <string.h>

#include "esp_log.h"
#include "esp_timer.h"
#include "freertos/FreeRTOS.h"
#include "freertos/timers.h"

static const char *TAG = "dice_hb";

/* Heartbeat interval: 25 seconds */
#define HEARTBEAT_INTERVAL_MS  25000

/* Shared latency value exported by websocket_client.c */
extern volatile uint32_t g_dice_ws_latency_ms;

/* ------------------------------------------------------------------ */
/* Module state                                                          */
/* ------------------------------------------------------------------ */

static TimerHandle_t  s_timer = NULL;
static uint64_t       s_jobs_completed = 0;

/* ------------------------------------------------------------------ */
/* Timer callback                                                        */
/* ------------------------------------------------------------------ */

static void heartbeat_timer_cb(TimerHandle_t xTimer)
{
    (void)xTimer;

    if (!dice_ws_is_connected()) {
        ESP_LOGD(TAG, "Not connected, skipping heartbeat");
        return;
    }

    /* Build heartbeat message */
    dice_message_t msg;
    memset(&msg, 0, sizeof(msg));
    msg.type = DICE_MSG_HEARTBEAT;

    /* node_id = compressed public key */
    if (!dice_crypto_get_pubkey(msg.heartbeat.node_id)) {
        ESP_LOGE(TAG, "Failed to get pubkey for heartbeat");
        return;
    }

    /* Uptime in seconds */
    msg.heartbeat.uptime_secs = (uint64_t)(esp_timer_get_time() / 1000000LL);

    /* Last round-trip latency */
    msg.heartbeat.latency_ms = g_dice_ws_latency_ms;

    /* Jobs completed counter */
    msg.heartbeat.jobs_completed = s_jobs_completed;

    /* Current Unix timestamp approximation.
     * In production, NTP sync would set this properly.
     * We use a monotonic fallback here. */
    msg.heartbeat.timestamp = msg.heartbeat.uptime_secs;

    ESP_LOGI(TAG, "Sending heartbeat (uptime=%llu s, latency=%lu ms, jobs=%llu)",
             (unsigned long long)msg.heartbeat.uptime_secs,
             (unsigned long)msg.heartbeat.latency_ms,
             (unsigned long long)msg.heartbeat.jobs_completed);

    if (!dice_ws_send(&msg)) {
        ESP_LOGW(TAG, "Heartbeat send failed");
    }
}

/* ------------------------------------------------------------------ */
/* Public API                                                            */
/* ------------------------------------------------------------------ */

void dice_heartbeat_start(void)
{
    if (s_timer) {
        ESP_LOGW(TAG, "Heartbeat timer already running");
        return;
    }

    s_timer = xTimerCreate(
        "dice_hb",
        pdMS_TO_TICKS(HEARTBEAT_INTERVAL_MS),
        pdTRUE,        /* auto-reload */
        NULL,
        heartbeat_timer_cb
    );

    if (!s_timer) {
        ESP_LOGE(TAG, "Failed to create heartbeat timer");
        return;
    }

    if (xTimerStart(s_timer, pdMS_TO_TICKS(1000)) != pdPASS) {
        ESP_LOGE(TAG, "Failed to start heartbeat timer");
        xTimerDelete(s_timer, portMAX_DELAY);
        s_timer = NULL;
        return;
    }

    ESP_LOGI(TAG, "Heartbeat timer started (%d ms interval)", HEARTBEAT_INTERVAL_MS);
}

void dice_heartbeat_stop(void)
{
    if (!s_timer) return;

    xTimerStop(s_timer, pdMS_TO_TICKS(1000));
    xTimerDelete(s_timer, pdMS_TO_TICKS(1000));
    s_timer = NULL;
    ESP_LOGI(TAG, "Heartbeat timer stopped");
}

void dice_heartbeat_job_completed(void)
{
    /* Increment atomically (called from the commit-reveal task context) */
    s_jobs_completed++;
}
