#include "dice_protocol.h"

#include <string.h>
#include <stdint.h>
#include <stdbool.h>

/* We use esp_log only if available (component is also used in host tests). */
#ifdef ESP_PLATFORM
#include "esp_log.h"
static const char *TAG = "dice_proto";
#define LOG_E(fmt, ...) ESP_LOGE(TAG, fmt, ##__VA_ARGS__)
#define LOG_W(fmt, ...) ESP_LOGW(TAG, fmt, ##__VA_ARGS__)
#else
#include <stdio.h>
#define LOG_E(fmt, ...) fprintf(stderr, "[ERROR] " fmt "\n", ##__VA_ARGS__)
#define LOG_W(fmt, ...) fprintf(stderr, "[WARN]  " fmt "\n", ##__VA_ARGS__)
#endif

/* ------------------------------------------------------------------ */
/* CBOR constants                                                        */
/* ------------------------------------------------------------------ */

/* Major types (upper 3 bits) */
#define CBOR_UINT   (0u << 5)
#define CBOR_BYTES  (2u << 5)
#define CBOR_TEXT   (3u << 5)
#define CBOR_MAP    (5u << 5)

/* Additional info values */
#define CBOR_AI_1BYTE  24u
#define CBOR_AI_2BYTE  25u
#define CBOR_AI_4BYTE  26u
#define CBOR_AI_8BYTE  27u

/* ------------------------------------------------------------------ */
/* Encoder helpers                                                       */
/* ------------------------------------------------------------------ */

typedef struct {
    uint8_t *buf;
    size_t   cap;
    size_t   pos;
    bool     overflow;
} cbor_enc_t;

static void enc_init(cbor_enc_t *e, uint8_t *buf, size_t cap)
{
    e->buf      = buf;
    e->cap      = cap;
    e->pos      = 0;
    e->overflow = false;
}

static void enc_byte(cbor_enc_t *e, uint8_t b)
{
    if (e->pos < e->cap) {
        e->buf[e->pos] = b;
    } else {
        e->overflow = true;
    }
    e->pos++;
}

static void enc_head(cbor_enc_t *e, uint8_t major, uint64_t val)
{
    if (val <= 23) {
        enc_byte(e, (uint8_t)(major | val));
    } else if (val <= 0xFF) {
        enc_byte(e, major | CBOR_AI_1BYTE);
        enc_byte(e, (uint8_t)val);
    } else if (val <= 0xFFFF) {
        enc_byte(e, major | CBOR_AI_2BYTE);
        enc_byte(e, (uint8_t)(val >> 8));
        enc_byte(e, (uint8_t)(val));
    } else if (val <= 0xFFFFFFFFu) {
        enc_byte(e, major | CBOR_AI_4BYTE);
        enc_byte(e, (uint8_t)(val >> 24));
        enc_byte(e, (uint8_t)(val >> 16));
        enc_byte(e, (uint8_t)(val >>  8));
        enc_byte(e, (uint8_t)(val));
    } else {
        enc_byte(e, major | CBOR_AI_8BYTE);
        enc_byte(e, (uint8_t)(val >> 56));
        enc_byte(e, (uint8_t)(val >> 48));
        enc_byte(e, (uint8_t)(val >> 40));
        enc_byte(e, (uint8_t)(val >> 32));
        enc_byte(e, (uint8_t)(val >> 24));
        enc_byte(e, (uint8_t)(val >> 16));
        enc_byte(e, (uint8_t)(val >>  8));
        enc_byte(e, (uint8_t)(val));
    }
}

/* Write an unsigned integer item */
static void write_uint(cbor_enc_t *e, uint64_t val)
{
    enc_head(e, CBOR_UINT, val);
}

/* Write a byte string item */
static void write_bytes(cbor_enc_t *e, const uint8_t *data, size_t len)
{
    enc_head(e, CBOR_BYTES, (uint64_t)len);
    for (size_t i = 0; i < len; i++) {
        enc_byte(e, data[i]);
    }
}

/* Write a text string item */
static void write_text(cbor_enc_t *e, const char *str)
{
    size_t len = strlen(str);
    enc_head(e, CBOR_TEXT, (uint64_t)len);
    for (size_t i = 0; i < len; i++) {
        enc_byte(e, (uint8_t)str[i]);
    }
}

/* Write a map header with n pairs */
static void write_map(cbor_enc_t *e, uint64_t n_pairs)
{
    enc_head(e, CBOR_MAP, n_pairs);
}

/* Convenience: write a map key (small unsigned int) */
static void write_key(cbor_enc_t *e, uint8_t key)
{
    write_uint(e, key);
}

/* ------------------------------------------------------------------ */
/* Public encoder                                                        */
/* ------------------------------------------------------------------ */

int dice_msg_encode(const dice_message_t *msg, uint8_t *buf, size_t buf_len)
{
    cbor_enc_t e;
    enc_init(&e, buf, buf_len);

    switch (msg->type) {
    /* -----------------------------------------------------------------
     * Heartbeat: { 0:type, 1:node_id, 2:latency_ms, 3:uptime_secs,
     *              4:jobs_completed, 5:timestamp }
     * ----------------------------------------------------------------- */
    case DICE_MSG_HEARTBEAT:
        write_map(&e, 6);
        write_key(&e, 0); write_uint(&e, DICE_MSG_HEARTBEAT);
        write_key(&e, 1); write_bytes(&e, msg->heartbeat.node_id, 33);
        write_key(&e, 2); write_uint(&e, msg->heartbeat.latency_ms);
        write_key(&e, 3); write_uint(&e, msg->heartbeat.uptime_secs);
        write_key(&e, 4); write_uint(&e, msg->heartbeat.jobs_completed);
        write_key(&e, 5); write_uint(&e, msg->heartbeat.timestamp);
        break;

    /* -----------------------------------------------------------------
     * CommitSubmission: { 0:type, 1:request_id, 2:node_id,
     *                     3:commit_hash, 4:signature }
     * ----------------------------------------------------------------- */
    case DICE_MSG_COMMIT:
        write_map(&e, 5);
        write_key(&e, 0); write_uint(&e, DICE_MSG_COMMIT);
        write_key(&e, 1); write_bytes(&e, msg->commit.request_id, 32);
        write_key(&e, 2); write_bytes(&e, msg->commit.node_id, 33);
        write_key(&e, 3); write_bytes(&e, msg->commit.commit_hash, 32);
        write_key(&e, 4); write_bytes(&e, msg->commit.signature, 64);
        break;

    /* -----------------------------------------------------------------
     * RevealSubmission: { 0:type, 1:request_id, 2:node_id,
     *                     3:entropy, 4:signature }
     * ----------------------------------------------------------------- */
    case DICE_MSG_REVEAL:
        write_map(&e, 5);
        write_key(&e, 0); write_uint(&e, DICE_MSG_REVEAL);
        write_key(&e, 1); write_bytes(&e, msg->reveal.request_id, 32);
        write_key(&e, 2); write_bytes(&e, msg->reveal.node_id, 33);
        write_key(&e, 3); write_bytes(&e, msg->reveal.entropy, 32);
        write_key(&e, 4); write_bytes(&e, msg->reveal.signature, 64);
        break;

    /* -----------------------------------------------------------------
     * JobAssignment: encoded by coordinator, decoded by node.
     * We still support encoding for testing.
     * { 0:type, 1:request_id, 2:round_seq, 3:deadline_ts }
     * ----------------------------------------------------------------- */
    case DICE_MSG_JOB_ASSIGN:
        write_map(&e, 4);
        write_key(&e, 0); write_uint(&e, DICE_MSG_JOB_ASSIGN);
        write_key(&e, 1); write_bytes(&e, msg->job_assign.request_id, 32);
        write_key(&e, 2); write_uint(&e, msg->job_assign.round_seq);
        write_key(&e, 3); write_uint(&e, msg->job_assign.deadline_ts);
        break;

    /* -----------------------------------------------------------------
     * RoundResult: { 0:type, 1:request_id, 2:status, 3:randomness }
     * ----------------------------------------------------------------- */
    case DICE_MSG_ROUND_RESULT:
        write_map(&e, 4);
        write_key(&e, 0); write_uint(&e, DICE_MSG_ROUND_RESULT);
        write_key(&e, 1); write_bytes(&e, msg->round_result.request_id, 32);
        write_key(&e, 2); write_text(&e, msg->round_result.status);
        write_key(&e, 3); write_bytes(&e, msg->round_result.randomness, 32);
        break;

    default:
        LOG_E("dice_msg_encode: unknown message type %u", msg->type);
        return -1;
    }

    if (e.overflow) {
        LOG_E("dice_msg_encode: buffer overflow (need %u, have %u)",
              (unsigned)e.pos, (unsigned)buf_len);
        return -1;
    }

    return (int)e.pos;
}

/* ------------------------------------------------------------------ */
/* Decoder helpers                                                       */
/* ------------------------------------------------------------------ */

typedef struct {
    const uint8_t *buf;
    size_t         len;
    size_t         pos;
    bool           error;
} cbor_dec_t;

static void dec_init(cbor_dec_t *d, const uint8_t *buf, size_t len)
{
    d->buf   = buf;
    d->len   = len;
    d->pos   = 0;
    d->error = false;
}

static uint8_t dec_byte(cbor_dec_t *d)
{
    if (d->pos >= d->len) {
        d->error = true;
        return 0;
    }
    return d->buf[d->pos++];
}

/* Read the additional-info value from an initial byte */
static uint64_t dec_addl(cbor_dec_t *d, uint8_t addl_info)
{
    if (addl_info < 24) {
        return addl_info;
    }
    switch (addl_info) {
    case CBOR_AI_1BYTE:
        return dec_byte(d);
    case CBOR_AI_2BYTE: {
        uint64_t v = (uint64_t)dec_byte(d) << 8;
        v |= dec_byte(d);
        return v;
    }
    case CBOR_AI_4BYTE: {
        uint64_t v = (uint64_t)dec_byte(d) << 24;
        v |= (uint64_t)dec_byte(d) << 16;
        v |= (uint64_t)dec_byte(d) <<  8;
        v |= (uint64_t)dec_byte(d);
        return v;
    }
    case CBOR_AI_8BYTE: {
        uint64_t v = (uint64_t)dec_byte(d) << 56;
        v |= (uint64_t)dec_byte(d) << 48;
        v |= (uint64_t)dec_byte(d) << 40;
        v |= (uint64_t)dec_byte(d) << 32;
        v |= (uint64_t)dec_byte(d) << 24;
        v |= (uint64_t)dec_byte(d) << 16;
        v |= (uint64_t)dec_byte(d) <<  8;
        v |= (uint64_t)dec_byte(d);
        return v;
    }
    default:
        d->error = true;
        return 0;
    }
}

/* Read a CBOR item header: returns major type and sets *val to argument */
static uint8_t dec_head(cbor_dec_t *d, uint64_t *val)
{
    if (d->error) return 0xFF;
    uint8_t b = dec_byte(d);
    if (d->error) return 0xFF;
    uint8_t major = (b >> 5) & 0x07;
    uint8_t addl  = b & 0x1F;
    *val = dec_addl(d, addl);
    return major;
}

/* Read a uint, returning 0 and setting error on type mismatch */
static uint64_t dec_uint(cbor_dec_t *d)
{
    uint64_t v = 0;
    uint8_t major = dec_head(d, &v);
    if (major != 0) {
        LOG_E("dec_uint: expected major 0, got %u", major);
        d->error = true;
        return 0;
    }
    return v;
}

/* Read a byte string into a fixed-size buffer; returns false on error */
static bool dec_bytes_fixed(cbor_dec_t *d, uint8_t *out, size_t expected_len)
{
    uint64_t v = 0;
    uint8_t major = dec_head(d, &v);
    if (major != 2) {
        LOG_E("dec_bytes_fixed: expected major 2, got %u", major);
        d->error = true;
        return false;
    }
    if (v != (uint64_t)expected_len) {
        LOG_E("dec_bytes_fixed: expected %u bytes, got %u",
              (unsigned)expected_len, (unsigned)v);
        d->error = true;
        return false;
    }
    for (size_t i = 0; i < expected_len; i++) {
        out[i] = dec_byte(d);
    }
    return !d->error;
}

/* Read a text string into a fixed-size buffer (null terminated) */
static bool dec_text_fixed(cbor_dec_t *d, char *out, size_t max_len)
{
    uint64_t v = 0;
    uint8_t major = dec_head(d, &v);
    if (major != 3) {
        LOG_E("dec_text_fixed: expected major 3, got %u", major);
        d->error = true;
        return false;
    }
    if (v >= max_len) {
        LOG_E("dec_text_fixed: string too long (%u >= %u)",
              (unsigned)v, (unsigned)max_len);
        d->error = true;
        return false;
    }
    for (uint64_t i = 0; i < v; i++) {
        out[i] = (char)dec_byte(d);
    }
    out[v] = '\0';
    return !d->error;
}

/* Skip one CBOR item (for unknown map keys) */
static void dec_skip(cbor_dec_t *d)
{
    if (d->error) return;
    uint64_t v = 0;
    uint8_t major = dec_head(d, &v);
    switch (major) {
    case 0: /* uint — nothing more to skip */
    case 1: /* neg int */
        break;
    case 2: /* bytes */
    case 3: /* text */
        if (d->pos + v > d->len) { d->error = true; return; }
        d->pos += (size_t)v;
        break;
    case 4: /* array — skip v items */
        for (uint64_t i = 0; i < v && !d->error; i++) dec_skip(d);
        break;
    case 5: /* map — skip v*2 items */
        for (uint64_t i = 0; i < v * 2 && !d->error; i++) dec_skip(d);
        break;
    default:
        d->error = true;
        break;
    }
}

/* ------------------------------------------------------------------ */
/* Public decoder                                                        */
/* ------------------------------------------------------------------ */

bool dice_msg_decode(const uint8_t *buf, size_t buf_len, dice_message_t *msg_out)
{
    memset(msg_out, 0, sizeof(*msg_out));

    cbor_dec_t d;
    dec_init(&d, buf, buf_len);

    /* Expect top-level map */
    uint64_t map_len = 0;
    uint8_t major = dec_head(&d, &map_len);
    if (major != 5) {
        LOG_E("decode: expected top-level map, got major %u", major);
        return false;
    }

    /* We need to find the type field (key 0) first to know how to decode.
     * Strategy: iterate over all map entries, decode type when we see key 0,
     * then continue decoding the remaining fields based on the known type.
     * To handle arbitrary key ordering, we do two passes:
     *   Pass 1: find type (key 0)
     *   Pass 2: decode all fields
     *
     * For simplicity, since coordinators we write will always put key 0
     * first, we do a single forward pass and handle key 0 first. */

    uint8_t msg_type = 0xFF;
    bool    type_seen = false;

    /* Reset and re-parse: store field data as we encounter it */

    /* We'll decode by scanning all map pairs. Use a simple state machine. */
    /* Re-init decoder at map body start */
    size_t map_body_start = d.pos;
    (void)map_body_start;

    /* Fields to collect, indexed by key */
#define MAX_MAP_KEYS 8
    struct {
        bool    seen;
        bool    is_uint;
        bool    is_bytes;
        bool    is_text;
        uint64_t uint_val;
        uint8_t  bytes_buf[128]; /* large enough for signature */
        size_t   bytes_len;
        char     text_buf[32];
    } fields[MAX_MAP_KEYS];
    memset(fields, 0, sizeof(fields));

    for (uint64_t pair = 0; pair < map_len; pair++) {
        if (d.error) break;

        /* Read key — must be uint */
        uint64_t key_val = 0;
        uint8_t key_major = dec_head(&d, &key_val);
        if (key_major != 0 || key_val >= MAX_MAP_KEYS) {
            /* Unknown/out-of-range key — skip the value */
            dec_skip(&d);
            continue;
        }
        uint8_t key = (uint8_t)key_val;

        /* Read value */
        if (d.error) break;
        size_t val_start = d.pos;
        uint64_t vv = 0;
        uint8_t val_major = dec_head(&d, &vv);

        fields[key].seen = true;

        switch (val_major) {
        case 0: /* uint */
            fields[key].is_uint    = true;
            fields[key].uint_val   = vv;
            break;
        case 2: /* bytes */
            if (vv > sizeof(fields[key].bytes_buf)) {
                LOG_E("decode: byte string key=%u too large (%u)", key, (unsigned)vv);
                d.error = true;
                break;
            }
            fields[key].is_bytes   = true;
            fields[key].bytes_len  = (size_t)vv;
            for (size_t i = 0; i < (size_t)vv; i++) {
                fields[key].bytes_buf[i] = dec_byte(&d);
            }
            break;
        case 3: /* text */
            if (vv >= sizeof(fields[key].text_buf)) {
                LOG_E("decode: text string key=%u too large (%u)", key, (unsigned)vv);
                d.error = true;
                break;
            }
            fields[key].is_text    = true;
            for (uint64_t i = 0; i < vv; i++) {
                fields[key].text_buf[i] = (char)dec_byte(&d);
            }
            fields[key].text_buf[vv] = '\0';
            break;
        default:
            /* Skip unsupported value types */
            d.pos = val_start;
            dec_skip(&d);
            break;
        }

        if (key == 0 && fields[0].is_uint) {
            msg_type    = (uint8_t)fields[0].uint_val;
            type_seen   = true;
        }
    }

    if (d.error) {
        LOG_E("decode: CBOR parse error");
        return false;
    }

    if (!type_seen) {
        LOG_E("decode: message missing type field (key 0)");
        return false;
    }

    msg_out->type = msg_type;

    /* Helper macro: check field exists as bytes with expected length */
#define REQ_BYTES(k, dst, dlen) \
    do { \
        if (!fields[k].seen || !fields[k].is_bytes || fields[k].bytes_len != (dlen)) { \
            LOG_E("decode: missing/bad bytes field key=%u", (k)); \
            return false; \
        } \
        memcpy((dst), fields[k].bytes_buf, (dlen)); \
    } while(0)

#define REQ_UINT(k, dst) \
    do { \
        if (!fields[k].seen || !fields[k].is_uint) { \
            LOG_E("decode: missing/bad uint field key=%u", (k)); \
            return false; \
        } \
        (dst) = fields[k].uint_val; \
    } while(0)

    switch (msg_type) {
    case DICE_MSG_JOB_ASSIGN:
        /* keys: 0=type,1=request_id,2=round_seq,3=deadline_ts */
        REQ_BYTES(1, msg_out->job_assign.request_id, 32);
        REQ_UINT(2, msg_out->job_assign.round_seq);
        REQ_UINT(3, msg_out->job_assign.deadline_ts);
        break;

    case DICE_MSG_ROUND_RESULT:
        /* keys: 0=type,1=request_id,2=status,3=randomness */
        REQ_BYTES(1, msg_out->round_result.request_id, 32);
        if (!fields[2].seen || !fields[2].is_text) {
            LOG_E("decode: missing text field key=2 (status)");
            return false;
        }
        memcpy(msg_out->round_result.status, fields[2].text_buf,
               sizeof(msg_out->round_result.status) - 1);
        msg_out->round_result.status[sizeof(msg_out->round_result.status) - 1] = '\0';
        REQ_BYTES(3, msg_out->round_result.randomness, 32);
        break;

    case DICE_MSG_HEARTBEAT:
        /* keys: 0=type,1=node_id,2=latency_ms,3=uptime_secs,4=jobs_completed,5=timestamp */
        REQ_BYTES(1, msg_out->heartbeat.node_id, 33);
        REQ_UINT(2, msg_out->heartbeat.latency_ms);
        REQ_UINT(3, msg_out->heartbeat.uptime_secs);
        REQ_UINT(4, msg_out->heartbeat.jobs_completed);
        REQ_UINT(5, msg_out->heartbeat.timestamp);
        break;

    case DICE_MSG_COMMIT:
        /* keys: 0=type,1=request_id,2=node_id,3=commit_hash,4=signature */
        REQ_BYTES(1, msg_out->commit.request_id, 32);
        REQ_BYTES(2, msg_out->commit.node_id, 33);
        REQ_BYTES(3, msg_out->commit.commit_hash, 32);
        REQ_BYTES(4, msg_out->commit.signature, 64);
        break;

    case DICE_MSG_REVEAL:
        /* keys: 0=type,1=request_id,2=node_id,3=entropy,4=signature */
        REQ_BYTES(1, msg_out->reveal.request_id, 32);
        REQ_BYTES(2, msg_out->reveal.node_id, 33);
        REQ_BYTES(3, msg_out->reveal.entropy, 32);
        REQ_BYTES(4, msg_out->reveal.signature, 64);
        break;

    default:
        LOG_E("decode: unknown message type %u", msg_type);
        return false;
    }

#undef REQ_BYTES
#undef REQ_UINT

    return true;
}
