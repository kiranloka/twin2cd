use crate::auth::hash_token;
use crate::state::AppState;
use axum::{
    Json,
    extract::{Request, State},
    http::{StatusCode, header::AUTHORIZATION},
    middleware::Next,
    response::{IntoResponse, Response},
};
use forge_storage::repos::tokens;
use forge_storage::{Permission, Role};
use serde::Serialize;

#[derive(Debug, Clone)]
pub struct AuthContext {
    pub token_id: String,
    pub organization_id: String,
    pub role: Role,
    pub runner_id: Option<String>,
}

pub async fn require_auth(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    let Some(auth) = extract_bearer(request.headers().get(AUTHORIZATION)) else {
        return unauthorized("missing bearer token").into_response();
    };

    let hash = hash_token(auth);
    let record = match tokens::find_by_hash(state.storage.pool(), &hash).await {
        Ok(Some(r)) => r,
        Ok(None) => return unauthorized("invalid token").into_response(),
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorBody {
                    error: e.to_string(),
                }),
            )
                .into_response();
        }
    };

    request.extensions_mut().insert(AuthContext {
        token_id: record.id,
        organization_id: record.organization_id,
        role: record.role,
        runner_id: record.runner_id,
    });

    next.run(request).await
}

pub fn require_permission(role: Role, permission: Permission) -> bool {
    Permission::allowed_for(role, permission)
}

fn extract_bearer(value: Option<&axum::http::HeaderValue>) -> Option<&str> {
    let header = value?.to_str().ok()?;
    header.strip_prefix("Bearer ").map(str::trim)
}

fn unauthorized(message: &str) -> (StatusCode, Json<ErrorBody>) {
    (
        StatusCode::UNAUTHORIZED,
        Json(ErrorBody {
            error: message.into(),
        }),
    )
}

#[derive(Debug, Serialize)]
pub struct ErrorBody {
    pub error: String,
}
