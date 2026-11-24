use forge_core::ForgeResult;
use serde::Serialize;
use sqlx::AnyPool;
use std::sync::atomic::{AtomicU64, Ordering};

static LOG_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct WorkflowRunRow {
    pub run_id: String,
    pub organization_id: String,
    pub workflow_name: String,
    pub trigger: String,
    pub status: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct WorkflowJobRow {
    pub job_id: String,
    pub run_id: String,
    pub name: String,
    pub command: String,
    pub status: String,
    pub lease_expires_at: Option<i64>,
    pub idempotency_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct RunLogRow {
    pub log_id: String,
    pub stream: String,
    pub line: String,
    pub created_at: i64,
}

pub async fn create_run(
    pool: &AnyPool,
    organization_id: &str,
    run_id: &str,
    project_id: Option<&str>,
    workflow_name: &str,
    trigger: &str,
) -> ForgeResult<()> {
    sqlx::query(
        "INSERT INTO workflow_runs (run_id, organization_id, project_id, workflow_name, trigger, status, created_at)
         VALUES (?, ?, ?, ?, ?, 'queued', ?)",
    )
    .bind(run_id)
    .bind(organization_id)
    .bind(project_id)
    .bind(workflow_name)
    .bind(trigger)
    .bind(unix_now())
    .execute(pool)
    .await
    .map_err(storage_error)?;
    Ok(())
}

pub async fn create_job(
    pool: &AnyPool,
    organization_id: &str,
    run_id: &str,
    job_id: &str,
    name: &str,
    command: &str,
) -> ForgeResult<()> {
    sqlx::query(
        "INSERT INTO workflow_jobs (job_id, run_id, organization_id, name, command, status, created_at)
         SELECT ?, run_id, organization_id, ?, ?, 'queued', ? FROM workflow_runs
         WHERE run_id = ? AND organization_id = ?",
    )
    .bind(job_id)
    .bind(name)
    .bind(command)
    .bind(unix_now())
    .bind(run_id)
    .bind(organization_id)
    .execute(pool)
    .await
    .map_err(storage_error)?;
    Ok(())
}

pub async fn list_runs_for_org(
    pool: &AnyPool,
    organization_id: &str,
    limit: i64,
) -> ForgeResult<Vec<WorkflowRunRow>> {
    sqlx::query_as::<_, WorkflowRunRow>(
        "SELECT run_id, organization_id, workflow_name, trigger, status, created_at
         FROM workflow_runs WHERE organization_id = ? ORDER BY created_at DESC LIMIT ?",
    )
    .bind(organization_id)
    .bind(limit.clamp(1, 200))
    .fetch_all(pool)
    .await
    .map_err(storage_error)
}

pub async fn find_run_for_org(
    pool: &AnyPool,
    organization_id: &str,
    run_id: &str,
) -> ForgeResult<Option<WorkflowRunRow>> {
    sqlx::query_as::<_, WorkflowRunRow>(
        "SELECT run_id, organization_id, workflow_name, trigger, status, created_at
         FROM workflow_runs WHERE organization_id = ? AND run_id = ?",
    )
    .bind(organization_id)
    .bind(run_id)
    .fetch_optional(pool)
    .await
    .map_err(storage_error)
}

pub async fn list_jobs_for_run(
    pool: &AnyPool,
    organization_id: &str,
    run_id: &str,
) -> ForgeResult<Vec<WorkflowJobRow>> {
    sqlx::query_as::<_, WorkflowJobRow>(
        "SELECT job_id, run_id, name, command, status, lease_expires_at, idempotency_key
         FROM workflow_jobs WHERE organization_id = ? AND run_id = ? ORDER BY created_at",
    )
    .bind(organization_id)
    .bind(run_id)
    .fetch_all(pool)
    .await
    .map_err(storage_error)
}

pub async fn claim_next_job(
    pool: &AnyPool,
    organization_id: &str,
    runner_id: &str,
    lease_ttl_secs: u64,
    idempotency_key: &str,
) -> ForgeResult<Option<WorkflowJobRow>> {
    let now = unix_now();
    let expires_at = now.saturating_add(lease_ttl_secs as i64);
    let mut transaction = pool.begin().await.map_err(storage_error)?;
    let reserved = sqlx::query(
        "UPDATE runners SET active_jobs = active_jobs + 1
         WHERE id = ? AND organization_id = ? AND active_jobs < max_concurrent_jobs",
    )
    .bind(runner_id)
    .bind(organization_id)
    .execute(&mut *transaction)
    .await
    .map_err(storage_error)?;
    if reserved.rows_affected() == 0 {
        transaction.commit().await.map_err(storage_error)?;
        return Ok(None);
    }
    let claimed = sqlx::query_as::<_, WorkflowJobRow>(
        "UPDATE workflow_jobs
         SET status = 'leased', runner_id = ?, lease_expires_at = ?, idempotency_key = ?
         WHERE job_id = (
           SELECT job_id FROM workflow_jobs
           WHERE organization_id = ? AND status = 'queued'
           ORDER BY created_at, job_id LIMIT 1
         ) AND organization_id = ? AND status = 'queued'
         RETURNING job_id, run_id, name, command, status, lease_expires_at, idempotency_key",
    )
    .bind(runner_id)
    .bind(expires_at)
    .bind(idempotency_key)
    .bind(organization_id)
    .bind(organization_id)
    .fetch_optional(&mut *transaction)
    .await
    .map_err(storage_error)?;
    if let Some(job) = &claimed {
        sqlx::query(
            "UPDATE workflow_runs SET status = 'running' WHERE run_id = ? AND organization_id = ?",
        )
        .bind(&job.run_id)
        .bind(organization_id)
        .execute(&mut *transaction)
        .await
        .map_err(storage_error)?;
    } else {
        sqlx::query("UPDATE runners SET active_jobs = CASE WHEN active_jobs > 0 THEN active_jobs - 1 ELSE 0 END WHERE id = ? AND organization_id = ?")
            .bind(runner_id)
            .bind(organization_id)
            .execute(&mut *transaction)
            .await
            .map_err(storage_error)?;
    }
    transaction.commit().await.map_err(storage_error)?;
    Ok(claimed)
}

pub async fn renew_job_lease(
    pool: &AnyPool,
    organization_id: &str,
    runner_id: &str,
    job_id: &str,
    idempotency_key: &str,
    extend_secs: u64,
) -> ForgeResult<bool> {
    let result = sqlx::query(
        "UPDATE workflow_jobs SET lease_expires_at = ?
         WHERE organization_id = ? AND job_id = ? AND runner_id = ? AND status = 'leased'
           AND idempotency_key = ? AND lease_expires_at > ?",
    )
    .bind(unix_now().saturating_add(extend_secs as i64))
    .bind(organization_id)
    .bind(job_id)
    .bind(runner_id)
    .bind(idempotency_key)
    .bind(unix_now())
    .execute(pool)
    .await
    .map_err(storage_error)?;
    Ok(result.rows_affected() > 0)
}

pub async fn complete_job(
    pool: &AnyPool,
    organization_id: &str,
    runner_id: &str,
    job_id: &str,
    idempotency_key: &str,
    exit_code: i32,
) -> ForgeResult<bool> {
    let state = if exit_code == 0 {
        "completed"
    } else {
        "failed"
    };
    let mut transaction = pool.begin().await.map_err(storage_error)?;
    let result = sqlx::query(
        "UPDATE workflow_jobs SET status = ?, exit_code = ?, lease_expires_at = NULL
         WHERE organization_id = ? AND job_id = ? AND runner_id = ? AND status = 'leased'
           AND idempotency_key = ?",
    )
    .bind(state)
    .bind(exit_code)
    .bind(organization_id)
    .bind(job_id)
    .bind(runner_id)
    .bind(idempotency_key)
    .execute(&mut *transaction)
    .await
    .map_err(storage_error)?;
    if result.rows_affected() == 0 {
        let current: Option<(String, Option<String>)> = sqlx::query_as(
            "SELECT status, idempotency_key FROM workflow_jobs
             WHERE organization_id = ? AND job_id = ? AND runner_id = ?",
        )
        .bind(organization_id)
        .bind(job_id)
        .bind(runner_id)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(storage_error)?;
        let already_completed = current.is_some_and(|(status, key)| {
            status == state && key.as_deref() == Some(idempotency_key)
        });
        transaction.commit().await.map_err(storage_error)?;
        return Ok(already_completed);
    }
    sqlx::query("UPDATE runners SET active_jobs = CASE WHEN active_jobs > 0 THEN active_jobs - 1 ELSE 0 END WHERE id = ? AND organization_id = ?")
        .bind(runner_id)
        .bind(organization_id)
        .execute(&mut *transaction)
        .await
        .map_err(storage_error)?;
    let run: Option<String> = sqlx::query_scalar(
        "SELECT run_id FROM workflow_jobs WHERE organization_id = ? AND job_id = ?",
    )
    .bind(organization_id)
    .bind(job_id)
    .fetch_optional(&mut *transaction)
    .await
    .map_err(storage_error)?;
    if let Some(run_id) = run {
        let counts: (i64, i64, i64) = sqlx::query_as(
            "SELECT
               SUM(CASE WHEN status IN ('queued', 'leased') THEN 1 ELSE 0 END),
               SUM(CASE WHEN status = 'failed' THEN 1 ELSE 0 END),
               SUM(CASE WHEN status = 'unknown' THEN 1 ELSE 0 END)
             FROM workflow_jobs WHERE organization_id = ? AND run_id = ?",
        )
        .bind(organization_id)
        .bind(&run_id)
        .fetch_one(&mut *transaction)
        .await
        .map_err(storage_error)?;
        if counts.0 == 0 {
            let run_status = if counts.2 > 0 {
                "unknown"
            } else if counts.1 > 0 {
                "failed"
            } else {
                "completed"
            };
            sqlx::query(
                "UPDATE workflow_runs SET status = ? WHERE run_id = ? AND organization_id = ?",
            )
            .bind(run_status)
            .bind(run_id)
            .bind(organization_id)
            .execute(&mut *transaction)
            .await
            .map_err(storage_error)?;
        }
    }
    transaction.commit().await.map_err(storage_error)?;
    Ok(true)
}

/// Expired leases become `unknown` and require an explicit operator or policy
/// decision; the scheduler never silently replays a possibly completed job.
pub async fn mark_expired_jobs_unknown(pool: &AnyPool) -> ForgeResult<u64> {
    let now = unix_now();
    let mut transaction = pool.begin().await.map_err(storage_error)?;
    sqlx::query(
        "UPDATE workflow_runs SET status = 'unknown'
         WHERE run_id IN (SELECT run_id FROM workflow_jobs
           WHERE status = 'leased' AND lease_expires_at <= ?)",
    )
    .bind(now)
    .execute(&mut *transaction)
    .await
    .map_err(storage_error)?;
    let result = sqlx::query(
        "UPDATE workflow_jobs SET status = 'unknown', lease_expires_at = NULL
         WHERE status = 'leased' AND lease_expires_at <= ?",
    )
    .bind(now)
    .execute(&mut *transaction)
    .await
    .map_err(storage_error)?;
    sqlx::query(
        "UPDATE runners SET active_jobs = (SELECT COUNT(*) FROM workflow_jobs
         WHERE workflow_jobs.runner_id = runners.id AND workflow_jobs.status = 'leased')",
    )
    .execute(&mut *transaction)
    .await
    .map_err(storage_error)?;
    transaction.commit().await.map_err(storage_error)?;
    Ok(result.rows_affected())
}

pub async fn reconcile_runner_job_counts(pool: &AnyPool) -> ForgeResult<()> {
    sqlx::query(
        "UPDATE runners SET active_jobs = (SELECT COUNT(*) FROM workflow_jobs
         WHERE workflow_jobs.runner_id = runners.id AND workflow_jobs.status = 'leased')",
    )
    .execute(pool)
    .await
    .map_err(storage_error)?;
    Ok(())
}

pub async fn append_run_log(
    pool: &AnyPool,
    organization_id: &str,
    runner_id: &str,
    run_id: &str,
    stream: &str,
    line: &str,
) -> ForgeResult<bool> {
    let id = format!(
        "log_{}_{}",
        unique_nanos(),
        LOG_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    );
    let result = sqlx::query(
        "INSERT INTO run_logs (log_id, run_id, organization_id, stream, line, created_at)
         SELECT ?, r.run_id, r.organization_id, ?, ?, ? FROM workflow_runs r
         WHERE r.run_id = ? AND r.organization_id = ? AND EXISTS (
           SELECT 1 FROM workflow_jobs j WHERE j.run_id = r.run_id AND j.organization_id = r.organization_id
             AND j.runner_id = ? AND j.status = 'leased'
         )",
    )
    .bind(id)
    .bind(stream)
    .bind(line)
    .bind(unix_now())
    .bind(run_id)
    .bind(organization_id)
    .bind(runner_id)
    .execute(pool)
    .await
    .map_err(storage_error)?;
    Ok(result.rows_affected() > 0)
}

pub async fn list_run_logs(
    pool: &AnyPool,
    organization_id: &str,
    run_id: &str,
    after: Option<&str>,
    limit: i64,
) -> ForgeResult<Vec<RunLogRow>> {
    let rows = sqlx::query_as::<_, RunLogRow>(
        "SELECT log_id, stream, line, created_at FROM run_logs
         WHERE organization_id = ? AND run_id = ? AND (? IS NULL OR log_id > ?)
         ORDER BY created_at, log_id LIMIT ?",
    )
    .bind(organization_id)
    .bind(run_id)
    .bind(after)
    .bind(after)
    .bind(limit.clamp(1, 10_000))
    .fetch_all(pool)
    .await
    .map_err(storage_error)?;
    Ok(rows)
}

fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

fn unique_nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0)
}

fn storage_error(error: sqlx::Error) -> forge_core::ForgeError {
    forge_core::ForgeError::Storage(error.to_string())
}
