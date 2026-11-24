mod billing;
mod health;
mod organizations;
mod projects;
mod runners;
mod runs;
mod secrets;
mod webhooks;
mod workflows;

use crate::auth::require_auth;
use crate::state::AppState;
use axum::{middleware, routing::get, Router};

pub fn router(state: AppState) -> Router {
    let public = Router::new()
        .merge(health::routes())
        .merge(workflows::public_routes())
        .merge(webhooks::public_routes());

    let protected = Router::new()
        .merge(workflows::protected_routes())
        .merge(webhooks::protected_routes())
        .merge(organizations::routes())
        .merge(projects::routes())
        .merge(runs::routes())
        .merge(secrets::routes())
        .merge(runners::routes())
        .merge(billing::routes())
        .route("/api/v1/auth/whoami", get(crate::api::auth_whoami))
        .layer(middleware::from_fn_with_state(state.clone(), require_auth));

    Router::new()
        .merge(public)
        .merge(protected)
        .with_state(state)
}

async fn auth_whoami(
    axum::Extension(auth): axum::Extension<crate::auth::AuthContext>,
) -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "organization_id": auth.organization_id,
        "token_id": auth.token_id,
        "role": auth.role,
    }))
}
