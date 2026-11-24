use forge_core::ForgeResult;
use serde::Serialize;
use sqlx::AnyPool;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct RunnerRow {
    pub id: String,
    pub organization_id: String,
    pub name: String,
    pub platform: String,
    pub architecture: String,
    pub labels_json: String,
    pub executors_json: String,
    pub capacity_json: String,
    pub status: String,
    pub last_heartbeat_at: Option<String>,
    pub active_jobs: i64,
}

pub async fn list_for_org(pool: &AnyPool, organization_id: &str) -> ForgeResult<Vec<RunnerRow>> {
    let rows = sqlx::query_as::<_, RunnerRow>(
        "SELECT r.id, r.organization_id, r.name, r.platform, r.architecture, r.labels_json, r.executors_json,
           r.capacity_json, r.status, r.last_heartbeat_at, r.active_jobs
         FROM runners r WHERE r.organization_id = ? ORDER BY r.name",
    )
    .bind(organization_id)
    .fetch_all(pool)
    .await
    .map_err(|e| forge_core::ForgeError::Storage(e.to_string()))?;
    Ok(rows)
}

pub async fn upsert(
    pool: &AnyPool,
    organization_id: &str,
    id: &str,
    name: &str,
    platform: &str,
    architecture: &str,
    labels_json: &str,
    executors_json: &str,
    capacity_json: &str,
    max_concurrent_jobs: u32,
) -> ForgeResult<bool> {
    let result = sqlx::query(
        "INSERT INTO runners (id, organization_id, name, platform, architecture, labels_json, executors_json, capacity_json, max_concurrent_jobs, status, last_heartbeat_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 'online', ?)
         ON CONFLICT(id) DO UPDATE SET
           name = excluded.name,
           platform = excluded.platform,
           architecture = excluded.architecture,
           labels_json = excluded.labels_json,
           executors_json = excluded.executors_json,
           capacity_json = excluded.capacity_json,
           max_concurrent_jobs = excluded.max_concurrent_jobs,
           status = 'online',
           last_heartbeat_at = excluded.last_heartbeat_at
         WHERE runners.organization_id = excluded.organization_id",
    )
    .bind(id)
    .bind(organization_id)
    .bind(name)
    .bind(platform)
    .bind(architecture)
    .bind(labels_json)
    .bind(executors_json)
    .bind(capacity_json)
    .bind(max_concurrent_jobs.max(1) as i64)
    .bind(utc_now())
    .execute(pool)
    .await
    .map_err(|e| forge_core::ForgeError::Storage(e.to_string()))?;
    Ok(result.rows_affected() > 0)
}

pub async fn record_heartbeat(
    pool: &AnyPool,
    organization_id: &str,
    runner_id: &str,
) -> ForgeResult<bool> {
    let result = sqlx::query(
        "UPDATE runners SET last_heartbeat_at = ?, status = 'online'
         WHERE id = ? AND organization_id = ?",
    )
    .bind(utc_now())
    .bind(runner_id)
    .bind(organization_id)
    .execute(pool)
    .await
    .map_err(|e| forge_core::ForgeError::Storage(e.to_string()))?;
    Ok(result.rows_affected() > 0)
}

pub async fn set_offline(
    pool: &AnyPool,
    organization_id: &str,
    runner_id: &str,
) -> ForgeResult<bool> {
    let result =
        sqlx::query("UPDATE runners SET status = 'offline' WHERE id = ? AND organization_id = ?")
            .bind(runner_id)
            .bind(organization_id)
            .execute(pool)
            .await
            .map_err(|error| forge_core::ForgeError::Storage(error.to_string()))?;
    Ok(result.rows_affected() > 0)
}

fn utc_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    format!(
        "{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    )
}
