#pragma once

/**
 * Factory reset — hold the BOOT button for FACTORY_RESET_HOLD_SECS to
 * erase the user-provided portion of NVS (wifi_ssid, wifi_pass,
 * sol_wallet) and reboot into the captive portal.
 *
 * What this preserves (so a device doesn't get bricked by factory reset):
 *   - coordinator_uri
 *   - node_name
 *   - priv_key_der      (VRF signing key — never regenerate after ship)
 *   - client_cert_pem   (mTLS cert — never regenerate after ship)
 *   - client_key_pem
 *   - ca_cert_pem
 *
 * What this erases:
 *   - wifi_ssid
 *   - wifi_pass
 *   - sol_wallet
 *
 * Use case: an end user wants to move the device to a new WiFi network
 * (new ISP, new home, new password) or change the Solana wallet the
 * device is attributed to.
 *
 * GPIO 0 is the BOOT button on every ESP32-S3 DevKitC / N16R8 variant.
 * It is active LOW (pressed = 0).
 */

#define FACTORY_RESET_GPIO      0      /* BOOT button on DevKitC */
#define FACTORY_RESET_HOLD_SECS 5      /* hold duration to trigger reset */

/**
 * Spawn the factory reset monitor task. Must be called once at startup,
 * AFTER the GPIO driver and LED subsystem are initialized.
 *
 * The task runs forever at a low priority (~tskIDLE_PRIORITY + 1),
 * polling the button every 100 ms.
 */
void dice_factory_reset_start(void);
