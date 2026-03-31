#include "led_status.h"
#include "driver/gpio.h"
#include "esp_log.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "freertos/timers.h"
#include "led_strip.h"

static const char *TAG = "dice_led";

/* ESP32-S3-N16R8 onboard addressable LED on GPIO48 */
#define LED_GPIO 48
#define LED_COUNT 1

static led_strip_handle_t s_led_strip = NULL;
static led_status_t s_current_status = LED_STATUS_OFF;
static TimerHandle_t s_blink_timer = NULL;
static bool s_blink_on = true;

/* Color definitions (R, G, B) — brightness scaled to ~20% to avoid blinding */
static void set_rgb(uint8_t r, uint8_t g, uint8_t b)
{
    if (s_led_strip) {
        led_strip_set_pixel(s_led_strip, 0, r, g, b);
        led_strip_refresh(s_led_strip);
    }
}

static void blink_callback(TimerHandle_t timer)
{
    s_blink_on = !s_blink_on;
    if (s_current_status == LED_STATUS_ACTIVE) {
        if (s_blink_on) {
            set_rgb(0, 40, 0); /* Green on */
        } else {
            set_rgb(0, 0, 0);  /* Off */
        }
    }
}

void dice_led_init(void)
{
    /* Configure LED strip (WS2812 on GPIO48) */
    led_strip_config_t strip_config = {
        .strip_gpio_num = LED_GPIO,
        .max_leds = LED_COUNT,
        .led_pixel_format = LED_PIXEL_FORMAT_GRB,
        .led_model = LED_MODEL_WS2812,
    };
    led_strip_rmt_config_t rmt_config = {
        .resolution_hz = 10 * 1000 * 1000, /* 10 MHz */
    };

    esp_err_t err = led_strip_new_rmt_device(&strip_config, &rmt_config, &s_led_strip);
    if (err != ESP_OK) {
        ESP_LOGW(TAG, "LED strip init failed (%s) — LED indicators disabled",
                 esp_err_to_name(err));
        s_led_strip = NULL;
        return;
    }

    /* Create blink timer (500ms period for active status) */
    s_blink_timer = xTimerCreate("led_blink", pdMS_TO_TICKS(500),
                                  pdTRUE, NULL, blink_callback);

    led_strip_clear(s_led_strip);
    ESP_LOGI(TAG, "LED status indicators initialised (GPIO%d)", LED_GPIO);
}

void dice_led_set(led_status_t status)
{
    s_current_status = status;

    /* Stop blink timer unless we're in ACTIVE mode */
    if (s_blink_timer && status != LED_STATUS_ACTIVE) {
        xTimerStop(s_blink_timer, 0);
        s_blink_on = true;
    }

    switch (status) {
    case LED_STATUS_SETUP:
        set_rgb(0, 0, 50);     /* Blue */
        break;
    case LED_STATUS_CONNECTING:
        set_rgb(50, 30, 0);    /* Yellow */
        break;
    case LED_STATUS_ONLINE:
        set_rgb(0, 40, 0);     /* Green */
        break;
    case LED_STATUS_ACTIVE:
        set_rgb(0, 40, 0);     /* Green (blink timer will toggle) */
        if (s_blink_timer) {
            xTimerStart(s_blink_timer, 0);
        }
        break;
    case LED_STATUS_ERROR:
        set_rgb(50, 0, 0);     /* Red */
        break;
    case LED_STATUS_OFF:
    default:
        set_rgb(0, 0, 0);      /* Off */
        break;
    }
}
