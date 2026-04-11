#include <string.h>
#include <stdlib.h>

#include "esp_log.h"
#include "esp_err.h"
#include "esp_timer.h"
#include "esp_wifi.h"
#include "esp_event.h"
#include "esp_netif.h"
#include "nvs_flash.h"
#include "nvs.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "freertos/event_groups.h"

#include "entropy.h"
#include "crypto.h"
#include "websocket_client.h"
#include "heartbeat.h"
#include "commit_reveal.h"
#include "dice_protocol.h"
#include "captive_portal.h"
#include "factory_reset.h"
#include "led_status.h"

static const char *TAG = "dice_main";

/* ------------------------------------------------------------------ */
/* WiFi event handling                                                   */
/* ------------------------------------------------------------------ */

/* WiFi event bits — only WIFI_CONNECTED_BIT is used in practice. We no
 * longer surface a "failed" bit because the credentials were already
 * validated by the captive portal before being saved to NVS. If the STA
 * drops after boot it's an environmental issue (router down, channel
 * change, roaming), not bad creds — so we retry forever with exponential
 * backoff instead of giving up and wiping NVS like the old code did. */
#define WIFI_CONNECTED_BIT BIT0

/* Exponential backoff between reconnect attempts: 1s, 2s, 4s, 8s, 16s, 32s,
 * then capped at BACKOFF_MAX_SECS thereafter. */
#define BACKOFF_INITIAL_SECS 1
#define BACKOFF_MAX_SECS     60

static EventGroupHandle_t s_wifi_event_group;
static uint32_t           s_wifi_backoff_secs = BACKOFF_INITIAL_SECS;

static void wifi_event_handler(void *arg,
                                esp_event_base_t event_base,
                                int32_t event_id,
                                void *event_data)
{
    if (event_base == WIFI_EVENT && event_id == WIFI_EVENT_STA_START) {
        esp_wifi_connect();
        return;
    }

    if (event_base == WIFI_EVENT && event_id == WIFI_EVENT_STA_DISCONNECTED) {
        ESP_LOGW(TAG, "WiFi disconnected — retrying in %lu seconds",
                 (unsigned long)s_wifi_backoff_secs);

        /* Sleep the current backoff interval, then reconnect. Blocking this
         * handler task is fine: esp_wifi runs its own task and isn't held up. */
        vTaskDelay(pdMS_TO_TICKS(s_wifi_backoff_secs * 1000));

        /* Double the backoff for next time, capped at BACKOFF_MAX_SECS. */
        s_wifi_backoff_secs *= 2;
        if (s_wifi_backoff_secs > BACKOFF_MAX_SECS) {
            s_wifi_backoff_secs = BACKOFF_MAX_SECS;
        }

        esp_wifi_connect();
        return;
    }

    if (event_base == IP_EVENT && event_id == IP_EVENT_STA_GOT_IP) {
        ip_event_got_ip_t *event = (ip_event_got_ip_t *)event_data;
        ESP_LOGI(TAG, "Got IP: " IPSTR, IP2STR(&event->ip_info.ip));

        /* Reset backoff on a successful connect so the next drop starts
         * at 1 second again. */
        s_wifi_backoff_secs = BACKOFF_INITIAL_SECS;
        xEventGroupSetBits(s_wifi_event_group, WIFI_CONNECTED_BIT);
        return;
    }
}

/**
 * Initialize WiFi in station mode.
 * Credentials loaded from NVS: "dice"/"wifi_ssid" and "dice"/"wifi_pass".
 * Blocks until connected or fails with abort().
 */
static void wifi_init_sta(void)
{
    /* Read SSID and password from NVS */
    nvs_handle_t nvs_handle;
    ESP_ERROR_CHECK(nvs_open("dice", NVS_READONLY, &nvs_handle));

    char ssid[33] = {0};
    char pass[65] = {0};
    size_t ssid_len = sizeof(ssid);
    size_t pass_len = sizeof(pass);

    esp_err_t err = nvs_get_str(nvs_handle, "wifi_ssid", ssid, &ssid_len);
    if (err != ESP_OK) {
        ESP_LOGE(TAG, "Failed to read wifi_ssid from NVS: %s", esp_err_to_name(err));
        nvs_close(nvs_handle);
        abort();
    }

    err = nvs_get_str(nvs_handle, "wifi_pass", pass, &pass_len);
    if (err != ESP_OK) {
        ESP_LOGE(TAG, "Failed to read wifi_pass from NVS: %s", esp_err_to_name(err));
        nvs_close(nvs_handle);
        abort();
    }
    nvs_close(nvs_handle);

    ESP_LOGI(TAG, "Connecting to WiFi SSID: %s", ssid);

    /* Initialize networking stack */
    s_wifi_event_group = xEventGroupCreate();

    ESP_ERROR_CHECK(esp_netif_init());
    ESP_ERROR_CHECK(esp_event_loop_create_default());
    esp_netif_create_default_wifi_sta();

    wifi_init_config_t cfg = WIFI_INIT_CONFIG_DEFAULT();
    ESP_ERROR_CHECK(esp_wifi_init(&cfg));

    /* Register event handlers */
    esp_event_handler_instance_t inst_any_id;
    esp_event_handler_instance_t inst_got_ip;
    ESP_ERROR_CHECK(esp_event_handler_instance_register(
        WIFI_EVENT, ESP_EVENT_ANY_ID, &wifi_event_handler, NULL, &inst_any_id));
    ESP_ERROR_CHECK(esp_event_handler_instance_register(
        IP_EVENT, IP_EVENT_STA_GOT_IP, &wifi_event_handler, NULL, &inst_got_ip));

    /* Configure WiFi */
    wifi_config_t wifi_config = {0};
    strncpy((char *)wifi_config.sta.ssid,     ssid, sizeof(wifi_config.sta.ssid) - 1);
    strncpy((char *)wifi_config.sta.password,  pass, sizeof(wifi_config.sta.password) - 1);
    wifi_config.sta.threshold.authmode = WIFI_AUTH_WPA2_PSK;
    wifi_config.sta.pmf_cfg.capable    = true;
    wifi_config.sta.pmf_cfg.required   = false;

    /* Scrub password from stack after configuring */
    ESP_ERROR_CHECK(esp_wifi_set_mode(WIFI_MODE_STA));
    ESP_ERROR_CHECK(esp_wifi_set_config(WIFI_IF_STA, &wifi_config));
    memset(pass, 0, sizeof(pass));
    memset((char *)wifi_config.sta.password, 0, sizeof(wifi_config.sta.password));

    ESP_ERROR_CHECK(esp_wifi_start());

    /* Block until STA is connected. There is no failure path here — the
     * credentials were validated by the captive portal before being saved
     * to NVS, so a failure means environmental (router down, ISP issue,
     * moved devices). The event handler retries with exponential backoff
     * forever, so we just wait.
     *
     * If the user legitimately needs to change WiFi creds (moved house,
     * switched ISP) they hold the BOOT button for 5 seconds to trigger
     * factory reset — see factory_reset.c. */
    xEventGroupWaitBits(
        s_wifi_event_group,
        WIFI_CONNECTED_BIT,
        pdFALSE,
        pdFALSE,
        portMAX_DELAY);

    ESP_LOGI(TAG, "WiFi connected");

    /* Leave the event handlers registered: the wifi_event_handler is still
     * responsible for reconnecting on transient drops during normal
     * operation. Do NOT unregister them or the device will sit dead on
     * the first ISP outage. */
    (void)inst_any_id;
    (void)inst_got_ip;
}

/* ------------------------------------------------------------------ */
/* Incoming message handler                                              */
/* ------------------------------------------------------------------ */

static void on_message(const dice_message_t *msg)
{
    switch (msg->type) {
    case DICE_MSG_JOB_ASSIGN:
        ESP_LOGI(TAG, "Received JobAssignment (round_seq=%lu)",
                 (unsigned long)msg->job_assign.round_seq);
        dice_cr_handle_job(msg->job_assign.request_id,
                           msg->job_assign.round_seq,
                           msg->job_assign.deadline_ts);
        break;

    case DICE_MSG_ROUND_RESULT:
        ESP_LOGI(TAG, "Round result received: status=%s",
                 msg->round_result.status);

        /* If the result signals reveal phase, trigger reveal */
        if (strncmp(msg->round_result.status, "reveal", 6) == 0) {
            dice_cr_do_reveal(msg->round_result.request_id);
        }
        break;

    case DICE_MSG_HEARTBEAT:
    case DICE_MSG_COMMIT:
    case DICE_MSG_REVEAL:
        /* These are node→coordinator messages; ignore if received */
        ESP_LOGW(TAG, "Unexpected message type %u received from coordinator",
                 msg->type);
        break;

    default:
        ESP_LOGW(TAG, "Unknown message type %u", msg->type);
        break;
    }
}

/* ------------------------------------------------------------------ */
/* app_main                                                              */
/* ------------------------------------------------------------------ */

void app_main(void)
{
    ESP_LOGI(TAG, "DICE firmware starting up");

    /* ----------------------------------------------------------------
     * 1. Initialize NVS flash
     * -------------------------------------------------------------- */
    esp_err_t err = nvs_flash_init();
    if (err == ESP_ERR_NVS_NO_FREE_PAGES ||
        err == ESP_ERR_NVS_NEW_VERSION_FOUND) {
        ESP_LOGW(TAG, "NVS partition issue (%s), erasing and reinitialising",
                 esp_err_to_name(err));
        ESP_ERROR_CHECK(nvs_flash_erase());
        err = nvs_flash_init();
    }
    ESP_ERROR_CHECK(err);
    ESP_LOGI(TAG, "NVS flash initialised");

    /* ----------------------------------------------------------------
     * 1b. Initialize LED status indicators
     * -------------------------------------------------------------- */
    dice_led_init();

    /* ----------------------------------------------------------------
     * 1c. First-boot detection — if not provisioned, start captive portal
     *     This blocks until the user configures WiFi via the web UI.
     *     After config is saved, the device reboots into normal mode.
     * -------------------------------------------------------------- */
    if (!dice_is_provisioned()) {
        ESP_LOGI(TAG, "Device not provisioned — starting captive portal");
        dice_captive_portal_start();
        /* Never reaches here — captive_portal_start blocks until reboot */
    }
    ESP_LOGI(TAG, "Device is provisioned — normal boot");

    /* ----------------------------------------------------------------
     * 2. Initialise crypto (loads device key from NVS)
     *    Done before WiFi so the key is in RAM before network is up.
     * -------------------------------------------------------------- */
    dice_led_set(LED_STATUS_CONNECTING);

    if (!dice_crypto_init()) {
        ESP_LOGE(TAG, "Crypto init FAILED — halting");
        dice_led_set(LED_STATUS_ERROR);
        abort();
    }
    ESP_LOGI(TAG, "Crypto initialised");

    /* ----------------------------------------------------------------
     * 3. Entropy self-test — halt if it fails
     * -------------------------------------------------------------- */
    if (!dice_entropy_selftest()) {
        ESP_LOGE(TAG, "Entropy self-test FAILED — halting");
        dice_led_set(LED_STATUS_ERROR);
        abort();
    }

    /* ----------------------------------------------------------------
     * 4. Connect to WiFi
     * -------------------------------------------------------------- */
    wifi_init_sta();

    /* ----------------------------------------------------------------
     * 5. Read coordinator URI from NVS
     * -------------------------------------------------------------- */
    nvs_handle_t nvs_handle;
    ESP_ERROR_CHECK(nvs_open("dice", NVS_READONLY, &nvs_handle));

    char coord_uri[256] = {0};
    size_t uri_len = sizeof(coord_uri);
    err = nvs_get_str(nvs_handle, "coordinator_uri", coord_uri, &uri_len);
    nvs_close(nvs_handle);

    if (err != ESP_OK) {
        ESP_LOGE(TAG, "Failed to read coordinator_uri from NVS: %s",
                 esp_err_to_name(err));
        abort();
    }
    ESP_LOGI(TAG, "Coordinator URI: %s", coord_uri);

    /* ----------------------------------------------------------------
     * 6. Connect WebSocket to coordinator
     * -------------------------------------------------------------- */
    if (!dice_ws_connect(coord_uri, on_message)) {
        ESP_LOGE(TAG, "WebSocket connect failed — halting");
        dice_led_set(LED_STATUS_ERROR);
        abort();
    }
    ESP_LOGI(TAG, "WebSocket client started");
    dice_led_set(LED_STATUS_ONLINE);

    /* ----------------------------------------------------------------
     * 7. Start heartbeat timer
     * -------------------------------------------------------------- */
    dice_heartbeat_start();
    ESP_LOGI(TAG, "Heartbeat started");

    /* ----------------------------------------------------------------
     * 7b. Start factory reset monitor (BOOT button watchdog)
     * -------------------------------------------------------------- */
    dice_factory_reset_start();
    ESP_LOGI(TAG, "Factory reset monitor started");

    /* ----------------------------------------------------------------
     * 8. Main loop — all work is done by FreeRTOS tasks/timers.
     *    We just feed the watchdog and log periodic status here.
     * -------------------------------------------------------------- */
    ESP_LOGI(TAG, "DICE firmware running — entering main loop");

    while (1) {
        vTaskDelay(pdMS_TO_TICKS(60000));  /* 60-second sleep */
        ESP_LOGI(TAG, "Alive. Uptime: %llu s, WS connected: %s",
                 (unsigned long long)(esp_timer_get_time() / 1000000LL),
                 dice_ws_is_connected() ? "yes" : "no");
    }
}
