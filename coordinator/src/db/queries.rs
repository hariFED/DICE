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
