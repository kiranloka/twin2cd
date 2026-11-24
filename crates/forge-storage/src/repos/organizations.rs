use forge_core::ForgeResult;
use serde::Serialize;
use sqlx::AnyPool;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct OrganizationRow {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub created_at: String,
}

pub async fn list_for_org(
    pool: &AnyPool,
    organization_id: &str,
) -> ForgeResult<Vec<OrganizationRow>> {
    let rows = sqlx::query_as::<_, OrganizationRow>(
        "SELECT id, slug, name, created_at FROM organizations WHERE id = ? ORDER BY created_at",
    )
    .bind(organization_id)
    .fetch_all(pool)
    .await
    .map_err(|e| forge_core::ForgeError::Storage(e.to_string()))?;
    Ok(rows)
}

pub async fn ensure_default(pool: &AnyPool) -> ForgeResult<OrganizationRow> {
    let existing = sqlx::query_as::<_, OrganizationRow>(
        "SELECT id, slug, name, created_at FROM organizations WHERE slug = 'default' LIMIT 1",
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| forge_core::ForgeError::Storage(e.to_string()))?;

    if let Some(row) = existing {
        return Ok(row);
    }

    let id = "org_default";
    let created_at = utc_now();
    sqlx::query("INSERT INTO organizations (id, slug, name, created_at) VALUES (?, ?, ?, ?)")
        .bind(id)
        .bind("default")
        .bind("Default")
        .bind(&created_at)
        .execute(pool)
        .await
        .map_err(|e| forge_core::ForgeError::Storage(e.to_string()))?;

    Ok(OrganizationRow {
        id: id.into(),
        slug: "default".into(),
        name: "Default".into(),
        created_at,
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
