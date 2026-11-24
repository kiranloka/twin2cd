use forge_core::ForgeResult;
use sqlx::AnyPool;

pub async fn write(
    pool: &AnyPool,
    organization_id: &str,
    actor_id: Option<&str>,
    action: &str,
    resource: &str,
    metadata_json: Option<&str>,
) -> ForgeResult<()> {
    let id = new_audit_id();
    let created_at = utc_now();
    sqlx::query(
        "INSERT INTO audit_logs (id, organization_id, actor_id, action, resource, metadata_json, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(organization_id)
    .bind(actor_id)
    .bind(action)
    .bind(resource)
    .bind(metadata_json)
    .bind(created_at)
    .execute(pool)
    .await
    .map_err(|e| forge_core::ForgeError::Storage(e.to_string()))?;
    Ok(())
}

fn new_audit_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("aud_{ns}")
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
