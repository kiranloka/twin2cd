use forge_core::ForgeResult;
use serde::Serialize;
use sqlx::AnyPool;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct ProjectRow {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub repository: String,
    pub default_branch: String,
    pub last_run_at: Option<i64>,
    pub workflow_count: i64,
}

pub async fn list_for_org(pool: &AnyPool, organization_id: &str) -> ForgeResult<Vec<ProjectRow>> {
    sqlx::query_as::<_, ProjectRow>(
        "SELECT p.id, p.name, p.slug, p.repository, p.default_branch,
           (SELECT MAX(r.created_at) FROM workflow_runs r
             WHERE r.organization_id = p.organization_id AND r.project_id = p.id) AS last_run_at,
           (SELECT COUNT(*) FROM workflow_definitions w
             WHERE w.organization_id = p.organization_id AND w.project_id = p.id) AS workflow_count
         FROM projects p WHERE p.organization_id = ? ORDER BY p.name",
    )
    .bind(organization_id)
    .fetch_all(pool)
    .await
    .map_err(storage_error)
}

pub async fn create(
    pool: &AnyPool,
    organization_id: &str,
    name: &str,
    slug: &str,
    repository: &str,
    default_branch: &str,
) -> ForgeResult<ProjectRow> {
    let id = format!("prj_{}", unique_suffix());
    sqlx::query(
        "INSERT INTO projects (id, organization_id, slug, name, repository, default_branch, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(organization_id)
    .bind(slug)
    .bind(name)
    .bind(repository)
    .bind(default_branch)
    .bind(unix_now())
    .execute(pool)
    .await
    .map_err(storage_error)?;
    Ok(ProjectRow {
        id,
        name: name.into(),
        slug: slug.into(),
        repository: repository.into(),
        default_branch: default_branch.into(),
        last_run_at: None,
        workflow_count: 0,
    })
}

pub async fn belongs_to_org(
    pool: &AnyPool,
    organization_id: &str,
    project_id: &str,
) -> ForgeResult<bool> {
    let row: Option<(i64,)> =
        sqlx::query_as("SELECT 1 FROM projects WHERE organization_id = ? AND id = ?")
            .bind(organization_id)
            .bind(project_id)
            .fetch_optional(pool)
            .await
            .map_err(storage_error)?;
    Ok(row.is_some())
}

fn unique_suffix() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos().to_string())
        .unwrap_or_else(|_| "0".into())
}

fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

fn storage_error(error: sqlx::Error) -> forge_core::ForgeError {
    forge_core::ForgeError::Storage(error.to_string())
}
