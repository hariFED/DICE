#pragma once

#include <stdbool.h>

/**
 * Captive Portal for first-boot WiFi + wallet configuration.
 *
 * When no WiFi credentials are found in NVS, the device:
 *   1. Creates WiFi AP: "DICE-XXXX" (last 4 hex of device MAC)
 *   2. Starts HTTP server on 192.168.4.1
 *   3. Serves setup page for WiFi + wallet configuration
 *   4. DNS redirects all queries to 192.168.4.1 (captive portal auto-open)
 *   5. On save: writes to NVS, reboots into station mode
 */

/**
 * Start the captive portal.
 * Creates AP, starts HTTP server and DNS server.
 * Blocks until user completes setup and device reboots.
 */
void dice_captive_portal_start(void);

/**
 * Check if WiFi credentials exist in NVS.
 * Returns true if both wifi_ssid and wifi_pass are set.
 */
bool dice_is_provisioned(void);
