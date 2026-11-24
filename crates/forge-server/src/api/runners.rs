use crate::auth::{require_permission, AuthContext};
use crate::state::AppState;
use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Extension, Json, Router,
};
use forge_common::JobId;
use forge_protocol::RunnerCapacity;
use forge_protocol::{JobAssignment, DEFAULT_JOB_LEASE_TTL_SECS};
use forge_storage::Permission;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Deserialize)]
pub struct RegisterRunnerRequest {
    pub runner_id: String,
    pub name: String,
    pub platform: String,
    pub architecture: String,
    pub labels: Vec<String>,
    pub executors: Vec<String>,
    #[serde(default)]
    pub capacity: RunnerCapacity,
}

#[derive(Debug, Deserialize)]
pub struct HeartbeatRequest {
    pub runner_id: String,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/runners", get(list_runners))
        .route("/api/v1/runners/register", post(register_runner))
        .route("/api/v1/runners/heartbeat", post(runner_heartbeat))
        .route("/api/v1/runners/jobs/next", get(next_job))
        .route("/api/v1/runners/jobs/{job_id}/lease", post(renew_job_lease))
        .route("/api/v1/runners/jobs/{job_id}/complete", post(complete_job))
}

async fn list_runners(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<Vec<RunnerResponse>>, StatusCode> {
    if !require_permission(auth.role, Permission::ReadOrganization) {
        return Err(StatusCode::FORBIDDEN);
    }
    let rows =
        forge_storage::repos::runners::list_for_org(state.storage.pool(), &auth.organization_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let rows = rows
        .into_iter()
        .map(|row| {
            let labels = serde_json::from_str(&row.labels_json)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            let executors = serde_json::from_str(&row.executors_json)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            let capacity = serde_json::from_str(&row.capacity_json)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            let recent = row
                .last_heartbeat_at
                .as_deref()
                .and_then(|time| time.parse::<u64>().ok())
                .is_some_and(|time| unix_now().saturating_sub(time) <= 45);
            Ok(RunnerResponse {
                id: row.id,
                name: row.name,
                status: if recent { row.status } else { "offline".into() },
                platform: row.platform,
                architecture: row.architecture,
                labels,
                executors,
                capacity,
                active_jobs: row.active_jobs,
                last_heartbeat_at: row.last_heartbeat_at,
            })
        })
        .collect::<Result<Vec<_>, StatusCode>>()?;
    Ok(Json(rows))
}

async fn register_runner(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(body): Json<RegisterRunnerRequest>,
) -> Result<Json<RegisterRunnerResponse>, StatusCode> {
    if !require_permission(auth.role, Permission::ManageRunners) {
        return Err(StatusCode::FORBIDDEN);
    }

    let labels_json = serde_json::to_string(&body.labels).map_err(|_| StatusCode::BAD_REQUEST)?;
    let executors_json =
        serde_json::to_string(&body.executors).map_err(|_| StatusCode::BAD_REQUEST)?;
    let capacity_json =
        serde_json::to_string(&body.capacity).map_err(|_| StatusCode::BAD_REQUEST)?;

    let inserted = forge_storage::repos::runners::upsert(
        state.storage.pool(),
        &auth.organization_id,
        &body.runner_id,
        &body.name,
        &body.platform,
        &body.architecture,
        &labels_json,
        &executors_json,
        &capacity_json,
        body.capacity.max_concurrent_jobs,
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if !inserted {
        return Err(StatusCode::CONFLICT);
    }

    let mut random = [0u8; 32];
    getrandom::getrandom(&mut random).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let token = format!("frt_{}", hex::encode(random));
    let token_hash = hex::encode(Sha256::digest(token.as_bytes()));
    let token_saved = forge_storage::repos::tokens::insert_runner_token(
        state.storage.pool(),
        &auth.organization_id,
        &body.runner_id,
        &token_hash,
        &format!("runner:{}", body.name),
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if !token_saved {
        return Err(StatusCode::CONFLICT);
    }

    let _ = forge_storage::repos::audit::write(
        state.storage.pool(),
        &auth.organization_id,
        Some(&auth.token_id),
        "runner.register",
        &body.runner_id,
        None,
    )
    .await;

    Ok(Json(RegisterRunnerResponse {
        runner_id: body.runner_id,
        token,
    }))
}

async fn runner_heartbeat(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(body): Json<HeartbeatRequest>,
) -> Result<Json<HeartbeatResponse>, StatusCode> {
    let runner_token_matches = auth.runner_id.as_deref() == Some(body.runner_id.as_str());
    if !runner_token_matches && !require_permission(auth.role, Permission::ManageRunners) {
        return Err(StatusCode::FORBIDDEN);
    }

    let ok = forge_storage::repos::runners::record_heartbeat(
        state.storage.pool(),
        &auth.organization_id,
        &body.runner_id,
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !ok {
        return Err(StatusCode::NOT_FOUND);
    }

    Ok(Json(HeartbeatResponse { ok: true }))
}

#[derive(Serialize)]
struct HeartbeatResponse {
    ok: bool,
}

#[derive(Serialize)]
struct RegisterRunnerResponse {
    runner_id: String,
    token: String,
}

async fn next_job(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<Option<JobAssignment>>, StatusCode> {
    let runner_id = auth.runner_id.as_deref().ok_or(StatusCode::FORBIDDEN)?;
    let mut key_bytes = [0u8; 16];
    getrandom::getrandom(&mut key_bytes).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let idempotency_key = hex::encode(key_bytes);
    let job = forge_storage::repos::executions::claim_next_job(
        state.storage.pool(),
        &auth.organization_id,
        runner_id,
        DEFAULT_JOB_LEASE_TTL_SECS,
        &idempotency_key,
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(job.map(|job| JobAssignment {
        job_id: JobId::new(job.job_id),
        run_id: job.run_id,
        name: job.name,
        command: job.command,
        lease_ttl_secs: DEFAULT_JOB_LEASE_TTL_SECS,
        idempotency_key,
    })))
}

#[derive(Debug, Deserialize)]
struct RenewLeaseRequest {
    idempotency_key: String,
    #[serde(default = "default_lease_extension")]
    extend_secs: u64,
}

async fn renew_job_lease(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    axum::extract::Path(job_id): axum::extract::Path<String>,
    Json(body): Json<RenewLeaseRequest>,
) -> Result<StatusCode, StatusCode> {
    let runner_id = auth.runner_id.as_deref().ok_or(StatusCode::FORBIDDEN)?;
    let extended = forge_storage::repos::executions::renew_job_lease(
        state.storage.pool(),
        &auth.organization_id,
        runner_id,
        &job_id,
        &body.idempotency_key,
        body.extend_secs.clamp(1, 300),
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if !extended {
        return Err(StatusCode::CONFLICT);
    }
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
struct CompleteJobRequest {
    idempotency_key: String,
    exit_code: i32,
}

async fn complete_job(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    axum::extract::Path(job_id): axum::extract::Path<String>,
    Json(body): Json<CompleteJobRequest>,
) -> Result<StatusCode, StatusCode> {
    let runner_id = auth.runner_id.as_deref().ok_or(StatusCode::FORBIDDEN)?;
    let accepted = forge_storage::repos::executions::complete_job(
        state.storage.pool(),
        &auth.organization_id,
        runner_id,
        &job_id,
        &body.idempotency_key,
        body.exit_code,
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if !accepted {
        return Err(StatusCode::CONFLICT);
    }
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Serialize)]
struct RunnerResponse {
    id: String,
    name: String,
    status: String,
    platform: String,
    architecture: String,
    labels: Vec<String>,
    executors: Vec<String>,
    capacity: RunnerCapacity,
    active_jobs: i64,
    last_heartbeat_at: Option<String>,
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn default_lease_extension() -> u64 {
    DEFAULT_JOB_LEASE_TTL_SECS
}
