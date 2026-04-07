use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Row types (used when reading back rows from the DB)
// ---------------------------------------------------------------------------

/// A node record as stored in the database.
#[derive(Debug, sqlx::FromRow)]
pub struct NodeRow {
    pub node_id: Vec<u8>,
    pub latency_ms: Option<i32>,
    pub uptime_secs: Option<i64>,
    pub jobs_completed: Option<i64>,
    pub is_active: Option<bool>,
}

/// A round record as stored in the database.
#[derive(Debug, sqlx::FromRow)]
pub struct RoundRow {
    pub id: Uuid,
    pub request_id: Vec<u8>,
    pub status: String,
    pub randomness: Option<Vec<u8>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub finalized_at: Option<chrono::DateTime<chrono::Utc>>,
}

// ---------------------------------------------------------------------------
// Write helpers
// ---------------------------------------------------------------------------

/// Insert or update a node's heartbeat statistics.
pub async fn upsert_node(
    pool: &PgPool,
    node_id: &[u8],
    latency_ms: i32,
    uptime_secs: i64,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO nodes (node_id, last_seen, latency_ms, uptime_secs, is_active)
        VALUES ($1, NOW(), $2, $3, TRUE)
        ON CONFLICT (node_id) DO UPDATE
          SET last_seen   = NOW(),
              latency_ms  = EXCLUDED.latency_ms,
              uptime_secs = EXCLUDED.uptime_secs,
              is_active   = TRUE
        "#,
    )
    .bind(node_id)
    .bind(latency_ms)
    .bind(uptime_secs)
    .execute(pool)
    .await?;
    Ok(())
}

/// Create a new round row and return its generated UUID.
pub async fn create_round(
    pool: &PgPool,
    request_id: &[u8],
    selected_nodes: &[Vec<u8>],
) -> Result<Uuid> {
    let row: (Uuid,) = sqlx::query_as(
        r#"
        INSERT INTO rounds (request_id, status, selected_nodes)
        VALUES ($1, 'collecting_commits', $2)
        RETURNING id
        "#,
    )
    .bind(request_id)
    .bind(selected_nodes)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

/// Persist a commit submission.
pub async fn record_commit(
    pool: &PgPool,
    round_id: Uuid,
    node_id: &[u8],
    commit_hash: &[u8],
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO commits (round_id, node_id, commit_hash)
        VALUES ($1, $2, $3)
        ON CONFLICT (round_id, node_id) DO NOTHING
        "#,
    )
    .bind(round_id)
    .bind(node_id)
    .bind(commit_hash)
    .execute(pool)
    .await?;
    Ok(())
}

/// Persist a reveal submission.
pub async fn record_reveal(
    pool: &PgPool,
    round_id: Uuid,
    node_id: &[u8],
    entropy: &[u8],
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO reveals (round_id, node_id, entropy)
        VALUES ($1, $2, $3)
        ON CONFLICT (round_id, node_id) DO NOTHING
        "#,
    )
    .bind(round_id)
    .bind(node_id)
    .bind(entropy)
    .execute(pool)
    .await?;
    Ok(())
}

/// Mark a round as finalized with the combined randomness output.
pub async fn finalize_round(pool: &PgPool, round_id: Uuid, randomness: &[u8]) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE rounds
        SET status       = 'finalized',
            randomness   = $2,
            finalized_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(round_id)
    .bind(randomness)
    .execute(pool)
    .await?;
    Ok(())
}

/// Mark a round as failed with a human-readable reason.
pub async fn fail_round(pool: &PgPool, round_id: Uuid, reason: &str) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE rounds
        SET status       = 'failed',
            finalized_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(round_id)
    .execute(pool)
    .await?;

    // Record the failure reason in the audit log.
    sqlx::query(
        r#"
        INSERT INTO audit_log (event_type, payload)
        VALUES ('round_failed', jsonb_build_object('round_id', $1::text, 'reason', $2))
        "#,
    )
    .bind(round_id.to_string())
    .bind(reason)
    .execute(pool)
    .await?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Keeper helpers
// ---------------------------------------------------------------------------

/// Row type for keeper executions.
#[derive(Debug, sqlx::FromRow, serde::Serialize)]
pub struct KeeperExecutionRow {
    pub id: i64,
    pub task_id: Option<Uuid>,
    pub tx_signature: Option<String>,
    pub success: bool,
    pub error_msg: Option<String>,
    pub latency_ms: i32,
    pub executed_at: chrono::DateTime<chrono::Utc>,
}

/// Record a keeper task execution.
pub async fn record_keeper_execution(
    pool: &PgPool,
    task_id: Uuid,
    tx_signature: Option<&str>,
    success: bool,
    error_msg: Option<&str>,
    latency_ms: i32,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO keeper_executions (task_id, tx_signature, success, error_msg, latency_ms)
        VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(task_id)
    .bind(tx_signature)
    .bind(success)
    .bind(error_msg)
    .bind(latency_ms)
    .execute(pool)
    .await?;

    // Update aggregate stats on the task row.
    if success {
        sqlx::query(
            r#"
            UPDATE keeper_tasks
            SET total_executions = total_executions + 1,
                last_executed_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(task_id)
        .execute(pool)
        .await?;
    } else {
        sqlx::query(
            r#"
            UPDATE keeper_tasks
            SET total_executions = total_executions + 1,
                total_failures   = total_failures + 1,
                last_executed_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(task_id)
        .execute(pool)
        .await?;
    }

    Ok(())
}

/// Fetch recent keeper executions.
pub async fn get_keeper_history(pool: &PgPool, limit: i64) -> Result<Vec<KeeperExecutionRow>> {
    let rows: Vec<KeeperExecutionRow> = sqlx::query_as(
        r#"
        SELECT id, task_id, tx_signature, success, error_msg, latency_ms, executed_at
        FROM keeper_executions
        ORDER BY executed_at DESC
        LIMIT $1
        "#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

// ---------------------------------------------------------------------------
// Notary helpers
// ---------------------------------------------------------------------------

/// Row type for notary attestations.
#[derive(Debug, sqlx::FromRow, serde::Serialize)]
pub struct NotaryRow {
    pub id: Uuid,
    pub content_hash: Vec<u8>,
    pub hash_algorithm: String,
    pub metadata: Option<serde_json::Value>,
    pub witness_count: i16,
    pub threshold: i16,
    pub receipt_json: serde_json::Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Store a notary attestation.
pub async fn create_notary_attestation(
    pool: &PgPool,
    content_hash: &[u8],
    hash_algorithm: &str,
    metadata: Option<&serde_json::Value>,
    witness_count: i16,
    threshold: i16,
    receipt_json: &serde_json::Value,
) -> Result<Uuid> {
    let row: (Uuid,) = sqlx::query_as(
        r#"
        INSERT INTO notary_attestations (content_hash, hash_algorithm, metadata, witness_count, threshold, receipt_json)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id
        "#,
    )
    .bind(content_hash)
    .bind(hash_algorithm)
    .bind(metadata)
    .bind(witness_count)
    .bind(threshold)
    .bind(receipt_json)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

/// Fetch a notary attestation by UUID.
pub async fn get_notary_attestation(pool: &PgPool, id: Uuid) -> Result<Option<NotaryRow>> {
    let row: Option<NotaryRow> = sqlx::query_as(
        r#"
        SELECT id, content_hash, hash_algorithm, metadata, witness_count, threshold, receipt_json, created_at
        FROM notary_attestations
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Fetch recent notary attestations.
pub async fn get_notary_history(pool: &PgPool, limit: i64) -> Result<Vec<NotaryRow>> {
    let rows: Vec<NotaryRow> = sqlx::query_as(
        r#"
        SELECT id, content_hash, hash_algorithm, metadata, witness_count, threshold, receipt_json, created_at
        FROM notary_attestations
        ORDER BY created_at DESC
        LIMIT $1
        "#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

// ---------------------------------------------------------------------------
// Read helpers
// ---------------------------------------------------------------------------

/// Fetch round details by UUID.
pub async fn get_round(pool: &PgPool, round_id: Uuid) -> Result<Option<RoundRow>> {
    let row: Option<RoundRow> = sqlx::query_as(
        r#"
        SELECT id, request_id, status, randomness, created_at, finalized_at
        FROM rounds
        WHERE id = $1
        "#,
    )
    .bind(round_id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}
