#pragma once

/**
 * @file heartbeat.h
 * @brief Periodic heartbeat sender for DICE nodes.
 *
 * Fires every 25 seconds via a FreeRTOS software timer.
 * Each heartbeat includes uptime, latency, and job counters.
 */

/**
 * Start the heartbeat timer.
 *
 * Must be called after dice_ws_connect() and dice_crypto_init().
 * The timer is auto-reload: once started it runs until dice_heartbeat_stop().
 */
void dice_heartbeat_start(void);

/**
 * Stop the heartbeat timer.
 */
void dice_heartbeat_stop(void);

/**
 * Increment the completed-jobs counter (called by commit_reveal on success).
 */
void dice_heartbeat_job_completed(void);
