#include "entropy.h"

#include <string.h>
#include <stdint.h>
#include <stdbool.h>

#include "esp_random.h"
#include "esp_log.h"
#include "esp_timer.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"

/* ADC includes — ESP-IDF v5.x one-shot driver */
#include "esp_adc/adc_oneshot.h"

#include "mbedtls/sha256.h"

static const char *TAG = "dice_entropy";

/* ADC channel used for floating-pin noise.
 * ADC1_CHANNEL_0 maps to GPIO1 on ESP32-S3.
 * Leave the pin unconnected so it picks up thermal/EMI noise. */
#define ENTROPY_ADC_CHANNEL  ADC_CHANNEL_0
#define ENTROPY_ADC_UNIT     ADC_UNIT_1
#define ENTROPY_ADC_SAMPLES  8

/* ------------------------------------------------------------------ */
/* Internal helpers                                                     */
/* ------------------------------------------------------------------ */

/**
 * Collect raw ADC noise from a floating pin and fold all 8 samples
 * into a 32-bit value (XOR of samples at different byte positions).
 */
static uint32_t collect_adc_noise(void)
{
    adc_oneshot_unit_handle_t handle = NULL;
    adc_oneshot_unit_init_cfg_t unit_cfg = {
        .unit_id = ENTROPY_ADC_UNIT,
    };
    adc_oneshot_chan_cfg_t chan_cfg = {
        .atten    = ADC_ATTEN_DB_12,   /* full 0–3.3 V range */
        .bitwidth = ADC_BITWIDTH_12,
    };

    uint32_t noise = 0;

    if (adc_oneshot_new_unit(&unit_cfg, &handle) != ESP_OK) {
        ESP_LOGW(TAG, "ADC init failed, skipping ADC noise source");
        /* Non-fatal: we still have TRNG + tick */
        return (uint32_t)xTaskGetTickCount();
    }

    if (adc_oneshot_config_channel(handle, ENTROPY_ADC_CHANNEL, &chan_cfg) != ESP_OK) {
        ESP_LOGW(TAG, "ADC channel config failed");
        adc_oneshot_del_unit(handle);
        return (uint32_t)xTaskGetTickCount();
    }

    for (int i = 0; i < ENTROPY_ADC_SAMPLES; i++) {
        int raw = 0;
        if (adc_oneshot_read(handle, ENTROPY_ADC_CHANNEL, &raw) == ESP_OK) {
            /* Rotate noise left by 4 bits each iteration then XOR */
            noise = ((noise << 4) | (noise >> 28)) ^ (uint32_t)raw;
        }
    }

    adc_oneshot_del_unit(handle);
    return noise;
}

/* ------------------------------------------------------------------ */
/* Public API                                                           */
/* ------------------------------------------------------------------ */

bool dice_entropy_generate(uint8_t entropy_out[32])
{
    /* --- Source 1: Hardware TRNG (32 bytes) --- */
    uint8_t trng_buf[32];
    esp_fill_random(trng_buf, sizeof(trng_buf));

    /* --- Source 2: ADC floating-pin noise (4 bytes folded to 32) --- */
    uint32_t adc_noise = collect_adc_noise();
    uint8_t adc_buf[32];
    memset(adc_buf, 0, sizeof(adc_buf));
    /* Distribute 4 noise bytes across the buffer */
    for (int i = 0; i < 32; i++) {
        adc_buf[i] = (uint8_t)((adc_noise >> ((i % 4) * 8)) & 0xFF);
    }

    /* --- Source 3: FreeRTOS tick count (timing jitter) --- */
    TickType_t tick = xTaskGetTickCount();
    uint8_t tick_buf[32];
    memset(tick_buf, 0, sizeof(tick_buf));
    for (int i = 0; i < 4; i++) {
        tick_buf[i] = (uint8_t)((tick >> (i * 8)) & 0xFF);
    }
    /* Also fold esp_timer microseconds into tick_buf[4..7] */
    uint64_t us = (uint64_t)esp_timer_get_time();
    for (int i = 0; i < 8; i++) {
        tick_buf[4 + i] = (uint8_t)((us >> (i * 8)) & 0xFF);
    }

    /* --- Mix: XOR all three sources --- */
    uint8_t mixed[32];
    for (int i = 0; i < 32; i++) {
        mixed[i] = trng_buf[i] ^ adc_buf[i] ^ tick_buf[i];
    }

    /* --- Finalise: SHA-256(mixed) → entropy_out --- */
    int ret = mbedtls_sha256(mixed, sizeof(mixed), entropy_out, 0 /* is224=0 */);

    /* Scrub intermediate buffers */
    memset(trng_buf, 0, sizeof(trng_buf));
    memset(adc_buf,  0, sizeof(adc_buf));
    memset(tick_buf, 0, sizeof(tick_buf));
    memset(mixed,    0, sizeof(mixed));

    if (ret != 0) {
        ESP_LOGE(TAG, "SHA-256 failed: -0x%04X", (unsigned)(-ret));
        return false;
    }

    return true;
}

bool dice_entropy_selftest(void)
{
    ESP_LOGI(TAG, "Running entropy self-test (10 samples)...");

#define SELFTEST_SAMPLES 10
    uint8_t samples[SELFTEST_SAMPLES][32];

    for (int i = 0; i < SELFTEST_SAMPLES; i++) {
        if (!dice_entropy_generate(samples[i])) {
            ESP_LOGE(TAG, "Self-test: entropy_generate failed at sample %d", i);
            return false;
        }
    }

    /* Check no two samples are identical */
    for (int i = 0; i < SELFTEST_SAMPLES; i++) {
        for (int j = i + 1; j < SELFTEST_SAMPLES; j++) {
            if (memcmp(samples[i], samples[j], 32) == 0) {
                ESP_LOGE(TAG, "Self-test FAILED: samples %d and %d are identical", i, j);
                /* Scrub samples before returning */
                memset(samples, 0, sizeof(samples));
                return false;
            }
        }
    }

    /* Sanity: first sample must not be all zeros */
    bool all_zero = true;
    for (int i = 0; i < 32; i++) {
        if (samples[0][i] != 0) { all_zero = false; break; }
    }
    if (all_zero) {
        ESP_LOGE(TAG, "Self-test FAILED: first sample is all zeros");
        memset(samples, 0, sizeof(samples));
        return false;
    }

    memset(samples, 0, sizeof(samples));
    ESP_LOGI(TAG, "Entropy self-test PASSED");
    return true;
#undef SELFTEST_SAMPLES
}
