use forge_core::ForgeResult;
use serde::Serialize;
use sqlx::AnyPool;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct BillingAccountRow {
    pub organization_id: String,
    pub plan: String,
    pub stripe_customer_id: Option<String>,
    pub updated_at: String,
}

pub async fn ensure_free(pool: &AnyPool, organization_id: &str) -> ForgeResult<BillingAccountRow> {
    let existing = sqlx::query_as::<_, BillingAccountRow>(
        "SELECT organization_id, plan, stripe_customer_id, updated_at FROM billing_accounts WHERE organization_id = ?",
    )
    .bind(organization_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| forge_core::ForgeError::Storage(e.to_string()))?;

    if let Some(row) = existing {
        return Ok(row);
    }

    let updated_at = utc_now();
    sqlx::query(
        "INSERT INTO billing_accounts (organization_id, plan, updated_at) VALUES (?, 'free', ?)",
    )
    .bind(organization_id)
    .bind(&updated_at)
    .execute(pool)
    .await
    .map_err(|e| forge_core::ForgeError::Storage(e.to_string()))?;

    Ok(BillingAccountRow {
        organization_id: organization_id.into(),
        plan: "free".into(),
        stripe_customer_id: None,
        updated_at,
    })
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
