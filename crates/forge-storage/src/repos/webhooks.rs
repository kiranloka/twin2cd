use forge_core::ForgeResult;
use serde::Serialize;
use sqlx::AnyPool;

pub struct PushWorkflow<'a> {
    pub project_id: &'a str,
    pub workflow_name: &'a str,
    pub jobs: &'a [(String, String)],
}

/// Stores a delivery and queues all matching push workflows in one transaction.
/// GitHub retries are idempotent because the delivery key and runs commit together.
pub async fn record_push_and_queue(
    pool: &AnyPool,
    organization_id: &str,
    delivery_id: &str,
    payload_json: &str,
    workflows: &[PushWorkflow<'_>],
) -> ForgeResult<bool> {
    let mut transaction = pool.begin().await.map_err(storage_error)?;
    let inserted = sqlx::query(
        "INSERT INTO webhook_deliveries (organization_id, delivery_id, event, payload_json, received_at)
         VALUES (?, ?, 'push', ?, ?) ON CONFLICT (organization_id, delivery_id) DO NOTHING",
    )
    .bind(organization_id)
    .bind(delivery_id)
    .bind(payload_json)
    .bind(unix_now())
    .execute(&mut *transaction)
    .await
    .map_err(storage_error)?
    .rows_affected() > 0;
    if !inserted {
        transaction.commit().await.map_err(storage_error)?;
        return Ok(false);
    }

    let trigger = format!("github:{delivery_id}");
    for (workflow_index, workflow) in workflows.iter().enumerate() {
        let run_id = format!("run_{}_{}", unique_id(), workflow_index);
        sqlx::query(
            "INSERT INTO workflow_runs (run_id, organization_id, project_id, workflow_name, trigger, status, created_at)
             VALUES (?, ?, ?, ?, ?, 'queued', ?)",
        )
        .bind(&run_id)
        .bind(organization_id)
        .bind(workflow.project_id)
        .bind(workflow.workflow_name)
        .bind(&trigger)
        .bind(unix_now_i64())
        .execute(&mut *transaction)
        .await
        .map_err(storage_error)?;
        for (index, (name, command)) in workflow.jobs.iter().enumerate() {
            sqlx::query(
                "INSERT INTO workflow_jobs (job_id, run_id, organization_id, name, command, status, created_at)
                 VALUES (?, ?, ?, ?, ?, 'queued', ?)",
            )
            .bind(format!("{run_id}_{index}"))
            .bind(&run_id)
            .bind(organization_id)
            .bind(name)
            .bind(command)
            .bind(unix_now_i64())
            .execute(&mut *transaction)
            .await
            .map_err(storage_error)?;
        }
    }
    transaction.commit().await.map_err(storage_error)?;
    Ok(true)
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct WebhookDeliveryRow {
    pub delivery_id: String,
    pub event: String,
    pub received_at: String,
}

pub async fn record_delivery(
    pool: &AnyPool,
    organization_id: &str,
    delivery_id: &str,
    event: &str,
    payload_json: &str,
) -> ForgeResult<bool> {
    let result = sqlx::query(
        "INSERT INTO webhook_deliveries (organization_id, delivery_id, event, payload_json, received_at)
         VALUES (?, ?, ?, ?, ?)
         ON CONFLICT (organization_id, delivery_id) DO NOTHING",
    )
    .bind(organization_id)
    .bind(delivery_id)
    .bind(event)
    .bind(payload_json)
    .bind(unix_now())
    .execute(pool)
    .await
    .map_err(|error| forge_core::ForgeError::Storage(error.to_string()))?;
    Ok(result.rows_affected() > 0)
}

pub async fn list_for_org(
    pool: &AnyPool,
    organization_id: &str,
    limit: i64,
) -> ForgeResult<Vec<WebhookDeliveryRow>> {
    let rows = sqlx::query_as::<_, WebhookDeliveryRow>(
        "SELECT delivery_id, event, received_at FROM webhook_deliveries
         WHERE organization_id = ? ORDER BY received_at DESC LIMIT ?",
    )
    .bind(organization_id)
    .bind(limit.clamp(1, 200))
    .fetch_all(pool)
    .await
    .map_err(|error| forge_core::ForgeError::Storage(error.to_string()))?;
    Ok(rows)
}

fn unix_now() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".into())
}

fn unix_now_i64() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

fn unique_id() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0)
}

fn storage_error(error: sqlx::Error) -> forge_core::ForgeError {
    forge_core::ForgeError::Storage(error.to_string())
}
