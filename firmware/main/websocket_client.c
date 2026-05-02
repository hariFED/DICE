#include "websocket_client.h"

#include <string.h>
#include <stdlib.h>

#include "esp_log.h"
#include "esp_websocket_client.h"
#include "nvs.h"
#include "nvs_flash.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "freertos/semphr.h"
#include "esp_timer.h"

static const char *TAG = "dice_ws";

/* ------------------------------------------------------------------ */
/* NVS keys for TLS material                                            */
/* ------------------------------------------------------------------ */

#define WS_NVS_NS          "dice"
#define WS_NVS_CLIENT_CERT "client_cert_pem"
#define WS_NVS_CLIENT_KEY  "client_key_pem"
#define WS_NVS_CA_CERT     "ca_cert_pem"

/* Maximum PEM sizes (generous but bounded) */
#define MAX_PEM_LEN   4096

/* Maximum encoded CBOR message size */
#define WS_SEND_BUF_LEN  512

/* ------------------------------------------------------------------ */
/* Reconnection backoff                                                  */
/* ------------------------------------------------------------------ */

#define BACKOFF_INITIAL_MS   1000
#define BACKOFF_MAX_MS      60000

/* ------------------------------------------------------------------ */
/* Module state                                                          */
/* ------------------------------------------------------------------ */

typedef struct {
    esp_websocket_client_handle_t client;
    dice_message_handler_t        handler;
    bool                          connected;
    bool                          should_stop;
    uint32_t                      backoff_ms;
    SemaphoreHandle_t             send_mutex;
    int64_t                       last_send_us;  /* for latency tracking */
    int64_t                       last_recv_us;
    /* TLS material (heap-allocated, freed on disconnect) */
    char    *client_cert;
    char    *client_key;
    char    *ca_cert;
    char     uri[256];
} ws_state_t;

static ws_state_t s_ws;

/* ------------------------------------------------------------------ */
/* Latency tracking (accessible by heartbeat module)                    */
/* ------------------------------------------------------------------ */

/* Exposed via a weak symbol so heartbeat.c can read it without a header
 * dependency cycle.  Stored as milliseconds. */
volatile uint32_t g_dice_ws_latency_ms = 0;

/* ------------------------------------------------------------------ */
/* NVS helpers                                                           */
/* ------------------------------------------------------------------ */

static char *nvs_load_string(const char *ns, const char *key)
{
    nvs_handle_t handle;
    if (nvs_open(ns, NVS_READONLY, &handle) != ESP_OK) {
        ESP_LOGE(TAG, "nvs_open failed for '%s'", ns);
        return NULL;
    }

    size_t len = MAX_PEM_LEN;
    char *buf = malloc(len);
    if (!buf) {
        nvs_close(handle);
        return NULL;
    }

    esp_err_t err = nvs_get_str(handle, key, buf, &len);
    nvs_close(handle);

    if (err != ESP_OK) {
        ESP_LOGE(TAG, "nvs_get_str('%s') failed: %s", key, esp_err_to_name(err));
        free(buf);
        return NULL;
    }

    return buf;
}

/* ------------------------------------------------------------------ */
/* WebSocket event handler                                               */
/* ------------------------------------------------------------------ */

static void ws_event_handler(void *arg,
                              esp_event_base_t event_base,
                              int32_t event_id,
                              void *event_data)
{
    esp_websocket_event_data_t *data = (esp_websocket_event_data_t *)event_data;

    switch (event_id) {
    case WEBSOCKET_EVENT_CONNECTED:
        ESP_LOGI(TAG, "WebSocket connected");
        s_ws.connected  = true;
        s_ws.backoff_ms = BACKOFF_INITIAL_MS;
        break;

    case WEBSOCKET_EVENT_DISCONNECTED:
        ESP_LOGW(TAG, "WebSocket disconnected");
        s_ws.connected = false;
        break;

    case WEBSOCKET_EVENT_DATA:
        if (data->op_code == 0x02 /* binary */ && data->data_len > 0) {
            /* Track receive time for latency */
            int64_t now_us = esp_timer_get_time();
            if (s_ws.last_send_us > 0) {
                g_dice_ws_latency_ms =
                    (uint32_t)((now_us - s_ws.last_send_us) / 1000);
            }
            s_ws.last_recv_us = now_us;

            /* Decode and dispatch the message */
            dice_message_t msg;
            bool ok = dice_msg_decode((const uint8_t *)data->data_ptr,
                                       (size_t)data->data_len,
                                       &msg);
            if (ok && s_ws.handler) {
                s_ws.handler(&msg);
            } else if (!ok) {
                ESP_LOGW(TAG, "Failed to decode incoming CBOR message (%d bytes)",
                         data->data_len);
            }
        }
        break;

    case WEBSOCKET_EVENT_ERROR:
        ESP_LOGE(TAG, "WebSocket error");
        s_ws.connected = false;
        break;

    default:
        break;
    }
}

/* ------------------------------------------------------------------ */
/* Reconnection task                                                     */
/* ------------------------------------------------------------------ */

static void ws_reconnect_task(void *pvParam)
{
    (void)pvParam;

    while (!s_ws.should_stop) {
        if (!s_ws.connected && s_ws.client) {
            ESP_LOGI(TAG, "Attempting reconnect in %lu ms", (unsigned long)s_ws.backoff_ms);
            vTaskDelay(pdMS_TO_TICKS(s_ws.backoff_ms));

            if (s_ws.should_stop) break;

            esp_err_t err = esp_websocket_client_start(s_ws.client);
            if (err != ESP_OK) {
                ESP_LOGE(TAG, "Reconnect start failed: %s", esp_err_to_name(err));
                /* Exponential backoff */
                s_ws.backoff_ms = (s_ws.backoff_ms * 2 > BACKOFF_MAX_MS)
                                  ? BACKOFF_MAX_MS
                                  : s_ws.backoff_ms * 2;
            }
        } else {
            /* Connected — check again in 5 s */
            vTaskDelay(pdMS_TO_TICKS(5000));
        }
    }

    vTaskDelete(NULL);
}

/* ------------------------------------------------------------------ */
/* Public API                                                            */
/* ------------------------------------------------------------------ */

bool dice_ws_connect(const char *uri, dice_message_handler_t handler)
{
    if (s_ws.client) {
        ESP_LOGW(TAG, "dice_ws_connect called while already connected");
        return false;
    }

    memset(&s_ws, 0, sizeof(s_ws));

    /* Snapshot URI */
    strncpy(s_ws.uri, uri, sizeof(s_ws.uri) - 1);
    s_ws.handler    = handler;
    s_ws.backoff_ms = BACKOFF_INITIAL_MS;

    /* Load TLS material from NVS */
    s_ws.client_cert = nvs_load_string(WS_NVS_NS, WS_NVS_CLIENT_CERT);
    s_ws.client_key  = nvs_load_string(WS_NVS_NS, WS_NVS_CLIENT_KEY);
    s_ws.ca_cert     = nvs_load_string(WS_NVS_NS, WS_NVS_CA_CERT);

    if (!s_ws.client_cert || !s_ws.client_key || !s_ws.ca_cert) {
        ESP_LOGE(TAG, "Failed to load TLS credentials from NVS");
        goto fail_creds;
    }

    /* Mutex for thread-safe sends */
    s_ws.send_mutex = xSemaphoreCreateMutex();
    if (!s_ws.send_mutex) {
        ESP_LOGE(TAG, "Failed to create send mutex");
        goto fail_creds;
    }

    /* Configure the WebSocket client.
     * Auto-detect transport from URI scheme: ws:// = plain, wss:// = SSL+mTLS. */
    bool use_ssl = (strncmp(s_ws.uri, "wss://", 6) == 0);

    esp_websocket_client_config_t cfg = {
        .uri                = s_ws.uri,
        .transport          = use_ssl ? WEBSOCKET_TRANSPORT_OVER_SSL
                                      : WEBSOCKET_TRANSPORT_OVER_TCP,
        .task_stack         = 6144,
        .buffer_size        = 1024,
        .reconnect_timeout_ms = 0,
        .network_timeout_ms   = 10000,
        .ping_interval_sec    = 20,
    };

    /* Only attach TLS certificates when using wss:// */
    if (use_ssl) {
        cfg.cert_pem    = s_ws.ca_cert;
        cfg.client_cert = s_ws.client_cert;
        cfg.client_key  = s_ws.client_key;
        ESP_LOGI(TAG, "Using wss:// with mTLS certificates");
    } else {
        ESP_LOGI(TAG, "Using ws:// plain WebSocket (development mode)");
    }

    s_ws.client = esp_websocket_client_init(&cfg);
    if (!s_ws.client) {
        ESP_LOGE(TAG, "esp_websocket_client_init failed");
        goto fail_client;
    }

    /* Register event handler */
    ESP_ERROR_CHECK(esp_websocket_register_events(s_ws.client,
                                                   WEBSOCKET_EVENT_ANY,
                                                   ws_event_handler,
                                                   NULL));

    /* Start the WebSocket client */
    esp_err_t err = esp_websocket_client_start(s_ws.client);
    if (err != ESP_OK) {
        ESP_LOGE(TAG, "esp_websocket_client_start failed: %s", esp_err_to_name(err));
        /* Don't goto fail yet — reconnect task will retry */
    }

    /* Start reconnection watchdog task */
    s_ws.should_stop = false;
    BaseType_t task_ret = xTaskCreate(ws_reconnect_task,
                                       "ws_reconnect",
                                       4096,
                                       NULL,
                                       5,
                                       NULL);
    if (task_ret != pdPASS) {
        ESP_LOGE(TAG, "Failed to create reconnect task");
        goto fail_task;
    }

    ESP_LOGI(TAG, "WebSocket client started for %s", uri);
    return true;

fail_task:
    esp_websocket_client_stop(s_ws.client);
    esp_websocket_client_destroy(s_ws.client);
    s_ws.client = NULL;
fail_client:
    if (s_ws.send_mutex) {
        vSemaphoreDelete(s_ws.send_mutex);
        s_ws.send_mutex = NULL;
    }
fail_creds:
    if (s_ws.client_cert) { free(s_ws.client_cert); s_ws.client_cert = NULL; }
    if (s_ws.client_key)  { free(s_ws.client_key);  s_ws.client_key  = NULL; }
    if (s_ws.ca_cert)     { free(s_ws.ca_cert);     s_ws.ca_cert     = NULL; }
    return false;
}

bool dice_ws_send(const dice_message_t *msg)
{
    if (!s_ws.client || !s_ws.connected) {
        ESP_LOGW(TAG, "dice_ws_send: not connected");
        return false;
    }

    uint8_t cbor_buf[WS_SEND_BUF_LEN];
    int cbor_len = dice_msg_encode(msg, cbor_buf, sizeof(cbor_buf));
    if (cbor_len < 0) {
        ESP_LOGE(TAG, "dice_ws_send: CBOR encode failed");
        return false;
    }

    if (xSemaphoreTake(s_ws.send_mutex, pdMS_TO_TICKS(2000)) != pdTRUE) {
        ESP_LOGE(TAG, "dice_ws_send: mutex timeout");
        return false;
    }

    s_ws.last_send_us = esp_timer_get_time();

    int ret = esp_websocket_client_send_bin(s_ws.client,
                                             (const char *)cbor_buf,
                                             cbor_len,
                                             pdMS_TO_TICKS(5000));
    xSemaphoreGive(s_ws.send_mutex);

    if (ret < 0) {
        ESP_LOGE(TAG, "dice_ws_send: send failed (ret=%d)", ret);
        return false;
    }

    return true;
}

void dice_ws_disconnect(void)
{
    s_ws.should_stop = true;

    if (s_ws.client) {
        esp_websocket_client_stop(s_ws.client);
        esp_websocket_client_destroy(s_ws.client);
        s_ws.client    = NULL;
        s_ws.connected = false;
    }

    if (s_ws.send_mutex) {
        vSemaphoreDelete(s_ws.send_mutex);
        s_ws.send_mutex = NULL;
    }

    if (s_ws.client_cert) { free(s_ws.client_cert); s_ws.client_cert = NULL; }
    if (s_ws.client_key)  { free(s_ws.client_key);  s_ws.client_key  = NULL; }
    if (s_ws.ca_cert)     { free(s_ws.ca_cert);     s_ws.ca_cert     = NULL; }

    ESP_LOGI(TAG, "WebSocket disconnected");
}

bool dice_ws_is_connected(void)
{
    return s_ws.client != NULL && s_ws.connected;
}
