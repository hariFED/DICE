-- DICE Coordinator — PostgreSQL schema
-- Run once at startup via sqlx::migrate!

CREATE TABLE IF NOT EXISTS nodes (
    node_id        BYTEA PRIMARY KEY,          -- 33-byte compressed secp256k1 pubkey
    registered_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen      TIMESTAMPTZ,
    latency_ms     INTEGER,
    uptime_secs    BIGINT,
    jobs_completed BIGINT DEFAULT 0,
    is_active      BOOLEAN DEFAULT TRUE
);

CREATE TABLE IF NOT EXISTS rounds (
    id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    request_id     BYTEA NOT NULL,
    status         TEXT NOT NULL,              -- 'collecting_commits' | 'collecting_reveals' | 'finalized' | 'failed'
    selected_nodes BYTEA[],
    randomness     BYTEA,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    finalized_at   TIMESTAMPTZ
);

CREATE TABLE IF NOT EXISTS commits (
    round_id     UUID REFERENCES rounds(id),
    node_id      BYTEA NOT NULL,
    commit_hash  BYTEA NOT NULL,
    submitted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (round_id, node_id)
);

CREATE TABLE IF NOT EXISTS reveals (
    round_id     UUID REFERENCES rounds(id),
    node_id      BYTEA NOT NULL,
    entropy      BYTEA NOT NULL,
    submitted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (round_id, node_id)
);

CREATE TABLE IF NOT EXISTS audit_log (
    id          BIGSERIAL PRIMARY KEY,
    event_type  TEXT NOT NULL,
    payload     JSONB,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
