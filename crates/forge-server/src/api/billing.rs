use crate::auth::{require_permission, AuthContext};
use crate::state::AppState;
use axum::{extract::State, http::StatusCode, routing::get, Extension, Json, Router};
use forge_storage::Permission;

pub fn routes() -> Router<AppState> {
    Router::new().route("/api/v1/billing/plan", get(get_plan))
}

async fn get_plan(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<forge_storage::repos::billing::BillingAccountRow>, StatusCode> {
    if !require_permission(auth.role, Permission::ManageBilling)
        && !require_permission(auth.role, Permission::ReadOrganization)
    {
        return Err(StatusCode::FORBIDDEN);
    }

    let row =
        forge_storage::repos::billing::ensure_free(state.storage.pool(), &auth.organization_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(row))
}
