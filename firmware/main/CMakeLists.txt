idf_component_register(
    SRCS "app_main.c" "entropy.c" "crypto.c" "websocket_client.c" "heartbeat.c" "commit_reveal.c"
    INCLUDE_DIRS "."
    REQUIRES
        esp_wifi
        nvs_flash
        esp_tls
        esp_websocket_client
        mbedtls
        esp_timer
        freertos
        esp_adc
        dice_protocol
)
