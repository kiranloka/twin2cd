use crate::{
    auth::{require_permission, AuthContext},
    state::AppState,
};
use axum::{extract::State, http::StatusCode, routing::get, Extension, Json, Router};
use forge_storage::{
    repos::projects::{self, ProjectRow},
    Permission,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct CreateProjectRequest {
    name: String,
    slug: String,
    repository: String,
    #[serde(default = "default_branch")]
    default_branch: String,
}

pub fn routes() -> Router<AppState> {
    Router::new().route("/api/v1/projects", get(list_projects).post(create_project))
}

async fn list_projects(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<Vec<ProjectRow>>, StatusCode> {
    if !require_permission(auth.role, Permission::ReadOrganization) {
        return Err(StatusCode::FORBIDDEN);
    }
    let rows = projects::list_for_org(state.storage.pool(), &auth.organization_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(rows))
}

async fn create_project(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(body): Json<CreateProjectRequest>,
) -> Result<(StatusCode, Json<ProjectRow>), StatusCode> {
    if !require_permission(auth.role, Permission::ManageProjects) {
        return Err(StatusCode::FORBIDDEN);
    }
    let valid_slug = !body.slug.is_empty()
        && body.slug.len() <= 64
        && body
            .slug
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if !valid_slug
        || body.name.trim().is_empty()
        || body.repository.trim().is_empty()
        || body.default_branch.trim().is_empty()
    {
        return Err(StatusCode::BAD_REQUEST);
    }
    let project = projects::create(
        state.storage.pool(),
        &auth.organization_id,
        body.name.trim(),
        &body.slug,
        body.repository.trim(),
        body.default_branch.trim(),
    )
    .await
    .map_err(|error| {
        if error.to_string().contains("UNIQUE") || error.to_string().contains("unique") {
            StatusCode::CONFLICT
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        }
    })?;
    let _ = forge_storage::repos::audit::write(
        state.storage.pool(),
        &auth.organization_id,
        Some(&auth.token_id),
        "project.create",
        &project.id,
        None,
    )
    .await;
    Ok((StatusCode::CREATED, Json(project)))
}

fn default_branch() -> String {
    "main".into()
}
