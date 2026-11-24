use crate::{
    auth::{require_permission, AuthContext},
    state::AppState,
};
use axum::{extract::State, http::StatusCode, routing::get, Extension, Json, Router};
use forge_storage::Permission;
use serde::{Deserialize, Serialize};

pub fn routes() -> Router<AppState> {
    Router::new().route("/api/v1/secrets", get(list_secrets).put(set_secret))
}

#[derive(Debug, Serialize)]
struct SecretNames {
    names: Vec<String>,
}

async fn list_secrets(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<SecretNames>, StatusCode> {
    if !require_permission(auth.role, Permission::ManageSecrets) {
        return Err(StatusCode::FORBIDDEN);
    }
    let manager = state.engine_for_org(&auth.organization_id).secrets();
    Ok(Json(SecretNames {
        names: manager.names(),
    }))
}

#[derive(Debug, Deserialize)]
struct SetSecretRequest {
    name: String,
    value: String,
}

#[derive(Debug, Serialize)]
struct SecretSetResponse {
    name: String,
    stored: bool,
}

async fn set_secret(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(body): Json<SetSecretRequest>,
) -> Result<(StatusCode, Json<SecretSetResponse>), StatusCode> {
    if !require_permission(auth.role, Permission::ManageSecrets) {
        return Err(StatusCode::FORBIDDEN);
    }
    let valid_name = !body.name.is_empty()
        && body.name.len() <= 128
        && body
            .name
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_');
    if !valid_name || body.value.len() > 65_536 {
        return Err(StatusCode::BAD_REQUEST);
    }
    state
        .engine_for_org(&auth.organization_id)
        .secrets()
        .set(body.name.clone(), body.value);
    Ok((
        StatusCode::OK,
        Json(SecretSetResponse {
            name: body.name,
            stored: true,
        }),
    ))
}
