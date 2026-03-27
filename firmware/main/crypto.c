#include "crypto.h"

#include <string.h>
#include <stdlib.h>

#include "esp_log.h"
#include "nvs_flash.h"
#include "nvs.h"
#include "esp_random.h"

/* mbedTLS */
#include "mbedtls/pk.h"
#include "mbedtls/ecdsa.h"
#include "mbedtls/ecp.h"
#include "mbedtls/sha256.h"
#include "mbedtls/entropy.h"
#include "mbedtls/ctr_drbg.h"
#include "mbedtls/error.h"
#include "mbedtls/bignum.h"

static const char *TAG = "dice_crypto";

/* NVS namespace/key for the device private key */
#define DICE_NVS_NAMESPACE  "dice"
#define DICE_NVS_KEY_PRIVKEY "priv_key_der"

/* Maximum DER private key size (EC keys are small, 200 bytes is generous) */
#define MAX_PRIVKEY_DER_LEN  512

/* ------------------------------------------------------------------ */
/* Module-level state (singleton)                                       */
/* ------------------------------------------------------------------ */

static bool               s_initialized = false;
static mbedtls_pk_context s_pk;
static mbedtls_entropy_context  s_entropy_ctx;
static mbedtls_ctr_drbg_context s_ctr_drbg;

/* ------------------------------------------------------------------ */
/* mbedTLS entropy callback — wraps esp_fill_random                    */
/* ------------------------------------------------------------------ */

static int mbedtls_esp_entropy_source(void *data,
                                       unsigned char *output,
                                       size_t len,
                                       size_t *olen)
{
    (void)data;
    esp_fill_random(output, len);
    *olen = len;
    return 0;
}

/* ------------------------------------------------------------------ */
/* Public API                                                           */
/* ------------------------------------------------------------------ */

bool dice_crypto_init(void)
{
    if (s_initialized) {
        ESP_LOGW(TAG, "dice_crypto_init called more than once");
        return true;
    }

    int ret;
    char mbedtls_errbuf[128];

    /* --- Load private key DER blob from NVS --- */
    nvs_handle_t nvs_handle;
    esp_err_t err = nvs_open(DICE_NVS_NAMESPACE, NVS_READONLY, &nvs_handle);
    if (err != ESP_OK) {
        ESP_LOGE(TAG, "nvs_open failed: %s", esp_err_to_name(err));
        return false;
    }

    size_t key_len = MAX_PRIVKEY_DER_LEN;
    uint8_t *key_buf = calloc(1, key_len);
    if (!key_buf) {
        ESP_LOGE(TAG, "Out of memory allocating key buffer");
        nvs_close(nvs_handle);
        return false;
    }

    err = nvs_get_blob(nvs_handle, DICE_NVS_KEY_PRIVKEY, key_buf, &key_len);
    nvs_close(nvs_handle);

    if (err != ESP_OK) {
        ESP_LOGE(TAG, "nvs_get_blob('%s') failed: %s",
                 DICE_NVS_KEY_PRIVKEY, esp_err_to_name(err));
        memset(key_buf, 0, key_len);
        free(key_buf);
        return false;
    }

    ESP_LOGI(TAG, "Loaded private key from NVS (%u bytes)", (unsigned)key_len);

    /* --- Initialise mbedTLS entropy + DRBG --- */
    mbedtls_entropy_init(&s_entropy_ctx);
    mbedtls_ctr_drbg_init(&s_ctr_drbg);

    ret = mbedtls_entropy_add_source(&s_entropy_ctx,
                                      mbedtls_esp_entropy_source,
                                      NULL,
                                      32,
                                      MBEDTLS_ENTROPY_SOURCE_STRONG);
    if (ret != 0) {
        ESP_LOGE(TAG, "entropy_add_source failed: -0x%04X", (unsigned)(-ret));
        goto fail;
    }

    ret = mbedtls_ctr_drbg_seed(&s_ctr_drbg,
                                  mbedtls_entropy_func,
                                  &s_entropy_ctx,
                                  (const unsigned char *)"dice_node",
                                  9);
    if (ret != 0) {
        mbedtls_strerror(ret, mbedtls_errbuf, sizeof(mbedtls_errbuf));
        ESP_LOGE(TAG, "ctr_drbg_seed failed: %s", mbedtls_errbuf);
        goto fail;
    }

    /* --- Parse private key --- */
    mbedtls_pk_init(&s_pk);

    /* mbedtls_pk_parse_key signature (ESP-IDF v5 / mbedTLS 3.x):
     * pk, key, keylen, pwd, pwdlen, f_rng, p_rng */
    ret = mbedtls_pk_parse_key(&s_pk,
                                key_buf, key_len,
                                NULL, 0,
                                mbedtls_ctr_drbg_random, &s_ctr_drbg);

    /* Scrub key material from heap regardless of outcome */
    memset(key_buf, 0, key_len);
    free(key_buf);
    key_buf = NULL;

    if (ret != 0) {
        mbedtls_strerror(ret, mbedtls_errbuf, sizeof(mbedtls_errbuf));
        ESP_LOGE(TAG, "pk_parse_key failed: %s", mbedtls_errbuf);
        goto fail_after_free;
    }

    /* Verify the key is ECDSA secp256k1 */
    if (mbedtls_pk_get_type(&s_pk) != MBEDTLS_PK_ECKEY) {
        ESP_LOGE(TAG, "Key is not an EC key (type=%d)", mbedtls_pk_get_type(&s_pk));
        goto fail_after_free;
    }

    mbedtls_ecp_keypair *kp = mbedtls_pk_ec(s_pk);
    if (kp->MBEDTLS_PRIVATE(grp).id != MBEDTLS_ECP_DP_SECP256K1) {
        ESP_LOGE(TAG, "Key curve is not secp256k1 (id=%d)",
                 kp->MBEDTLS_PRIVATE(grp).id);
        goto fail_after_free;
    }

    s_initialized = true;
    ESP_LOGI(TAG, "Crypto context initialised (secp256k1)");
    return true;

fail:
    if (key_buf) {
        memset(key_buf, 0, MAX_PRIVKEY_DER_LEN);
        free(key_buf);
    }
fail_after_free:
    mbedtls_pk_free(&s_pk);
    mbedtls_ctr_drbg_free(&s_ctr_drbg);
    mbedtls_entropy_free(&s_entropy_ctx);
    return false;
}

void dice_crypto_sha256(const uint8_t *data, size_t len, uint8_t hash_out[32])
{
    int ret = mbedtls_sha256(data, len, hash_out, 0 /* is224=0 */);
    if (ret != 0) {
        ESP_LOGE(TAG, "SHA-256 failed: -0x%04X", (unsigned)(-ret));
        memset(hash_out, 0, 32);
    }
}

bool dice_crypto_sign(const uint8_t *data, size_t data_len, uint8_t sig_out[64])
{
    if (!s_initialized) {
        ESP_LOGE(TAG, "dice_crypto_sign: crypto not initialised");
        return false;
    }

    int ret;
    char errbuf[128];

    /* Hash the data first */
    uint8_t hash[32];
    dice_crypto_sha256(data, data_len, hash);

    /* Sign using ECDSA */
    mbedtls_ecdsa_context ecdsa_ctx;
    mbedtls_ecdsa_init(&ecdsa_ctx);

    ret = mbedtls_ecdsa_from_keypair(&ecdsa_ctx, mbedtls_pk_ec(s_pk));
    if (ret != 0) {
        mbedtls_strerror(ret, errbuf, sizeof(errbuf));
        ESP_LOGE(TAG, "ecdsa_from_keypair failed: %s", errbuf);
        mbedtls_ecdsa_free(&ecdsa_ctx);
        return false;
    }

    /* Get DER signature first, then extract raw r||s */
    uint8_t der_sig[128];
    size_t der_sig_len = 0;

    ret = mbedtls_ecdsa_write_signature(&ecdsa_ctx,
                                         MBEDTLS_MD_SHA256,
                                         hash, sizeof(hash),
                                         der_sig, sizeof(der_sig), &der_sig_len,
                                         mbedtls_ctr_drbg_random, &s_ctr_drbg);

    mbedtls_ecdsa_free(&ecdsa_ctx);
    memset(hash, 0, sizeof(hash));

    if (ret != 0) {
        mbedtls_strerror(ret, errbuf, sizeof(errbuf));
        ESP_LOGE(TAG, "ecdsa_write_signature failed: %s", errbuf);
        return false;
    }

    /* Parse DER to extract raw r and s as 32-byte big-endian integers.
     *
     * DER ECDSA signature structure:
     *   0x30 [total_len]
     *   0x02 [r_len] [r_bytes...]
     *   0x02 [s_len] [s_bytes...]
     *
     * r and s may have a leading 0x00 padding byte if the high bit is set.
     * We strip the padding and zero-pad on the left to exactly 32 bytes.
     */
    memset(sig_out, 0, 64);

    const uint8_t *p = der_sig;
    const uint8_t *end = der_sig + der_sig_len;

    /* Outer SEQUENCE tag */
    if (p >= end || *p != 0x30) {
        ESP_LOGE(TAG, "DER parse: expected SEQUENCE tag");
        return false;
    }
    p++;
    /* Length field (short form only, DER ECDSA sig is always < 128 bytes) */
    if (p >= end) return false;
    p++;  /* skip total length byte */

    /* ---- r INTEGER ---- */
    if (p >= end || *p != 0x02) {
        ESP_LOGE(TAG, "DER parse: expected INTEGER tag for r");
        return false;
    }
    p++;
    if (p >= end) return false;
    size_t r_field_len = *p++;          /* length of DER r field (may include 0x00 pad) */
    if (p + r_field_len > end) return false;

    const uint8_t *r_start = p;
    size_t r_len = r_field_len;
    if (r_len > 0 && *r_start == 0x00) { r_start++; r_len--; }  /* strip sign pad */
    if (r_len > 32) {
        ESP_LOGE(TAG, "DER parse: r too large (%u bytes)", (unsigned)r_len);
        return false;
    }
    memcpy(sig_out + (32 - r_len), r_start, r_len);
    p += r_field_len;  /* advance past all r bytes (including any leading 0x00) */

    /* ---- s INTEGER ---- */
    if (p >= end || *p != 0x02) {
        ESP_LOGE(TAG, "DER parse: expected INTEGER tag for s");
        return false;
    }
    p++;
    if (p >= end) return false;
    size_t s_field_len = *p++;
    if (p + s_field_len > end) return false;

    const uint8_t *s_start = p;
    size_t s_len = s_field_len;
    if (s_len > 0 && *s_start == 0x00) { s_start++; s_len--; }
    if (s_len > 32) {
        ESP_LOGE(TAG, "DER parse: s too large (%u bytes)", (unsigned)s_len);
        return false;
    }
    memcpy(sig_out + 32 + (32 - s_len), s_start, s_len);

    return true;
}

bool dice_crypto_get_pubkey(uint8_t pubkey_out[33])
{
    if (!s_initialized) {
        ESP_LOGE(TAG, "dice_crypto_get_pubkey: crypto not initialised");
        return false;
    }

    mbedtls_ecp_keypair *kp = mbedtls_pk_ec(s_pk);
    if (!kp) {
        ESP_LOGE(TAG, "Failed to get EC keypair from pk context");
        return false;
    }

    size_t olen = 0;
    int ret = mbedtls_ecp_point_write_binary(
        &kp->MBEDTLS_PRIVATE(grp),
        &kp->MBEDTLS_PRIVATE(Q),
        MBEDTLS_ECP_PF_COMPRESSED,
        &olen,
        pubkey_out, 33);

    if (ret != 0 || olen != 33) {
        ESP_LOGE(TAG, "ecp_point_write_binary failed: -0x%04X (olen=%u)",
                 (unsigned)(-ret), (unsigned)olen);
        return false;
    }

    return true;
}
