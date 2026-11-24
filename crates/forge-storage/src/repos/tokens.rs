use crate::rbac::Role;
use forge_core::ForgeResult;
use sqlx::AnyPool;

#[derive(Debug, Clone)]
pub struct ApiTokenRecord {
    pub id: String,
    pub organization_id: String,
    pub role: Role,
    pub runner_id: Option<String>,
}

pub async fn find_by_hash(pool: &AnyPool, token_hash: &str) -> ForgeResult<Option<ApiTokenRecord>> {
    let row: Option<(String, String, String, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT id, organization_id, role, runner_id, expires_at FROM api_tokens WHERE token_hash = ? LIMIT 1",
    )
    .bind(token_hash)
    .fetch_optional(pool)
    .await
    .map_err(|e| forge_core::ForgeError::Storage(e.to_string()))?;

    let now = unix_now();
    Ok(
        row.and_then(|(id, organization_id, role, runner_id, expires_at)| {
            if let Some(expiry) = expires_at.as_deref() {
                if expiry.parse::<u64>().map_or(true, |expiry| expiry <= now) {
                    return None;
                }
            }
            Role::parse(&role).map(|role| ApiTokenRecord {
                id,
                organization_id,
                role,
                runner_id,
            })
        }),
    )
}

pub async fn insert_runner_token(
    pool: &AnyPool,
    organization_id: &str,
    runner_id: &str,
    token_hash: &str,
    name: &str,
) -> ForgeResult<bool> {
    let result = sqlx::query(
        "INSERT INTO api_tokens (id, organization_id, token_hash, name, role, created_at, runner_id)
         VALUES (?, ?, ?, ?, 'runner', ?, ?)
         ON CONFLICT(id) DO UPDATE SET token_hash = excluded.token_hash,
           name = excluded.name, created_at = excluded.created_at, expires_at = NULL
         WHERE api_tokens.organization_id = excluded.organization_id
           AND api_tokens.runner_id = excluded.runner_id",
    )
    .bind(format!("tok_{runner_id}"))
    .bind(organization_id)
    .bind(token_hash)
    .bind(name)
    .bind(unix_now().to_string())
    .bind(runner_id)
    .execute(pool)
    .await
    .map_err(|error| forge_core::ForgeError::Storage(error.to_string()))?;
    Ok(result.rows_affected() > 0)
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

pub async fn insert_dev_token(
    pool: &AnyPool,
    organization_id: &str,
    token_hash: &str,
    name: &str,
    role: Role,
) -> ForgeResult<()> {
    let id = "tok_dev";
    let created_at = "0";
    sqlx::query(
        "INSERT INTO api_tokens (id, organization_id, token_hash, name, role, created_at)
         VALUES (?, ?, ?, ?, ?, ?)
         ON CONFLICT(id) DO NOTHING",
    )
    .bind(id)
    .bind(organization_id)
    .bind(token_hash)
    .bind(name)
    .bind(role.as_str())
    .bind(created_at)
    .execute(pool)
    .await
    .map_err(|e| forge_core::ForgeError::Storage(e.to_string()))?;
    Ok(())
}
