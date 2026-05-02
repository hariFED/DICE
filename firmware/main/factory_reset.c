#include "factory_reset.h"
#include "led_status.h"
#include "payout_binding.h"

#include "driver/gpio.h"
#include "esp_log.h"
#include "esp_system.h"
#include "nvs.h"
#include "nvs_flash.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"

static const char *TAG = "dice_reset";

/* Poll the BOOT button every 100 ms. At this interval, the 5-second hold
 * corresponds to 50 consecutive LOW reads. */
#define POLL_INTERVAL_MS 100
#define HOLD_COUNT_TARGET ((FACTORY_RESET_HOLD_SECS * 1000) / POLL_INTERVAL_MS)

/* Erase the user-provided portion of NVS. Called after a confirmed long-press.
 * Returns true on success. */
static bool erase_user_nvs(void)
{
    nvs_handle_t nvs;
    esp_err_t    err = nvs_open("dice", NVS_READWRITE, &nvs);
    if (err != ESP_OK) {
        ESP_LOGE(TAG, "nvs_open failed: %s", esp_err_to_name(err));
        return false;
    }

    /* Best-effort erase of each user key. Ignore ESP_ERR_NVS_NOT_FOUND
     * because a fresh device will not have these keys set. */
    esp_err_t e1 = nvs_erase_key(nvs, "wifi_ssid");
    esp_err_t e2 = nvs_erase_key(nvs, "wifi_pass");
    esp_err_t e3 = nvs_erase_key(nvs, "sol_wallet");

    if (e1 != ESP_OK && e1 != ESP_ERR_NVS_NOT_FOUND) {
        ESP_LOGW(TAG, "erase wifi_ssid: %s", esp_err_to_name(e1));
    }
    if (e2 != ESP_OK && e2 != ESP_ERR_NVS_NOT_FOUND) {
        ESP_LOGW(TAG, "erase wifi_pass: %s", esp_err_to_name(e2));
    }
    if (e3 != ESP_OK && e3 != ESP_ERR_NVS_NOT_FOUND) {
        ESP_LOGW(TAG, "erase sol_wallet: %s", esp_err_to_name(e3));
    }

    esp_err_t commit_err = nvs_commit(nvs);
    nvs_close(nvs);

    if (commit_err != ESP_OK) {
        ESP_LOGE(TAG, "nvs_commit failed: %s", esp_err_to_name(commit_err));
        return false;
    }

    /* Clear the payout binding "already sent" flag so the next boot
     * re-sends the binding for whatever new wallet the operator enters
     * in the captive portal. This lives in the same NVS namespace and
     * is best-effort — we've already committed the wifi/wallet wipe. */
    dice_payout_binding_clear_flag();

    return true;
}

static void factory_reset_task(void *arg)
{
    (void)arg;

    /* Configure the BOOT button as input with pull-up. DevKitC boards
     * have an external pull-up on GPIO 0 but the internal one is a
     * cheap belt-and-suspenders. */
    gpio_config_t cfg = {
        .pin_bit_mask = 1ULL << FACTORY_RESET_GPIO,
        .mode         = GPIO_MODE_INPUT,
        .pull_up_en   = GPIO_PULLUP_ENABLE,
        .pull_down_en = GPIO_PULLDOWN_DISABLE,
        .intr_type    = GPIO_INTR_DISABLE,
    };
    esp_err_t err = gpio_config(&cfg);
    if (err != ESP_OK) {
        ESP_LOGE(TAG, "gpio_config failed: %s — factory reset DISABLED",
                 esp_err_to_name(err));
        vTaskDelete(NULL);
        return;
    }

    ESP_LOGI(TAG, "Factory reset monitor running — hold BOOT %d s to wipe WiFi/wallet",
             FACTORY_RESET_HOLD_SECS);

    int  hold_count      = 0;
    bool warning_flashed = false;

    while (1) {
        vTaskDelay(pdMS_TO_TICKS(POLL_INTERVAL_MS));

        int level = gpio_get_level(FACTORY_RESET_GPIO);
        if (level == 0) {
            /* Button is being held. */
            hold_count++;

            /* Visual feedback: after 2 seconds held, switch LED to the
             * error (red) state so the user knows they're about to
             * trigger the reset. */
            if (!warning_flashed && hold_count >= (2000 / POLL_INTERVAL_MS)) {
                dice_led_set(LED_STATUS_ERROR);
                warning_flashed = true;
                ESP_LOGW(TAG, "BOOT held — release to cancel, keep holding to reset");
            }

            if (hold_count >= HOLD_COUNT_TARGET) {
                ESP_LOGW(TAG, "FACTORY RESET triggered — erasing WiFi/wallet and rebooting");
                if (erase_user_nvs()) {
                    ESP_LOGI(TAG, "user NVS erased; rebooting into captive portal");
                } else {
                    ESP_LOGE(TAG, "NVS erase failed — rebooting anyway, device may still be provisioned");
                }
                vTaskDelay(pdMS_TO_TICKS(500));
                esp_restart();
            }
        } else {
            /* Button released — reset the counter and restore LED if
             * we had flashed the warning. */
            if (hold_count > 0) {
                if (warning_flashed) {
                    /* Restore to the "online" status. We don't know exactly
                     * which state we're supposed to be in, but ONLINE is the
                     * safe default; the WS client and heartbeat will
                     * re-assert it on the next loop if needed. */
                    dice_led_set(LED_STATUS_ONLINE);
                    warning_flashed = false;
                }
                hold_count = 0;
            }
        }
    }
}

void dice_factory_reset_start(void)
{
    BaseType_t ok = xTaskCreate(
        factory_reset_task,
        "factory_reset",
        3072,
        NULL,
        tskIDLE_PRIORITY + 1,
        NULL
    );
    if (ok != pdPASS) {
        ESP_LOGE(TAG, "Failed to spawn factory reset task");
    }
}
