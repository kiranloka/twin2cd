use crate::state::AppState;
use axum::{
    body::Bytes,
    extract::{DefaultBodyLimit, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Extension, Json, Router,
};
use forge_storage::repos::webhooks::{self, WebhookDeliveryRow};
use serde::Serialize;
use sha2::{Digest, Sha256};

pub fn public_routes() -> Router<AppState> {
    Router::new()
        .route("/hooks/github", post(receive_github_webhook))
        .layer(DefaultBodyLimit::max(1_048_576))
}

pub fn protected_routes() -> Router<AppState> {
    Router::new().route("/api/v1/webhooks/deliveries", get(list_deliveries))
}

#[derive(Serialize)]
struct DeliveryAccepted {
    accepted: bool,
    duplicate: bool,
}

async fn receive_github_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<(StatusCode, Json<DeliveryAccepted>), StatusCode> {
    let secret = std::env::var("FORGE_GITHUB_WEBHOOK_SECRET")
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let signature = headers
        .get("x-hub-signature-256")
        .and_then(|value| value.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;
    if !verify_signature(&secret, signature, &body) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let event = headers
        .get("x-github-event")
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.is_empty())
        .ok_or(StatusCode::BAD_REQUEST)?;
    let delivery_id = headers
        .get("x-github-delivery")
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.is_empty() && value.len() <= 128)
        .ok_or(StatusCode::BAD_REQUEST)?;
    let payload: serde_json::Value =
        serde_json::from_slice(&body).map_err(|_| StatusCode::BAD_REQUEST)?;
    if !payload.is_object() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let payload_json = serde_json::to_string(&payload).map_err(|_| StatusCode::BAD_REQUEST)?;
    let organization_id =
        std::env::var("FORGE_GITHUB_ORGANIZATION_ID").unwrap_or_else(|_| "org_default".into());
    let inserted = if event == "push" {
        let repository = payload
            .pointer("/repository/full_name")
            .and_then(|v| v.as_str())
            .ok_or(StatusCode::BAD_REQUEST)?;
        let workflows: Vec<(String, String)> = sqlx::query_as(
            "SELECT w.project_id, w.yaml FROM workflow_definitions w
             JOIN projects p ON p.id = w.project_id AND p.organization_id = w.organization_id
             WHERE w.organization_id = ? AND lower(p.repository) = lower(?)",
        )
        .bind(&organization_id)
        .bind(repository)
        .fetch_all(state.storage.pool())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let mut compiled_workflows = Vec::new();
        for (project_id, yaml) in workflows {
            let Ok(document) = forge_workflow::WorkflowParser::parse_str(&yaml) else {
                continue;
            };
            let Ok(compiled) = forge_workflow::WorkflowCompiler::compile(document) else {
                continue;
            };
            let jobs = compiled
                .graph
                .order
                .iter()
                .map(|job_id| {
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
                    (job_id.clone(), command)
                })
                .collect::<Vec<_>>();
            compiled_workflows.push((project_id, compiled.document.name, jobs));
        }
        let rows: Vec<_> = compiled_workflows
            .iter()
            .map(|(project_id, name, jobs)| webhooks::PushWorkflow {
                project_id,
                workflow_name: name,
                jobs,
            })
            .collect();
        webhooks::record_push_and_queue(
            state.storage.pool(),
            &organization_id,
            delivery_id,
            &payload_json,
            &rows,
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    } else {
        webhooks::record_delivery(
            state.storage.pool(),
            &organization_id,
            delivery_id,
            event,
            &payload_json,
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    };
    Ok((
        if inserted {
            StatusCode::ACCEPTED
        } else {
            StatusCode::OK
        },
        Json(DeliveryAccepted {
            accepted: true,
            duplicate: !inserted,
        }),
    ))
}

async fn list_deliveries(
    State(state): State<AppState>,
    Extension(auth): Extension<crate::auth::AuthContext>,
) -> Result<Json<Vec<WebhookDeliveryRow>>, StatusCode> {
    if !crate::auth::require_permission(auth.role, forge_storage::Permission::ReadOrganization) {
        return Err(StatusCode::FORBIDDEN);
    }
    let deliveries = webhooks::list_for_org(state.storage.pool(), &auth.organization_id, 50)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(deliveries))
}

fn verify_signature(secret: &str, signature: &str, body: &[u8]) -> bool {
    let Some(encoded) = signature.strip_prefix("sha256=") else {
        return false;
    };
    let Ok(provided) = hex::decode(encoded) else {
        return false;
    };
    if provided.len() != 32 {
        return false;
    }

    const BLOCK_SIZE: usize = 64;
    let mut key = [0_u8; BLOCK_SIZE];
    let secret = secret.as_bytes();
    if secret.len() > BLOCK_SIZE {
        key[..32].copy_from_slice(&Sha256::digest(secret));
    } else {
        key[..secret.len()].copy_from_slice(secret);
    }
    let mut inner_pad = [0x36_u8; BLOCK_SIZE];
    let mut outer_pad = [0x5c_u8; BLOCK_SIZE];
    for (index, byte) in key.iter().enumerate() {
        inner_pad[index] ^= byte;
        outer_pad[index] ^= byte;
    }
    let mut inner = Sha256::new();
    inner.update(inner_pad);
    inner.update(body);
    let mut outer = Sha256::new();
    outer.update(outer_pad);
    outer.update(inner.finalize());
    let expected = outer.finalize();

    expected
        .iter()
        .zip(provided.iter())
        .fold(0_u8, |difference, (left, right)| {
            difference | (left ^ right)
        })
        == 0
}
