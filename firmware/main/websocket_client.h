#pragma once
#include <stdbool.h>
#include "dice_protocol.h"

/**
 * @file websocket_client.h
 * @brief mTLS WebSocket client for DICE coordinator communication.
 *
 * Connects to the coordinator via wss:// with mutual TLS authentication.
 * Device cert, device key, and CA cert are loaded from NVS.
 *
 * Reconnection uses exponential backoff: 1 s, 2 s, 4 s, 8 s, ... max 60 s.
 */

/**
 * Callback type invoked when a complete DICE message is received.
 * The message pointer is valid only for the duration of the callback.
 */
typedef void (*dice_message_handler_t)(const dice_message_t *msg);

/**
 * Connect to the coordinator WebSocket endpoint with mTLS.
 *
 * Loads from NVS:
 *   - "dice"/"client_cert_pem" : device certificate (PEM, null-terminated)
 *   - "dice"/"client_key_pem"  : device private key  (PEM, null-terminated)
 *   - "dice"/"ca_cert_pem"     : CA certificate      (PEM, null-terminated)
 *
 * Spawns a FreeRTOS task to maintain the connection and call handler
 * whenever a message arrives.  Returns immediately after starting the task.
 *
 * @param uri      WebSocket URI, e.g. "wss://coordinator.dice.io/ws"
 * @param handler  Callback invoked for each incoming DICE message.
 * @return true if the connection task was started successfully.
 */
bool dice_ws_connect(const char *uri, dice_message_handler_t handler);

/**
 * Encode and send a DICE message to the coordinator.
 *
 * Thread-safe; may be called from any FreeRTOS task.
 *
 * @param msg  Message to send.
 * @return true if the message was sent successfully.
 */
bool dice_ws_send(const dice_message_t *msg);

/**
 * Disconnect from the coordinator and stop the reconnection task.
 */
void dice_ws_disconnect(void);

/**
 * Returns true if the WebSocket is currently connected.
 */
bool dice_ws_is_connected(void);
