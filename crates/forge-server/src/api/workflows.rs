use axum::{extract::State, http::StatusCode, routing::post, Extension, Json, Router};
use forge_workflow::{WorkflowCompiler, WorkflowParser};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Deserialize)]
pub struct ValidateWorkflowRequest {
    yaml: String,
}

#[derive(Debug, Serialize)]
pub struct ValidateWorkflowResponse {
    valid: bool,
    job_order: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

pub fn public_routes() -> Router<crate::state::AppState> {
    Router::new().route("/api/v1/workflows/validate", post(validate_workflow))
}

pub fn protected_routes() -> Router<crate::state::AppState> {
    Router::new()
        .route("/api/v1/workflows/run", post(start_workflow_run))
        .route("/api/v1/workflows", post(save_workflow))
}

#[derive(Debug, Deserialize)]
struct SaveWorkflowRequest {
    project_id: String,
    yaml: String,
}

#[derive(Debug, Serialize)]
struct SaveWorkflowResponse {
    workflow_id: String,
    name: String,
    version: i64,
}

async fn save_workflow(
    State(state): State<crate::state::AppState>,
    Extension(auth): Extension<crate::auth::AuthContext>,
    Json(body): Json<SaveWorkflowRequest>,
) -> Result<(StatusCode, Json<SaveWorkflowResponse>), StatusCode> {
    if !crate::auth::require_permission(auth.role, forge_storage::Permission::ManageProjects) {
        return Err(StatusCode::FORBIDDEN);
    }
    let document = WorkflowParser::parse_str(&body.yaml).map_err(|_| StatusCode::BAD_REQUEST)?;
    WorkflowCompiler::compile(document.clone()).map_err(|_| StatusCode::BAD_REQUEST)?;
    let belongs = forge_storage::repos::projects::belongs_to_org(
        state.storage.pool(),
        &auth.organization_id,
        &body.project_id,
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if !belongs {
        return Err(StatusCode::NOT_FOUND);
    }
    let workflow_id = format!("wf_{}", unique_id());
    let row: (String, i32) = sqlx::query_as(
        "INSERT INTO workflow_definitions (workflow_id, organization_id, project_id, name, yaml, version, updated_at)
         VALUES (?, ?, ?, ?, ?, 1, ?)
         ON CONFLICT (organization_id, project_id, name) DO UPDATE SET yaml = excluded.yaml,
           version = workflow_definitions.version + 1, updated_at = excluded.updated_at
         RETURNING workflow_id, version",
    )
    .bind(&workflow_id)
    .bind(&auth.organization_id)
    .bind(&body.project_id)
    .bind(&document.name)
    .bind(&body.yaml)
    .bind(SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0))
    .fetch_one(state.storage.pool()).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((
        StatusCode::OK,
        Json(SaveWorkflowResponse {
            workflow_id: row.0,
            name: document.name,
            version: i64::from(row.1),
        }),
    ))
}

async fn validate_workflow(
    Json(body): Json<ValidateWorkflowRequest>,
) -> Json<ValidateWorkflowResponse> {
    match WorkflowParser::parse_str(&body.yaml) {
        Ok(document) => match WorkflowCompiler::compile(document) {
            Ok(compiled) => Json(ValidateWorkflowResponse {
                valid: true,
                job_order: compiled.graph.order,
                error: None,
            }),
            Err(e) => Json(ValidateWorkflowResponse {
                valid: false,
                job_order: vec![],
                error: Some(e.to_string()),
            }),
        },
        Err(e) => Json(ValidateWorkflowResponse {
            valid: false,
            job_order: vec![],
            error: Some(e.to_string()),
        }),
    }
}

#[derive(Debug, Deserialize)]
struct StartRunRequest {
    yaml: String,
    project_id: Option<String>,
    #[serde(default = "manual_trigger")]
    trigger: String,
}

#[derive(Debug, Serialize)]
struct StartRunResponse {
    run_id: String,
    workflow_name: String,
    status: &'static str,
    job_order: Vec<String>,
}

async fn start_workflow_run(
    State(state): State<crate::state::AppState>,
    Extension(auth): Extension<crate::auth::AuthContext>,
    Json(body): Json<StartRunRequest>,
) -> Result<(StatusCode, Json<StartRunResponse>), StatusCode> {
    let document = WorkflowParser::parse_str(&body.yaml).map_err(|_| StatusCode::BAD_REQUEST)?;
    let compiled = WorkflowCompiler::compile(document).map_err(|_| StatusCode::BAD_REQUEST)?;
    if let Some(project_id) = body.project_id.as_deref() {
        let belongs = forge_storage::repos::projects::belongs_to_org(
            state.storage.pool(),
            &auth.organization_id,
            project_id,
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        if !belongs {
            return Err(StatusCode::NOT_FOUND);
        }
    }

    let run_id = format!("run_{}", unique_id());
    forge_storage::repos::executions::create_run(
        state.storage.pool(),
        &auth.organization_id,
        &run_id,
        body.project_id.as_deref(),
        &compiled.document.name,
        &body.trigger,
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    for (index, job_id) in compiled.graph.order.iter().enumerate() {
        let command = compiled
            .document
            .jobs
            .get(job_id)
            .map(|job| {
                job.steps
                    .iter()
                    .filter_map(|step| step.run.as_deref())
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_default();
        forge_storage::repos::executions::create_job(
            state.storage.pool(),
            &auth.organization_id,
            &run_id,
            &format!("{run_id}_{index}"),
            job_id,
            &command,
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    let _ = forge_storage::repos::audit::write(
        state.storage.pool(),
        &auth.organization_id,
        Some(&auth.token_id),
        "workflow.run.start",
        &run_id,
        None,
    )
    .await;
    Ok((
        StatusCode::ACCEPTED,
        Json(StartRunResponse {
            run_id,
            workflow_name: compiled.document.name,
            status: "queued",
            job_order: compiled.graph.order,
        }),
    ))
}

fn unique_id() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0)
}

fn manual_trigger() -> String {
    "manual".into()
}
