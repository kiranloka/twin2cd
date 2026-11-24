use crate::auth::{require_permission, AuthContext};
use crate::state::AppState;
use axum::{extract::State, routing::get, Extension, Json, Router};
use forge_storage::Permission;

pub fn routes() -> Router<AppState> {
    Router::new().route("/api/v1/organizations", get(list_organizations))
}

async fn list_organizations(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<Vec<forge_storage::repos::organizations::OrganizationRow>>, axum::http::StatusCode>
{
    if !require_permission(auth.role, Permission::ReadOrganization) {
        return Err(axum::http::StatusCode::FORBIDDEN);
    }

    let orgs = forge_storage::repos::organizations::list_for_org(
        state.storage.pool(),
        &auth.organization_id,
    )
    .await
    .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(orgs))
}
