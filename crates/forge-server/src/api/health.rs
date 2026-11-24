use crate::state::AppState;
use axum::{extract::State, routing::get, Json, Router};
use serde::Serialize;

#[derive(Serialize)]
pub struct HealthResponse {
    status: &'static str,
    database: &'static str,
    backend: String,
    cloud: bool,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(health_plain))
        .route("/api/v1/health", get(health_v1))
}

async fn health_plain() -> &'static str {
    "ok"
}

async fn health_v1(State(state): State<AppState>) -> Json<HealthResponse> {
    let database = match state.storage.ping().await {
        Ok(()) => "ok",
        Err(_) => "error",
    };
    Json(HealthResponse {
        status: if database == "ok" { "ok" } else { "degraded" },
        database,
        backend: format!("{:?}", state.storage.backend()).to_lowercase(),
        cloud: state.cloud,
    })
}
