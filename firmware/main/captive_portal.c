#include "captive_portal.h"
#include "base58.h"
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
#include "freertos/event_groups.h"

static const char *TAG = "dice_portal";

/* ------------------------------------------------------------------ */
/* WiFi test state — used by handle_save to test user-supplied creds    */
/* in APSTA mode without tearing down the AP.                           */
/* ------------------------------------------------------------------ */

#define WIFI_TEST_CONNECTED_BIT BIT0
#define WIFI_TEST_FAILED_BIT    BIT1
#define WIFI_TEST_TIMEOUT_MS    15000

static EventGroupHandle_t s_test_event_group = NULL;
static bool               s_testing          = false;
static int                s_test_retry_count = 0;

/* ------------------------------------------------------------------ */
/* DNS redirect task — redirects all DNS queries to 192.168.4.1         */
/* ------------------------------------------------------------------ */

#define DNS_PORT    53
#define DNS_MAX_LEN 512

static bool s_dns_running = false;

static void dns_redirect_task(void *arg)
{
    struct sockaddr_in server_addr = {
        .sin_family      = AF_INET,
        .sin_port        = htons(DNS_PORT),
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
        socklen_t          addr_len = sizeof(client_addr);
        int                len      = recvfrom(sock, rx_buf, sizeof(rx_buf), 0,
                                (struct sockaddr *)&client_addr, &addr_len);
        if (len < 12) continue; /* DNS header is 12 bytes minimum */

        memcpy(tx_buf, rx_buf, len);
        tx_buf[2] = 0x85; /* QR=1, AA=1, Opcode=0 */
        tx_buf[3] = 0x80; /* RA=1, RCODE=0 */
        tx_buf[6] = 0x00; /* ANCOUNT = 1 */
        tx_buf[7] = 0x01;

        int offset = len;
        /* Name pointer → query name at offset 12 */
        tx_buf[offset++] = 0xC0;
        tx_buf[offset++] = 0x0C;
        /* Type A (1), Class IN (1) */
        tx_buf[offset++] = 0x00;
        tx_buf[offset++] = 0x01;
        tx_buf[offset++] = 0x00;
        tx_buf[offset++] = 0x01;
        /* TTL = 60s */
        tx_buf[offset++] = 0x00;
        tx_buf[offset++] = 0x00;
        tx_buf[offset++] = 0x00;
        tx_buf[offset++] = 0x3C;
        /* RDLENGTH = 4, RDATA = 192.168.4.1 */
        tx_buf[offset++] = 0x00;
        tx_buf[offset++] = 0x04;
        tx_buf[offset++] = 192;
        tx_buf[offset++] = 168;
        tx_buf[offset++] = 4;
        tx_buf[offset++] = 1;

        sendto(sock, tx_buf, offset, 0, (struct sockaddr *)&client_addr, addr_len);
    }

    close(sock);
    vTaskDelete(NULL);
}

/* ------------------------------------------------------------------ */
/* Setup page HTML — DICE v7.7 dark-mono / blueprint-editorial design.  */
/*                                                                      */
/* Source: firmware/captive-portal-preview.html (open in a browser to   */
/* design + iterate, then port any tweaks back here).                   */
/*                                                                      */
/* The JS POSTs to /save and handles the JSON response:                 */
/*   - {"ok":true}                  -> "Provisioned. Rebooting..."      */
/*   - {"ok":false,"error":"..."}   -> bracketed [ ERR ] alert          */
/*                                                                      */
/* All special chars use HTML entities (&middot; &times; &hellip;       */
/* &ndash; &bull;) so this stays pure-ASCII C source.                   */
/* ------------------------------------------------------------------ */

static const char SETUP_HTML[] =
    "<!DOCTYPE html>"
    "<html lang='en'><head>"
    "<meta charset='utf-8'>"
    "<meta name='viewport' content='width=device-width,initial-scale=1,viewport-fit=cover'>"
    "<title>DICE Node &middot; Setup</title>"
    "<style>"
    ":root{--bg:#0a0a0a;--fg:#fafafa;--muted:#a1a1aa;--border:rgba(255,255,255,.10);"
    "--border-strong:rgba(255,255,255,.25);--ok:#4ade80;--warn:#facc15;--err:#f87171}"
    "*{box-sizing:border-box;margin:0;padding:0}"
    "html,body{background:var(--bg);color:var(--fg);"
    "font-family:ui-monospace,SFMono-Regular,Menlo,Consolas,monospace;"
    "font-size:14px;line-height:1.5;min-height:100vh;overflow-x:hidden;-webkit-text-size-adjust:100%}"
    "body{padding:28px 16px 48px;"
    "background-image:linear-gradient(to right,rgba(255,255,255,.035) 1px,transparent 1px),"
    "linear-gradient(to bottom,rgba(255,255,255,.035) 1px,transparent 1px);"
    "background-size:24px 24px}"
    ".wrap{max-width:480px;margin:0 auto;position:relative}"
    ".bar{display:flex;align-items:center;justify-content:space-between;font-size:10px;"
    "letter-spacing:.16em;color:var(--muted);text-transform:uppercase;margin-bottom:14px}"
    ".bar .live{display:inline-flex;align-items:center;gap:8px}"
    ".bar .live::before{content:'';width:6px;height:6px;background:var(--ok);border-radius:50%;"
    "box-shadow:0 0 8px var(--ok);animation:pulse 1.6s ease-in-out infinite}"
    "@keyframes pulse{0%,100%{opacity:1}50%{opacity:.35}}"
    "h1{font-family:ui-sans-serif,system-ui,-apple-system,sans-serif;font-size:30px;"
    "font-weight:500;letter-spacing:-0.015em;margin-bottom:6px}"
    ".tagline{color:var(--muted);font-size:12.5px;letter-spacing:.02em;margin-bottom:22px}"
    ".tagline strong{color:var(--fg);font-weight:500}"
    ".rule{display:flex;align-items:center;gap:12px;font-size:10px;color:var(--muted);"
    "margin:20px 0 14px;letter-spacing:.2em;text-transform:uppercase}"
    ".rule::before,.rule::after{content:'';flex:1;border-top:1px dashed var(--border)}"
    ".stats{display:grid;grid-template-columns:1fr 1fr;gap:1px;background:var(--border);"
    "border:1px solid var(--border)}"
    ".stat{background:var(--bg);padding:12px 14px}"
    ".stat-label{font-size:9.5px;letter-spacing:.18em;color:var(--muted);"
    "text-transform:uppercase;margin-bottom:4px}"
    ".stat-label::before{content:'[ '}.stat-label::after{content:' ]'}"
    ".stat-value{font-size:13px;color:var(--fg);font-feature-settings:'tnum' 1}"
    ".card{border:1px solid var(--border);padding:22px 18px 20px;position:relative;background:var(--bg)}"
    ".card::before,.card::after{content:'';position:absolute;width:10px;height:10px;"
    "border:1px solid var(--fg);opacity:.45;pointer-events:none}"
    ".card::before{top:-1px;left:-1px;border-right:none;border-bottom:none}"
    ".card::after{bottom:-1px;right:-1px;border-left:none;border-top:none}"
    ".field{margin-bottom:16px}"
    "label{display:block;font-size:9.5px;letter-spacing:.2em;color:var(--muted);"
    "text-transform:uppercase;margin-bottom:6px}"
    ".input-row{display:flex;align-items:stretch;border:1px solid var(--border);"
    "background:var(--bg);transition:border-color .12s,box-shadow .12s}"
    ".input-row:hover{border-color:var(--border-strong)}"
    ".input-row:focus-within{border-color:var(--fg);box-shadow:0 0 0 1px var(--fg)}"
    ".input-row::before{content:'>';color:var(--muted);padding:0 10px 0 12px;"
    "display:flex;align-items:center;font-weight:600}"
    ".input-row input{flex:1;width:100%;border:0;background:transparent;color:var(--fg);"
    "font:inherit;font-size:15px;padding:12px 12px 12px 0;outline:none;letter-spacing:.01em}"
    ".input-row input::placeholder{color:var(--muted);opacity:.55}"
    ".btn{display:block;width:100%;background:transparent;color:var(--fg);"
    "border:1px solid var(--fg);padding:13px 16px;font:inherit;font-size:12px;"
    "letter-spacing:.22em;text-transform:uppercase;cursor:pointer;margin-top:6px;"
    "transition:background .12s,color .12s}"
    ".btn::before{content:'[  '}.btn::after{content:'  ]'}"
    ".btn:hover:not(:disabled){background:var(--fg);color:var(--bg)}"
    ".btn:disabled{opacity:.35;cursor:not-allowed}"
    ".result{margin-top:14px}"
    ".alert{padding:10px 12px;border:1px solid;font-size:12px;letter-spacing:.02em;line-height:1.45}"
    ".alert::before{font-weight:600;margin-right:8px;letter-spacing:.18em}"
    ".alert.err{color:var(--err);border-color:var(--err)}"
    ".alert.err::before{content:'[ ERR ]'}"
    ".alert.ok{color:var(--ok);border-color:var(--ok)}"
    ".alert.ok::before{content:'[ OK ]'}"
    ".alert.run{color:var(--warn);border-color:var(--warn)}"
    ".alert.run::before{content:'[ ... ]'}"
    ".foot{text-align:center;margin-top:22px;font-size:9.5px;color:var(--muted);"
    "letter-spacing:.22em;text-transform:uppercase;line-height:1.8}"
    ".foot .sep{opacity:.4;margin:0 6px}"
    "@media (max-width:380px){body{padding:18px 12px 32px}h1{font-size:24px}}"
    "</style></head><body>"
    "<div class='wrap'>"
    "<div class='bar'><span class='live'>DEVICE LIVE</span>"
    "<span>NODE &middot; INITIAL SETUP</span></div>"
    "<h1>DICE Node</h1>"
    "<p class='tagline'>Hardware-backed randomness. Provision your <strong>node</strong> to join the network.</p>"
    "<div class='stats'>"
    "<div class='stat'><div class='stat-label'>Device</div>"
    "<div class='stat-value' id='devid'>DICE-&bull;&bull;&bull;&bull;</div></div>"
    "<div class='stat'><div class='stat-label'>Board</div>"
    "<div class='stat-value'>ESP32-S3-N16R8</div></div>"
    "<div class='stat'><div class='stat-label'>Firmware</div>"
    "<div class='stat-value'>v1.0.0</div></div>"
    "<div class='stat'><div class='stat-label'>Network</div>"
    "<div class='stat-value'>Solana &middot; Devnet</div></div>"
    "</div>"
    "<div class='rule'>&times; &nbsp;CONFIG&nbsp; &times;</div>"
    "<div class='card'>"
    "<form id='form' autocomplete='off' novalidate>"
    "<div class='field'><label for='ssid'>WiFi Network</label>"
    "<div class='input-row'><input id='ssid' type='text' placeholder='ssid' maxlength='32' required></div></div>"
    "<div class='field'><label for='pass'>WiFi Password</label>"
    "<div class='input-row'><input id='pass' type='password' placeholder='&bull;&bull;&bull;&bull;&bull;&bull;&bull;&bull;' maxlength='64' required></div></div>"
    "<div class='field'><label for='wallet'>Solana Payout Wallet</label>"
    "<div class='input-row'><input id='wallet' type='text' placeholder='32&ndash;44 base58 chars' maxlength='44' spellcheck='false' autocapitalize='off' required></div></div>"
    "<button class='btn' id='btn' type='submit'>Provision &amp; Connect</button>"
    "<div class='result' id='result' aria-live='polite'></div>"
    "</form></div>"
    "<div class='foot'>DICE PROTOCOL <span class='sep'>&middot;</span> v7.7<br>"
    "HARDWARE-BACKED VRF <span class='sep'>&middot;</span> ON-CHAIN AUDIT</div>"
    "</div>"
    "<script>"
    "(function(){"
    "var btn=document.getElementById('btn');"
    "var result=document.getElementById('result');"
    "var form=document.getElementById('form');"
    "function show(cls,msg){result.innerHTML='<div class=\"alert '+cls+'\">'+msg+'</div>';}"
    "fetch('/info').then(function(r){return r.json();}).then(function(d){"
    "if(d&&d.device_id)document.getElementById('devid').textContent=d.device_id;"
    "}).catch(function(){});"
    "form.addEventListener('submit',function(e){"
    "e.preventDefault();"
    "var s=document.getElementById('ssid').value.trim();"
    "var p=document.getElementById('pass').value;"
    "var w=document.getElementById('wallet').value.trim();"
    "if(!s)return show('err','WiFi network required');"
    "if(!p)return show('err','WiFi password required');"
    "if(!w)return show('err','Solana wallet required');"
    "if(w.length<32||w.length>44)return show('err','Wallet must be 32&ndash;44 base58 chars');"
    "btn.disabled=true;"
    "show('run','Testing connection to '+s+'. Up to 15 sec&hellip;');"
    "fetch('/save',{method:'POST',headers:{'Content-Type':'application/json'},"
    "body:JSON.stringify({ssid:s,pass:p,wallet:w})})"
    ".then(function(r){return r.json();}).then(function(d){"
    "if(d.ok)show('ok','Provisioned. Rebooting in 3 sec&hellip;');"
    "else{show('err',d.error||'Unknown error');btn.disabled=false;}"
    "}).catch(function(){"
    "show('err','Network dropped. Reconnect to DICE WiFi and retry.');"
    "btn.disabled=false;"
    "});"
    "});"
    "})();"
    "</script>"
    "</body></html>";

/* ------------------------------------------------------------------ */
/* Friendly device names — Greek mythology + cosmic + Greek alphabet.   */
/*                                                                      */
/* Public-domain, globally recognizable, zero IP risk. Each device      */
/* deterministically maps to one of these via FNV-1a(MAC) % 64. The     */
/* trailing 2-hex MAC byte disambiguates collisions (e.g. two boards    */
/* hashing to APOLLO would still differ as DICE-APOLLO-A4 vs            */
/* DICE-APOLLO-7B). Set once at boot in wifi_init_apsta().              */
/* ------------------------------------------------------------------ */

static const char * const DICE_NAMES[] = {
    "APOLLO",   "ATHENA",    "ATLAS",     "HERMES",    "HERA",      "ZEUS",      "ARES",      "POSEIDON",
    "HADES",    "ARTEMIS",   "APHRODITE", "DEMETER",   "DIONYSUS",  "HESTIA",    "IRIS",      "NEMESIS",
    "MORPHEUS", "ORPHEUS",   "EROS",      "PERSEUS",   "THESEUS",   "ICARUS",    "JASON",     "ATALANTA",
    "CASSANDRA","ANDROMEDA", "ARIADNE",   "ECHO",      "PSYCHE",    "NIKE",      "GAIA",      "HELIOS",
    "SELENE",   "EOS",       "BOREAS",    "ZEPHYR",    "NOTUS",     "ORION",     "LYRA",      "VEGA",
    "DRACO",    "PEGASUS",   "PHOENIX",   "AQUILA",    "CYGNUS",    "CASSIOPEIA","NOVA",      "NEBULA",
    "ALPHA",    "BETA",      "GAMMA",     "DELTA",     "EPSILON",   "ZETA",      "THETA",     "KAPPA",
    "LAMBDA",   "SIGMA",     "OMEGA",     "OMICRON",   "TITAN",     "OBERON",    "TRITON",    "RHEA",
};
#define DICE_NAME_COUNT (sizeof(DICE_NAMES) / sizeof(DICE_NAMES[0]))

/* ------------------------------------------------------------------ */
/* Shared state                                                         */
/* ------------------------------------------------------------------ */

/* Buffer sized for "DICE-CASSIOPEIA-A4" (longest possible) + slack. */
static char s_device_id[24] = {0};

/* ------------------------------------------------------------------ */
/* HTTP handlers                                                        */
/* ------------------------------------------------------------------ */

static esp_err_t handle_root(httpd_req_t *req)
{
    httpd_resp_set_type(req, "text/html");
    httpd_resp_send(req, SETUP_HTML, HTTPD_RESP_USE_STRLEN);
    return ESP_OK;
}

/* Redirect any unknown path to root (for captive portal auto-open). */
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

/* Helper: respond with {"ok":false,"error":"..."} and return ESP_OK.
 * The HTTP status stays 200 because the JS layer drives the UX; we
 * don't want the browser's error page getting in the way. */
static esp_err_t respond_error(httpd_req_t *req, const char *error_msg)
{
    char buf[256];
    /* Escape double quotes conservatively — for our use the error strings
     * are hand-written literals so quoting isn't an issue. */
    snprintf(buf, sizeof(buf), "{\"ok\":false,\"error\":\"%s\"}", error_msg);
    httpd_resp_set_type(req, "application/json");
    httpd_resp_send(req, buf, HTTPD_RESP_USE_STRLEN);
    return ESP_OK;
}

/* Attempt to connect STA with the given SSID/password. Returns true on
 * success, false on failure or timeout. Leaves the AP side untouched. */
static bool test_wifi_credentials(const char *ssid, const char *pass)
{
    /* Reset the event group — discard any stale bits from a previous test. */
    xEventGroupClearBits(s_test_event_group, WIFI_TEST_CONNECTED_BIT | WIFI_TEST_FAILED_BIT);

    wifi_config_t sta_config = {0};
    strncpy((char *)sta_config.sta.ssid,     ssid, sizeof(sta_config.sta.ssid) - 1);
    strncpy((char *)sta_config.sta.password, pass, sizeof(sta_config.sta.password) - 1);
    sta_config.sta.threshold.authmode = WIFI_AUTH_OPEN; /* accept any auth mode */
    sta_config.sta.pmf_cfg.capable    = true;
    sta_config.sta.pmf_cfg.required   = false;

    /* Ensure we're in APSTA mode (AP stays up so the user's browser doesn't drop). */
    ESP_ERROR_CHECK(esp_wifi_set_mode(WIFI_MODE_APSTA));
    ESP_ERROR_CHECK(esp_wifi_set_config(WIFI_IF_STA, &sta_config));

    /* Reset retry counter and enter testing mode so the event handler knows
     * to count disconnects and signal failure after enough retries. */
    s_test_retry_count = 0;
    s_testing          = true;

    esp_err_t err = esp_wifi_connect();
    if (err != ESP_OK) {
        ESP_LOGW(TAG, "esp_wifi_connect() returned %s", esp_err_to_name(err));
        s_testing = false;
        return false;
    }

    ESP_LOGI(TAG, "Testing WiFi '%s' (up to %d ms)...", ssid, WIFI_TEST_TIMEOUT_MS);
    EventBits_t bits = xEventGroupWaitBits(
        s_test_event_group,
        WIFI_TEST_CONNECTED_BIT | WIFI_TEST_FAILED_BIT,
        pdFALSE,
        pdFALSE,
        pdMS_TO_TICKS(WIFI_TEST_TIMEOUT_MS)
    );

    s_testing = false;
    bool success = (bits & WIFI_TEST_CONNECTED_BIT) != 0;

    if (!success) {
        /* Make sure STA is disconnected before we return, so the next test
         * attempt starts from a clean slate. */
        esp_wifi_disconnect();
        ESP_LOGW(TAG, "WiFi test FAILED for SSID '%s'", ssid);
    } else {
        ESP_LOGI(TAG, "WiFi test SUCCESS for SSID '%s'", ssid);
    }
    return success;
}

static esp_err_t handle_save(httpd_req_t *req)
{
    char buf[384] = {0};
    int  ret      = httpd_req_recv(req, buf, sizeof(buf) - 1);
    if (ret <= 0) {
        return respond_error(req, "No request body");
    }

    /* Minimal JSON field extraction — three known keys, short fixed sizes. */
    char ssid[33]   = {0};
    char pass[65]   = {0};
    char wallet[45] = {0};

    const char *p;
    char       *end;

    p = strstr(buf, "\"ssid\":\"");
    if (p) {
        p += 8;
        end = strchr(p, '"');
        if (end && (end - p) < (int)sizeof(ssid)) {
            memcpy(ssid, p, end - p);
        }
    }
    p = strstr(buf, "\"pass\":\"");
    if (p) {
        p += 8;
        end = strchr(p, '"');
        if (end && (end - p) < (int)sizeof(pass)) {
            memcpy(pass, p, end - p);
        }
    }
    p = strstr(buf, "\"wallet\":\"");
    if (p) {
        p += 10;
        end = strchr(p, '"');
        if (end && (end - p) < (int)sizeof(wallet)) {
            memcpy(wallet, p, end - p);
        }
    }

    /* ─── Input validation (cheap checks first) ─────────────────────── */

    if (strlen(ssid) == 0) {
        return respond_error(req, "WiFi network name is required");
    }
    if (strlen(pass) == 0) {
        return respond_error(req, "WiFi password is required");
    }
    if (strlen(wallet) == 0) {
        return respond_error(req, "Solana wallet address is required");
    }

    /* Validate Solana wallet before any WiFi work — cheap and catches typos. */
    if (!dice_is_valid_solana_wallet(wallet)) {
        return respond_error(
            req,
            "Solana wallet address is not valid. Expect 32-44 base58 characters "
            "that decode to exactly 32 bytes."
        );
    }

    /* ─── Test WiFi credentials before saving anything ──────────────── */

    if (!test_wifi_credentials(ssid, pass)) {
        /* Scrub password from stack before returning. */
        memset(pass, 0, sizeof(pass));
        return respond_error(
            req,
            "Could not connect to that WiFi network. Check the name and "
            "password and try again. Your inputs have not been saved."
        );
    }

    /* ─── Commit to NVS ──────────────────────────────────────────────── */

    nvs_handle_t nvs;
    esp_err_t    err = nvs_open("dice", NVS_READWRITE, &nvs);
    if (err != ESP_OK) {
        memset(pass, 0, sizeof(pass));
        ESP_LOGE(TAG, "nvs_open failed: %s", esp_err_to_name(err));
        return respond_error(req, "Could not open device storage. Contact support.");
    }

    esp_err_t e1 = nvs_set_str(nvs, "wifi_ssid",  ssid);
    esp_err_t e2 = nvs_set_str(nvs, "wifi_pass",  pass);
    esp_err_t e3 = nvs_set_str(nvs, "sol_wallet", wallet);
    esp_err_t e4 = nvs_commit(nvs);
    nvs_close(nvs);

    /* Scrub password from stack and buffer as soon as NVS write is done. */
    memset(pass, 0, sizeof(pass));
    memset(buf,  0, sizeof(buf));

    if (e1 != ESP_OK || e2 != ESP_OK || e3 != ESP_OK || e4 != ESP_OK) {
        ESP_LOGE(TAG, "NVS save failed: %d %d %d %d", e1, e2, e3, e4);
        return respond_error(req, "Could not save to device storage. Try again.");
    }

    ESP_LOGI(TAG, "Configuration saved — rebooting in 3 seconds");

    /* Respond OK, then reboot. */
    httpd_resp_set_type(req, "application/json");
    httpd_resp_send(req, "{\"ok\":true}", HTTPD_RESP_USE_STRLEN);

    vTaskDelay(pdMS_TO_TICKS(3000));
    esp_restart();

    return ESP_OK;
}

/* ------------------------------------------------------------------ */
/* Portal-local WiFi event handler                                      */
/*                                                                      */
/* Only active while the captive portal is running. Responsible for     */
/* driving the STA side during credential testing. AP side is handled   */
/* automatically by esp_wifi.                                           */
/* ------------------------------------------------------------------ */

#define WIFI_TEST_MAX_RETRIES 3

static void portal_wifi_event_handler(void *arg,
                                      esp_event_base_t event_base,
                                      int32_t event_id,
                                      void *event_data)
{
    if (event_base == WIFI_EVENT && event_id == WIFI_EVENT_STA_START) {
        /* Don't auto-connect — handle_save controls when STA attempts connection. */
        return;
    }

    if (event_base == WIFI_EVENT && event_id == WIFI_EVENT_STA_DISCONNECTED) {
        if (!s_testing) return;  /* Spurious disconnect while idle — ignore. */

        if (s_test_retry_count < WIFI_TEST_MAX_RETRIES) {
            s_test_retry_count++;
            ESP_LOGW(TAG, "STA disconnected during test, retry %d/%d",
                     s_test_retry_count, WIFI_TEST_MAX_RETRIES);
            esp_wifi_connect();
        } else {
            xEventGroupSetBits(s_test_event_group, WIFI_TEST_FAILED_BIT);
        }
        return;
    }

    if (event_base == IP_EVENT && event_id == IP_EVENT_STA_GOT_IP) {
        ip_event_got_ip_t *ev = (ip_event_got_ip_t *)event_data;
        ESP_LOGI(TAG, "STA got IP during test: " IPSTR, IP2STR(&ev->ip_info.ip));
        if (s_testing) {
            xEventGroupSetBits(s_test_event_group, WIFI_TEST_CONNECTED_BIT);
        }
        return;
    }
}

/* ------------------------------------------------------------------ */
/* WiFi APSTA init                                                      */
/* ------------------------------------------------------------------ */

static void wifi_init_apsta(void)
{
    /* Derive device name from MAC for AP SSID + portal display.
     *
     * Format: DICE-{NAME}-{XX} where:
     *   - NAME comes from DICE_NAMES[FNV-1a(MAC) % count]
     *   - XX is the last MAC byte in hex, disambiguates rare name collisions
     *
     * FNV-1a 32-bit chosen for tiny code (~12 bytes) and good MAC distribution.
     */
    uint8_t mac[6];
    esp_read_mac(mac, ESP_MAC_WIFI_STA);

    uint32_t h = 2166136261u; /* FNV-1a offset basis */
    for (int i = 0; i < 6; i++) {
        h ^= mac[i];
        h *= 16777619u;       /* FNV-1a prime */
    }
    const char *name = DICE_NAMES[h % DICE_NAME_COUNT];
    snprintf(s_device_id, sizeof(s_device_id), "DICE-%s-%02X", name, mac[5]);

    ESP_ERROR_CHECK(esp_netif_init());
    ESP_ERROR_CHECK(esp_event_loop_create_default());
    esp_netif_create_default_wifi_ap();
    esp_netif_create_default_wifi_sta();

    wifi_init_config_t cfg = WIFI_INIT_CONFIG_DEFAULT();
    ESP_ERROR_CHECK(esp_wifi_init(&cfg));

    /* Register portal-local WiFi/IP event handlers. These are only
     * active while the captive portal is running — they're removed
     * implicitly when the device reboots into normal mode. */
    ESP_ERROR_CHECK(esp_event_handler_instance_register(
        WIFI_EVENT, ESP_EVENT_ANY_ID, &portal_wifi_event_handler, NULL, NULL));
    ESP_ERROR_CHECK(esp_event_handler_instance_register(
        IP_EVENT, IP_EVENT_STA_GOT_IP, &portal_wifi_event_handler, NULL, NULL));

    /* AP configuration: DICE-XXXX, open network for easy setup. */
    wifi_config_t ap_config = {
        .ap = {
            .max_connection = 4,
            .authmode       = WIFI_AUTH_OPEN,
            .channel        = 1,
        },
    };
    strncpy((char *)ap_config.ap.ssid, s_device_id, sizeof(ap_config.ap.ssid) - 1);
    ap_config.ap.ssid_len = strlen(s_device_id);

    /* STA starts unconfigured — handle_save will fill this in when
     * the user submits their credentials. */
    wifi_config_t sta_config = {0};

    ESP_ERROR_CHECK(esp_wifi_set_mode(WIFI_MODE_APSTA));
    ESP_ERROR_CHECK(esp_wifi_set_config(WIFI_IF_AP,  &ap_config));
    ESP_ERROR_CHECK(esp_wifi_set_config(WIFI_IF_STA, &sta_config));
    ESP_ERROR_CHECK(esp_wifi_start());

    ESP_LOGI(TAG, "WiFi APSTA started — AP: %s (open), STA: idle", s_device_id);
    ESP_LOGI(TAG, "Setup page: http://192.168.4.1/");
}

/* ------------------------------------------------------------------ */
/* Public API                                                           */
/* ------------------------------------------------------------------ */

bool dice_is_provisioned(void)
{
    nvs_handle_t nvs;
    esp_err_t    err = nvs_open("dice", NVS_READONLY, &nvs);
    if (err != ESP_OK) return false;

    char   ssid[33] = {0};
    size_t len      = sizeof(ssid);
    err             = nvs_get_str(nvs, "wifi_ssid", ssid, &len);
    nvs_close(nvs);

    return (err == ESP_OK && strlen(ssid) > 0);
}

void dice_captive_portal_start(void)
{
    ESP_LOGI(TAG, "Starting captive portal — device not provisioned");

    dice_led_set(LED_STATUS_SETUP);

    /* Create event group for WiFi credential testing. */
    s_test_event_group = xEventGroupCreate();
    if (!s_test_event_group) {
        ESP_LOGE(TAG, "Failed to create event group");
        abort();
    }

    wifi_init_apsta();

    /* DNS redirect so "captive portal detection" URLs bounce to our page. */
    xTaskCreate(dns_redirect_task, "dns_redir", 4096, NULL, 5, NULL);

    /* HTTP server. */
    httpd_config_t config = HTTPD_DEFAULT_CONFIG();
    config.max_uri_handlers = 8;
    config.uri_match_fn     = httpd_uri_match_wildcard;

    httpd_handle_t server = NULL;
    ESP_ERROR_CHECK(httpd_start(&server, &config));

    httpd_uri_t uri_root     = {.uri = "/",      .method = HTTP_GET,  .handler = handle_root};
    httpd_uri_t uri_info     = {.uri = "/info",  .method = HTTP_GET,  .handler = handle_info};
    httpd_uri_t uri_save     = {.uri = "/save",  .method = HTTP_POST, .handler = handle_save};
    httpd_uri_t uri_catchall = {.uri = "/*",     .method = HTTP_GET,  .handler = handle_redirect};

    httpd_register_uri_handler(server, &uri_root);
    httpd_register_uri_handler(server, &uri_info);
    httpd_register_uri_handler(server, &uri_save);
    httpd_register_uri_handler(server, &uri_catchall);

    ESP_LOGI(TAG, "HTTP server started on port 80");
    ESP_LOGI(TAG, "Connect to WiFi '%s' and open http://192.168.4.1", s_device_id);

    /* Idle forever — handle_save will reboot the device when the user
     * completes setup successfully. No timeout; the user might take
     * minutes to find their WiFi password. */
    while (1) {
        vTaskDelay(pdMS_TO_TICKS(60000));
    }
}
