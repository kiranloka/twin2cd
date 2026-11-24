use crate::{
    auth::{require_permission, AuthContext},
    state::AppState,
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
    Extension, Json, Router,
};
use forge_common::RunId;
use forge_core::{JobRun, Run};
use forge_storage::repos::executions::{self, RunLogRow, WorkflowJobRow, WorkflowRunRow};
use forge_storage::Permission;
use forge_workflow::{WorkflowCompiler, WorkflowParser};
use serde::{Deserialize, Serialize};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/runs", get(list_runs).post(start_inprocess_run))
        .route("/api/v1/runs/{run_id}", get(get_inprocess_run))
        .route("/api/v1/runs/{run_id}/cancel", post(cancel_inprocess_run))
        .route("/api/v1/runs/{run_id}/jobs", get(list_jobs))
        .route(
            "/api/v1/runs/{run_id}/logs",
            get(list_logs).post(append_log),
        )
}

#[derive(Debug, Deserialize)]
struct TriggerRunRequest {
    yaml: String,
    #[serde(default = "manual_trigger")]
    trigger: String,
}

#[derive(Debug, Serialize)]
struct TriggerRunResponse {
    run_id: RunId,
    status: forge_core::RunStatus,
}

async fn start_inprocess_run(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(body): Json<TriggerRunRequest>,
) -> Result<(StatusCode, Json<TriggerRunResponse>), StatusCode> {
    if !require_permission(auth.role, Permission::ManageProjects) {
        return Err(StatusCode::FORBIDDEN);
    }
    if body.yaml.len() > 1_048_576 {
        return Err(StatusCode::PAYLOAD_TOO_LARGE);
    }
    let document = WorkflowParser::parse_str(&body.yaml).map_err(|_| StatusCode::BAD_REQUEST)?;
    let compiled = WorkflowCompiler::compile(document).map_err(|_| StatusCode::BAD_REQUEST)?;
    let engine = state.engine_for_org(&auth.organization_id);
    let run_id = engine
        .trigger(compiled, &body.trigger)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let run = engine
        .store()
        .get_run(&run_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((
        StatusCode::ACCEPTED,
        Json(TriggerRunResponse {
            run_id,
            status: run.status,
        }),
    ))
}

#[derive(Debug, Serialize)]
struct InProcessRunResponse {
    run: Run,
    jobs: Vec<JobRun>,
}

async fn get_inprocess_run(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(run_id): Path<String>,
) -> Result<Json<InProcessRunResponse>, StatusCode> {
    if !require_permission(auth.role, Permission::ReadOrganization) {
        return Err(StatusCode::FORBIDDEN);
    }
    let engine = state.engine_for_org(&auth.organization_id);
    let id = RunId::new(run_id);
    let store = engine.store();
    let run = store
        .get_run(&id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let mut jobs = store
        .list_jobs_for_run(&id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    // Job records retain the resolved environment for debugging/execution, but
    // that environment can contain secrets and must never be returned by the API.
    for job in &mut jobs {
        job.env.clear();
    }
    Ok(Json(InProcessRunResponse { run, jobs }))
}

async fn cancel_inprocess_run(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(run_id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    if !require_permission(auth.role, Permission::ManageProjects) {
        return Err(StatusCode::FORBIDDEN);
    }
    state
        .engine_for_org(&auth.organization_id)
        .cancel(&RunId::new(run_id))
        .await
        .map_err(|error| {
            if error.to_string().contains("not found") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::CONFLICT
            }
        })?;
    Ok(StatusCode::ACCEPTED)
}

fn manual_trigger() -> String {
    "manual".into()
}

async fn list_runs(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<Vec<WorkflowRunRow>>, StatusCode> {
    if !require_permission(auth.role, Permission::ReadOrganization) {
        return Err(StatusCode::FORBIDDEN);
    }
    let mut rows = executions::list_runs_for_org(state.storage.pool(), &auth.organization_id, 100)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let engine = state.engine_for_org(&auth.organization_id);
    let inprocess = engine
        .store()
        .list_runs()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    rows.extend(inprocess.into_iter().map(|run| {
        WorkflowRunRow {
            run_id: run.id.to_string(),
            organization_id: auth.organization_id.clone(),
            workflow_name: run.workflow_name,
            trigger: run.trigger,
            status: match run.status {
                forge_core::RunStatus::Pending => "pending",
                forge_core::RunStatus::InProgress => "in_progress",
                forge_core::RunStatus::Completed => "completed",
                forge_core::RunStatus::Failed => "failed",
                forge_core::RunStatus::Cancelled => "cancelled",
            }
            .into(),
            created_at: (run.created_at_ms / 1000) as i64,
        }
    }));
    rows.sort_by(|a, b| {
        b.created_at
            .cmp(&a.created_at)
            .then_with(|| a.run_id.cmp(&b.run_id))
    });
    rows.truncate(100);
    Ok(Json(rows))
}

async fn list_jobs(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(run_id): Path<String>,
) -> Result<Json<Vec<WorkflowJobRow>>, StatusCode> {
    if !require_permission(auth.role, Permission::ReadOrganization) {
        return Err(StatusCode::FORBIDDEN);
    }
    let engine = state.engine_for_org(&auth.organization_id);
    if let Ok(jobs) = engine
        .store()
        .list_jobs_for_run(&RunId::new(run_id.clone()))
        .await
    {
        return Ok(Json(
            jobs.into_iter()
                .map(|mut job| {
                    job.env.clear();
                    WorkflowJobRow {
                        job_id: job.id.to_string(),
                        run_id: job.run_id.to_string(),
                        name: job.name,
                        command: String::new(),
                        status: match job.status {
                            forge_core::JobStatus::Pending => "pending",
                            forge_core::JobStatus::Ready => "ready",
                            forge_core::JobStatus::Running => "running",
                            forge_core::JobStatus::Completed => "completed",
                            forge_core::JobStatus::Failed => "failed",
                            forge_core::JobStatus::Skipped => "skipped",
                            forge_core::JobStatus::Cancelled => "cancelled",
                        }
                        .into(),
                        lease_expires_at: None,
                        idempotency_key: None,
                    }
                })
                .collect(),
        ));
    }
    if executions::find_run_for_org(state.storage.pool(), &auth.organization_id, &run_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .is_none()
    {
        return Err(StatusCode::NOT_FOUND);
    }
    let rows = executions::list_jobs_for_run(state.storage.pool(), &auth.organization_id, &run_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(rows))
}

#[derive(Debug, Deserialize)]
struct LogQuery {
    after: Option<String>,
}

async fn list_logs(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(run_id): Path<String>,
    Query(query): Query<LogQuery>,
) -> Result<Json<Vec<RunLogRow>>, StatusCode> {
    if !require_permission(auth.role, Permission::ReadOrganization) {
        return Err(StatusCode::FORBIDDEN);
    }
    let engine = state.engine_for_org(&auth.organization_id);
    if let Ok(run) = engine.store().get_run(&RunId::new(run_id.clone())).await {
        let jobs = engine
            .store()
            .list_jobs_for_run(&run.id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let mut logs = Vec::new();
        for job in jobs {
            for step in job.steps {
                for (stream, output) in [("stdout", step.stdout), ("stderr", step.stderr)] {
                    for (line_idx, line) in output.lines().enumerate() {
                        let log_id = format!(
                            "{:020}:{}:{:08}:{}:{:08}",
                            step.started_at_ms.unwrap_or(step.created_at_ms),
                            job.id,
                            step.index,
                            stream,
                            line_idx
                        );
                        if query
                            .after
                            .as_ref()
                            .is_some_and(|after| log_id.as_str() <= after.as_str())
                        {
                            continue;
                        }
                        logs.push(RunLogRow {
                            log_id,
                            stream: stream.into(),
                            line: line.into(),
                            created_at: (step.started_at_ms.unwrap_or(step.created_at_ms) / 1000)
                                as i64,
                        });
                    }
                }
            }
        }
        logs.sort_by(|a, b| a.log_id.cmp(&b.log_id));
        logs.truncate(2_000);
        return Ok(Json(logs));
    }
    if executions::find_run_for_org(state.storage.pool(), &auth.organization_id, &run_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .is_none()
    {
        return Err(StatusCode::NOT_FOUND);
    }
    let rows = executions::list_run_logs(
        state.storage.pool(),
        &auth.organization_id,
        &run_id,
        query.after.as_deref(),
        2_000,
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(rows))
}

#[derive(Debug, Deserialize)]
struct AppendLogRequest {
    stream: String,
    line: String,
}

#[derive(Serialize)]
struct LogAccepted {
    accepted: bool,
}

async fn append_log(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(run_id): Path<String>,
    Json(body): Json<AppendLogRequest>,
) -> Result<(StatusCode, Json<LogAccepted>), StatusCode> {
    let runner_id = auth.runner_id.as_deref().ok_or(StatusCode::FORBIDDEN)?;
    if body.line.len() > 16_384 || !matches!(body.stream.as_str(), "stdout" | "stderr" | "system") {
        return Err(StatusCode::BAD_REQUEST);
    }
    let accepted = executions::append_run_log(
        state.storage.pool(),
        &auth.organization_id,
        runner_id,
        &run_id,
        &body.stream,
        &body.line,
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if !accepted {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok((StatusCode::CREATED, Json(LogAccepted { accepted })))
}
