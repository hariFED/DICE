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

-- -----------------------------------------------------------------------
-- Keeper automation
-- -----------------------------------------------------------------------

CREATE TABLE IF NOT EXISTS keeper_tasks (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name             TEXT NOT NULL,
    trigger_type     TEXT NOT NULL,          -- 'interval' | 'once'
    trigger_config   JSONB NOT NULL,         -- {"every_secs": 10}
    target_program   BYTEA NOT NULL,         -- 32-byte Pubkey
    instruction_data BYTEA NOT NULL,
    accounts_json    JSONB NOT NULL,         -- serialized Vec<AccountMeta>
    enabled          BOOLEAN DEFAULT TRUE,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    total_executions BIGINT DEFAULT 0,
    total_failures   BIGINT DEFAULT 0,
    last_executed_at TIMESTAMPTZ
);

CREATE TABLE IF NOT EXISTS keeper_executions (
    id            BIGSERIAL PRIMARY KEY,
    task_id       UUID REFERENCES keeper_tasks(id) ON DELETE CASCADE,
    tx_signature  TEXT,
    success       BOOLEAN NOT NULL,
    error_msg     TEXT,
    latency_ms    INTEGER NOT NULL,
    executed_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_keeper_exec_task ON keeper_executions(task_id);
CREATE INDEX IF NOT EXISTS idx_keeper_exec_time ON keeper_executions(executed_at DESC);

-- -----------------------------------------------------------------------
-- Notary timestamping
-- -----------------------------------------------------------------------

CREATE TABLE IF NOT EXISTS notary_attestations (
    id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    content_hash   BYTEA NOT NULL,
    hash_algorithm TEXT NOT NULL DEFAULT 'sha256',
    metadata       JSONB,
    witness_count  SMALLINT NOT NULL,
    threshold      SMALLINT NOT NULL,
    receipt_json   JSONB NOT NULL,          -- full self-contained receipt
    created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_notary_hash ON notary_attestations(content_hash);
CREATE INDEX IF NOT EXISTS idx_notary_time ON notary_attestations(created_at DESC);
