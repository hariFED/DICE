#include "captive_portal.h"
#include "led_status.h"

#include <string.h>
#include <stdlib.h>
#include "esp_log.h"
#include "esp_wifi.h"
#include "esp_event.h"
#include "esp_netif.h"
#include "esp_http_server.h"
#include "esp_mac.h"
#include "nvs_flash.h"
#include "nvs.h"
#include "lwip/sockets.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"

static const char *TAG = "dice_portal";

/* ------------------------------------------------------------------ */
/* DNS redirect task — redirects ALL DNS queries to our IP             */
/* ------------------------------------------------------------------ */

#define DNS_PORT 53
#define DNS_MAX_LEN 512

static bool s_dns_running = false;

static void dns_redirect_task(void *arg)
{
    struct sockaddr_in server_addr = {
        .sin_family = AF_INET,
        .sin_port = htons(DNS_PORT),
        .sin_addr.s_addr = htonl(INADDR_ANY),
    };

    int sock = socket(AF_INET, SOCK_DGRAM, IPPROTO_UDP);
    if (sock < 0) {
        ESP_LOGE(TAG, "DNS socket failed");
        vTaskDelete(NULL);
        return;
    }

    if (bind(sock, (struct sockaddr *)&server_addr, sizeof(server_addr)) < 0) {
        ESP_LOGE(TAG, "DNS bind failed");
        close(sock);
        vTaskDelete(NULL);
        return;
    }

    ESP_LOGI(TAG, "DNS redirect server started on port %d", DNS_PORT);
    s_dns_running = true;

    uint8_t rx_buf[DNS_MAX_LEN];
    uint8_t tx_buf[DNS_MAX_LEN];

    while (s_dns_running) {
        struct sockaddr_in client_addr;
        socklen_t addr_len = sizeof(client_addr);
        int len = recvfrom(sock, rx_buf, sizeof(rx_buf), 0,
                           (struct sockaddr *)&client_addr, &addr_len);
        if (len < 12) continue; /* DNS header is 12 bytes minimum */

        /* Build minimal DNS response:
         * Copy original header, set response flags, append answer pointing to 192.168.4.1 */
        memcpy(tx_buf, rx_buf, len);

        /* Set QR=1 (response), AA=1 (authoritative), RA=1, RCODE=0 (no error) */
        tx_buf[2] = 0x85; /* QR=1, AA=1, Opcode=0 */
        tx_buf[3] = 0x80; /* RA=1, RCODE=0 */

        /* ANCOUNT = 1 */
        tx_buf[6] = 0x00;
        tx_buf[7] = 0x01;

        /* Append answer record after the query section */
        int offset = len;

        /* Name pointer to query name (0xC00C = pointer to offset 12) */
        tx_buf[offset++] = 0xC0;
        tx_buf[offset++] = 0x0C;

        /* Type A (1) */
        tx_buf[offset++] = 0x00;
        tx_buf[offset++] = 0x01;

        /* Class IN (1) */
        tx_buf[offset++] = 0x00;
        tx_buf[offset++] = 0x01;

        /* TTL = 60 seconds */
        tx_buf[offset++] = 0x00;
        tx_buf[offset++] = 0x00;
        tx_buf[offset++] = 0x00;
        tx_buf[offset++] = 0x3C;

        /* RDLENGTH = 4 (IPv4) */
        tx_buf[offset++] = 0x00;
        tx_buf[offset++] = 0x04;

        /* RDATA = 192.168.4.1 */
        tx_buf[offset++] = 192;
        tx_buf[offset++] = 168;
        tx_buf[offset++] = 4;
        tx_buf[offset++] = 1;

        sendto(sock, tx_buf, offset, 0,
               (struct sockaddr *)&client_addr, addr_len);
    }

    close(sock);
    vTaskDelete(NULL);
}

/* ------------------------------------------------------------------ */
/* Setup page HTML (embedded in firmware)                              */
/* ------------------------------------------------------------------ */

static const char SETUP_HTML[] =
    "<!DOCTYPE html>"
    "<html><head>"
    "<meta charset='utf-8'>"
    "<meta name='viewport' content='width=device-width,initial-scale=1'>"
    "<title>DICE Node Setup</title>"
    "<style>"
    "*{box-sizing:border-box;margin:0;padding:0}"
    "body{font-family:-apple-system,sans-serif;background:#0d0d0d;color:#e0e0e0;padding:1.5rem}"
    "h1{color:#00ff88;font-size:1.4rem;margin-bottom:.5rem}"
    ".sub{color:#555;font-size:.8rem;margin-bottom:1.5rem}"
    ".card{background:#111;border:1px solid #1e1e1e;border-radius:8px;padding:1.25rem;margin-bottom:1rem}"
    "label{display:block;color:#aaa;font-size:.75rem;text-transform:uppercase;letter-spacing:.1em;margin-bottom:.4rem}"
    "input{width:100%%;padding:.6rem;background:#1a1a1a;border:1px solid #333;border-radius:4px;color:#fff;font-size:.9rem;margin-bottom:1rem}"
    "input:focus{outline:none;border-color:#00ff88}"
    "button{width:100%%;padding:.7rem;background:#00ff88;color:#000;border:none;border-radius:4px;font-size:1rem;font-weight:bold;cursor:pointer}"
    "button:hover{background:#00cc66}"
    ".info{color:#555;font-size:.75rem;margin-top:.5rem}"
    ".status{color:#00ff88;font-size:.8rem;margin-top:.5rem}"
    "#result{margin-top:1rem;font-size:.85rem}"
    ".err{color:#ff4444}"
    "</style>"
    "</head><body>"
    "<h1>DICE Node Setup</h1>"
    "<p class='sub'>Hardware-backed VRF Oracle — Device Configuration</p>"
    "<div class='card'>"
    "<label>WiFi Network Name (SSID)</label>"
    "<input id='ssid' type='text' placeholder='Your WiFi network name' maxlength='32'>"
    "<label>WiFi Password</label>"
    "<input id='pass' type='password' placeholder='WiFi password' maxlength='64'>"
    "<label>Solana Wallet Address</label>"
    "<input id='wallet' type='text' placeholder='Your Solana wallet (e.g. 7xKX...)' maxlength='44'>"
    "<button onclick='save()'>Save &amp; Connect</button>"
    "<div id='result'></div>"
    "</div>"
    "<div class='card'>"
    "<label>Device Info</label>"
    "<p class='info'>Device ID: <span id='devid'>loading...</span></p>"
    "<p class='info'>Firmware: v1.0.0</p>"
    "<p class='info'>Board: ESP32-S3-N16R8</p>"
    "</div>"
    "<script>"
    "fetch('/info').then(r=>r.json()).then(d=>{"
    "document.getElementById('devid').textContent=d.device_id;"
    "});"
    "function save(){"
    "var s=document.getElementById('ssid').value;"
    "var p=document.getElementById('pass').value;"
    "var w=document.getElementById('wallet').value;"
    "var r=document.getElementById('result');"
    "if(!s||!p){r.innerHTML='<span class=\"err\">WiFi name and password required</span>';return}"
    "if(!w||w.length<32){r.innerHTML='<span class=\"err\">Valid Solana wallet address required</span>';return}"
    "r.innerHTML='<span class=\"status\">Saving configuration...</span>';"
    "fetch('/save',{method:'POST',headers:{'Content-Type':'application/json'},"
    "body:JSON.stringify({ssid:s,pass:p,wallet:w})})"
    ".then(r=>r.json()).then(d=>{"
    "if(d.ok){r.innerHTML='<span class=\"status\">Saved! Rebooting in 3 seconds...</span>'}"
    "else{r.innerHTML='<span class=\"err\">Error: '+d.error+'</span>'}"
    "}).catch(e=>{r.innerHTML='<span class=\"err\">Network error</span>'})}"
    "</script>"
    "</body></html>";

/* ------------------------------------------------------------------ */
/* HTTP handlers                                                       */
/* ------------------------------------------------------------------ */

static char s_device_id[20] = {0};

static esp_err_t handle_root(httpd_req_t *req)
{
    httpd_resp_set_type(req, "text/html");
    httpd_resp_send(req, SETUP_HTML, HTTPD_RESP_USE_STRLEN);
    return ESP_OK;
}

/* Redirect any unknown path to root (captive portal behavior) */
static esp_err_t handle_redirect(httpd_req_t *req)
{
    httpd_resp_set_status(req, "302 Found");
    httpd_resp_set_hdr(req, "Location", "http://192.168.4.1/");
    httpd_resp_send(req, NULL, 0);
    return ESP_OK;
}

static esp_err_t handle_info(httpd_req_t *req)
{
    char json[128];
    snprintf(json, sizeof(json), "{\"device_id\":\"%s\",\"firmware\":\"1.0.0\"}", s_device_id);
    httpd_resp_set_type(req, "application/json");
    httpd_resp_send(req, json, HTTPD_RESP_USE_STRLEN);
    return ESP_OK;
}

static esp_err_t handle_save(httpd_req_t *req)
{
    char buf[256] = {0};
    int ret = httpd_req_recv(req, buf, sizeof(buf) - 1);
    if (ret <= 0) {
        httpd_resp_send_err(req, HTTPD_400_BAD_REQUEST, "No data");
        return ESP_FAIL;
    }

    /* Simple JSON parsing (no external library needed for 3 fields) */
    char ssid[33] = {0};
    char pass[65] = {0};
    char wallet[45] = {0};

    /* Extract "ssid":"value" */
    char *p = strstr(buf, "\"ssid\":\"");
    if (p) {
        p += 8;
        char *end = strchr(p, '"');
        if (end && (end - p) < (int)sizeof(ssid)) {
            memcpy(ssid, p, end - p);
        }
    }

    p = strstr(buf, "\"pass\":\"");
    if (p) {
        p += 8;
        char *end = strchr(p, '"');
        if (end && (end - p) < (int)sizeof(pass)) {
            memcpy(pass, p, end - p);
        }
    }

    p = strstr(buf, "\"wallet\":\"");
    if (p) {
        p += 10;
        char *end = strchr(p, '"');
        if (end && (end - p) < (int)sizeof(wallet)) {
            memcpy(wallet, p, end - p);
        }
    }

    if (strlen(ssid) == 0 || strlen(pass) == 0) {
        httpd_resp_set_type(req, "application/json");
        httpd_resp_send(req, "{\"ok\":false,\"error\":\"SSID and password required\"}", HTTPD_RESP_USE_STRLEN);
        return ESP_OK;
    }

    /* Save to NVS */
    nvs_handle_t nvs;
    esp_err_t err = nvs_open("dice", NVS_READWRITE, &nvs);
    if (err != ESP_OK) {
        httpd_resp_set_type(req, "application/json");
        httpd_resp_send(req, "{\"ok\":false,\"error\":\"NVS open failed\"}", HTTPD_RESP_USE_STRLEN);
        return ESP_OK;
    }

    nvs_set_str(nvs, "wifi_ssid", ssid);
    nvs_set_str(nvs, "wifi_pass", pass);
    if (strlen(wallet) > 0) {
        nvs_set_str(nvs, "sol_wallet", wallet);
    }
    nvs_commit(nvs);
    nvs_close(nvs);

    ESP_LOGI(TAG, "Configuration saved: SSID=%s, wallet=%s", ssid, wallet);

    /* Respond success */
    httpd_resp_set_type(req, "application/json");
    httpd_resp_send(req, "{\"ok\":true}", HTTPD_RESP_USE_STRLEN);

    /* Scrub credentials from stack */
    memset(pass, 0, sizeof(pass));
    memset(buf, 0, sizeof(buf));

    /* Reboot after short delay so response is sent */
    vTaskDelay(pdMS_TO_TICKS(3000));
    esp_restart();

    return ESP_OK;
}

/* ------------------------------------------------------------------ */
/* WiFi AP mode                                                        */
/* ------------------------------------------------------------------ */

static void wifi_init_ap(void)
{
    /* Get MAC for AP name */
    uint8_t mac[6];
    esp_read_mac(mac, ESP_MAC_WIFI_STA);
    snprintf(s_device_id, sizeof(s_device_id), "DICE-%02X%02X", mac[4], mac[5]);

    ESP_ERROR_CHECK(esp_netif_init());
    ESP_ERROR_CHECK(esp_event_loop_create_default());
    esp_netif_create_default_wifi_ap();

    wifi_init_config_t cfg = WIFI_INIT_CONFIG_DEFAULT();
    ESP_ERROR_CHECK(esp_wifi_init(&cfg));

    wifi_config_t wifi_config = {
        .ap = {
            .max_connection = 4,
            .authmode = WIFI_AUTH_OPEN, /* Open network for easy setup */
            .channel = 1,
        },
    };
    strncpy((char *)wifi_config.ap.ssid, s_device_id, sizeof(wifi_config.ap.ssid) - 1);
    wifi_config.ap.ssid_len = strlen(s_device_id);

    ESP_ERROR_CHECK(esp_wifi_set_mode(WIFI_MODE_AP));
    ESP_ERROR_CHECK(esp_wifi_set_config(WIFI_IF_AP, &wifi_config));
    ESP_ERROR_CHECK(esp_wifi_start());

    ESP_LOGI(TAG, "WiFi AP started: %s (open)", s_device_id);
    ESP_LOGI(TAG, "Setup page: http://192.168.4.1/");
}

/* ------------------------------------------------------------------ */
/* Public API                                                          */
/* ------------------------------------------------------------------ */

bool dice_is_provisioned(void)
{
    nvs_handle_t nvs;
    esp_err_t err = nvs_open("dice", NVS_READONLY, &nvs);
    if (err != ESP_OK) return false;

    char ssid[33] = {0};
    size_t len = sizeof(ssid);
    err = nvs_get_str(nvs, "wifi_ssid", ssid, &len);
    nvs_close(nvs);

    return (err == ESP_OK && strlen(ssid) > 0);
}

void dice_captive_portal_start(void)
{
    ESP_LOGI(TAG, "Starting captive portal — device not provisioned");

    /* Set LED to blue (setup mode) */
    dice_led_set(LED_STATUS_SETUP);

    /* Start WiFi AP */
    wifi_init_ap();

    /* Start DNS redirect server */
    xTaskCreate(dns_redirect_task, "dns_redir", 4096, NULL, 5, NULL);

    /* Start HTTP server */
    httpd_config_t config = HTTPD_DEFAULT_CONFIG();
    config.max_uri_handlers = 8;
    config.uri_match_fn = httpd_uri_match_wildcard;

    httpd_handle_t server = NULL;
    ESP_ERROR_CHECK(httpd_start(&server, &config));

    /* Register handlers */
    httpd_uri_t uri_root = {
        .uri = "/", .method = HTTP_GET, .handler = handle_root,
    };
    httpd_uri_t uri_info = {
        .uri = "/info", .method = HTTP_GET, .handler = handle_info,
    };
    httpd_uri_t uri_save = {
        .uri = "/save", .method = HTTP_POST, .handler = handle_save,
    };
    /* Catch-all redirect for captive portal detection */
    httpd_uri_t uri_catchall = {
        .uri = "/*", .method = HTTP_GET, .handler = handle_redirect,
    };

    httpd_register_uri_handler(server, &uri_root);
    httpd_register_uri_handler(server, &uri_info);
    httpd_register_uri_handler(server, &uri_save);
    httpd_register_uri_handler(server, &uri_catchall);

    ESP_LOGI(TAG, "HTTP server started on port 80");
    ESP_LOGI(TAG, "Connect to WiFi '%s' and open http://192.168.4.1", s_device_id);

    /* Block forever — device reboots after user saves config */
    while (1) {
        vTaskDelay(pdMS_TO_TICKS(10000));
    }
}
