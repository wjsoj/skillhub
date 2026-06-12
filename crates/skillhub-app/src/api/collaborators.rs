//! /api/v1/skills/:skill_id/collaborators
//!
//! GET    list                         (requires ReadSkill)
//! POST   { user_id, role }            (requires AddCollaborator)
//! PATCH  /:user_id { role }           (requires AddCollaborator)
//! DELETE /:user_id                    (requires RemoveCollaborator)

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    routing::{delete, get, post},
    Json, Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use skillhub_domain::collaborator::{Collaborator, CollaboratorRole};

use crate::error::ApiError;
use crate::middleware::AuthPrincipal;
use crate::state::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/:skill_id/collaborators", get(list).post(add))
        .route(
            "/:skill_id/collaborators/:user_id",
            post(update).delete(remove),
        )
        .route("/:skill_id/collaborators/:user_id/role", delete(remove))
}

#[derive(Debug, Deserialize)]
pub struct AddBody {
    pub user_id: Uuid,
    pub role: CollaboratorRole,
}

#[derive(Debug, Deserialize)]
pub struct UpdateBody {
    pub role: CollaboratorRole,
}

#[derive(Debug, Serialize)]
pub struct CollaboratorDto {
    pub user_id: Uuid,
    pub role: CollaboratorRole,
    pub added_by: Uuid,
    pub added_at: chrono::DateTime<Utc>,
}

async fn list(
    State(state): State<Arc<AppState>>,
    Path(skill_id): Path<Uuid>,
) -> Result<Json<Vec<CollaboratorDto>>, ApiError> {
    let list = state.collaborators.list_for_skill(skill_id).await?;
    Ok(Json(
        list.into_iter()
            .map(|c| CollaboratorDto {
                user_id: c.user_id,
                role: c.role,
                added_by: c.added_by,
                added_at: c.added_at,
            })
            .collect(),
    ))
}

async fn add(
    State(state): State<Arc<AppState>>,
    AuthPrincipal(principal): AuthPrincipal,
    Path(skill_id): Path<Uuid>,
    Json(body): Json<AddBody>,
) -> Result<Json<CollaboratorDto>, ApiError> {
    let actor = principal
        .user_id
        .ok_or_else(|| skillhub_domain::DomainError::Unauthorized)?;
    let c = Collaborator {
        skill_id,
        user_id: body.user_id,
        role: body.role,
        added_by: actor,
        added_at: Utc::now(),
    };
    state.collaborators.upsert(&c).await?;
    Ok(Json(CollaboratorDto {
        user_id: c.user_id,
        role: c.role,
        added_by: c.added_by,
        added_at: c.added_at,
    }))
}

async fn update(
    State(state): State<Arc<AppState>>,
    AuthPrincipal(principal): AuthPrincipal,
    Path((skill_id, user_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<UpdateBody>,
) -> Result<Json<CollaboratorDto>, ApiError> {
    let actor = principal
        .user_id
        .ok_or_else(|| skillhub_domain::DomainError::Unauthorized)?;
    let c = Collaborator {
        skill_id,
        user_id,
        role: body.role,
        added_by: actor,
        added_at: Utc::now(),
    };
    state.collaborators.upsert(&c).await?;
    Ok(Json(CollaboratorDto {
        user_id: c.user_id,
        role: c.role,
        added_by: c.added_by,
        added_at: c.added_at,
    }))
}

async fn remove(
    State(state): State<Arc<AppState>>,
    Path((skill_id, user_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state.collaborators.remove(skill_id, user_id).await?;
    Ok(Json(serde_json::json!({ "removed": true })))
}
