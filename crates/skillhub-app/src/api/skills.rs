//! /api/v1/skills — read endpoints. Write paths still live in their
//! own modules (drafts/proposals, iterations, collaborators).

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::Row;
use uuid::Uuid;

use crate::error::ApiError;
use crate::state::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list_all))
        .route("/:id", get(get_one))
}

#[derive(Debug, Serialize)]
pub struct SkillDto {
    pub id: Uuid,
    pub namespace_id: Uuid,
    pub namespace_slug: String,
    pub department_id: Option<Uuid>,
    pub slug: String,
    pub display_name: String,
    pub description: Option<String>,
    pub visibility: String,
    pub manifest: serde_json::Value,
    pub readme: Option<String>,
    pub install_command: Option<String>,
    pub repository_url: Option<String>,
    pub tags: Vec<String>,
    pub downloads: i64,
    pub install_count: i64,
    pub stars: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

async fn list_all(State(state): State<Arc<AppState>>) -> Result<Json<Vec<SkillDto>>, ApiError> {
    let rows = sqlx::query(
        r#"
        SELECT s.id, s.namespace_id, n.slug AS namespace_slug, n.department_id,
               s.slug, s.display_name, s.description, s.visibility,
               s.manifest, s.readme, s.install_command, s.repository_url,
               s.tags, s.downloads, s.install_count, s.stars,
               s.created_at, s.updated_at
        FROM skills s
        JOIN namespaces n ON n.id = s.namespace_id
        ORDER BY s.install_count DESC, s.display_name ASC
        "#,
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|e| skillhub_domain::DomainError::Internal(e.to_string()))?;

    Ok(Json(rows.into_iter().map(row_to_dto).collect()))
}

async fn get_one(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<SkillDto>, ApiError> {
    let row = sqlx::query(
        r#"
        SELECT s.id, s.namespace_id, n.slug AS namespace_slug, n.department_id,
               s.slug, s.display_name, s.description, s.visibility,
               s.manifest, s.readme, s.install_command, s.repository_url,
               s.tags, s.downloads, s.install_count, s.stars,
               s.created_at, s.updated_at
        FROM skills s
        JOIN namespaces n ON n.id = s.namespace_id
        WHERE s.id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| skillhub_domain::DomainError::Internal(e.to_string()))?
    .ok_or_else(|| skillhub_domain::DomainError::NotFound(format!("skill {id}")))?;
    Ok(Json(row_to_dto(row)))
}

fn row_to_dto(r: sqlx::postgres::PgRow) -> SkillDto {
    SkillDto {
        id: r.get("id"),
        namespace_id: r.get("namespace_id"),
        namespace_slug: r.get("namespace_slug"),
        department_id: r.get("department_id"),
        slug: r.get("slug"),
        display_name: r.get("display_name"),
        description: r.get("description"),
        visibility: r.get("visibility"),
        manifest: r.get("manifest"),
        readme: r.get("readme"),
        install_command: r.get("install_command"),
        repository_url: r.get("repository_url"),
        tags: r.get("tags"),
        downloads: r.get("downloads"),
        install_count: r.get("install_count"),
        stars: r.get("stars"),
        created_at: r.get("created_at"),
        updated_at: r.get("updated_at"),
    }
}
