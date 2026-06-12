//! /api/v1/namespaces — list, create, and look up namespaces.

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use crate::error::ApiError;
use crate::middleware::AuthPrincipal;
use crate::state::AppState;
use skillhub_domain::DomainError;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list_namespaces).post(create_namespace))
        .route("/:slug", get(get_namespace))
}

#[derive(Debug, Serialize)]
struct NamespaceDto {
    id: Uuid,
    slug: String,
    display_name: String,
    scope: String,
    department_id: Option<Uuid>,
    skill_count: i64,
    created_at: DateTime<Utc>,
}

fn row_to_dto(r: &sqlx::postgres::PgRow) -> NamespaceDto {
    NamespaceDto {
        id: r.get("id"),
        slug: r.get("slug"),
        display_name: r.get("display_name"),
        scope: r.get("scope"),
        department_id: r.get("department_id"),
        skill_count: r.get("skill_count"),
        created_at: r.get("created_at"),
    }
}

const SELECT_NS: &str = r#"
    SELECT n.id, n.slug, n.display_name, n.scope, n.department_id, n.created_at,
           COUNT(s.id) AS skill_count
    FROM namespaces n
    LEFT JOIN skills s ON s.namespace_id = n.id
"#;

async fn list_namespaces(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<NamespaceDto>>, ApiError> {
    let sql = format!("{SELECT_NS} GROUP BY n.id ORDER BY n.slug ASC");
    let rows = sqlx::query(&sql)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;
    Ok(Json(rows.iter().map(row_to_dto).collect()))
}

async fn get_namespace(
    State(state): State<Arc<AppState>>,
    Path(slug): Path<String>,
) -> Result<Json<NamespaceDto>, ApiError> {
    let sql = format!("{SELECT_NS} WHERE n.slug = $1 GROUP BY n.id");
    let row = sqlx::query(&sql)
        .bind(&slug)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?
        .ok_or_else(|| DomainError::NotFound(format!("namespace '{slug}'")))?;
    Ok(Json(row_to_dto(&row)))
}

#[derive(Debug, Deserialize)]
struct CreateBody {
    slug: String,
    display_name: String,
    #[serde(default = "default_scope")]
    scope: String,
    department_id: Option<Uuid>,
}

fn default_scope() -> String {
    "team".into()
}

async fn create_namespace(
    State(state): State<Arc<AppState>>,
    AuthPrincipal(principal): AuthPrincipal,
    Json(body): Json<CreateBody>,
) -> Result<Json<NamespaceDto>, ApiError> {
    let uid = principal.user_id.ok_or(DomainError::Unauthorized)?;
    let slug = body.slug.trim().to_lowercase();
    if slug.is_empty() {
        return Err(DomainError::Validation("slug is required".into()).into());
    }
    if body.scope != "team" && body.scope != "global" {
        return Err(DomainError::Validation("scope must be 'team' or 'global'".into()).into());
    }

    let dup = sqlx::query("SELECT 1 FROM namespaces WHERE slug = $1")
        .bind(&slug)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;
    if dup.is_some() {
        return Err(DomainError::AlreadyExists(format!("namespace '{slug}' exists")).into());
    }

    let ns_id: Uuid = sqlx::query_scalar(
        r#"INSERT INTO namespaces (slug, display_name, scope, department_id)
           VALUES ($1, $2, $3, $4) RETURNING id"#,
    )
    .bind(&slug)
    .bind(body.display_name.trim())
    .bind(&body.scope)
    .bind(body.department_id)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| DomainError::Internal(e.to_string()))?;

    // Creator becomes the namespace owner.
    sqlx::query(
        r#"INSERT INTO namespace_members (namespace_id, user_id, role)
           VALUES ($1, $2, 'owner') ON CONFLICT DO NOTHING"#,
    )
    .bind(ns_id)
    .bind(uid)
    .execute(&state.pool)
    .await
    .map_err(|e| DomainError::Internal(e.to_string()))?;

    let sql = format!("{SELECT_NS} WHERE n.id = $1 GROUP BY n.id");
    let row = sqlx::query(&sql)
        .bind(ns_id)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;
    Ok(Json(row_to_dto(&row)))
}
