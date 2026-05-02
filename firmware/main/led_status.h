#pragma once

/**
 * LED status indicators for ESP32-S3-N16R8.
 *
 * Uses the onboard RGB LED (GPIO48 / addressable WS2812).
 *
 *   Blue       = Setup mode (captive portal active)
 *   Yellow     = Connecting to WiFi / coordinator
 *   Green      = Online, waiting for jobs
 *   Green blink = Actively participating in a round
 *   Red        = Error (no WiFi, coordinator unreachable)
 */

typedef enum {
    LED_STATUS_SETUP,       /* Blue — captive portal active */
    LED_STATUS_CONNECTING,  /* Yellow — connecting to WiFi/coordinator */
    LED_STATUS_ONLINE,      /* Green — connected, idle */
    LED_STATUS_ACTIVE,      /* Green blink — processing a round */
    LED_STATUS_ERROR,       /* Red — error state */
    LED_STATUS_OFF,         /* All LEDs off */
} led_status_t;

/**
 * Initialize the LED GPIO and RMT driver for WS2812.
 */
void dice_led_init(void);

/**
 * Set the current LED status. Thread-safe.
 */
void dice_led_set(led_status_t status);
