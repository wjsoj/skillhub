//! /api/v1/versions — recent published versions across all skills.

use std::sync::Arc;

use axum::{
    extract::{Query, State},
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use crate::error::ApiError;
use crate::state::AppState;
use skillhub_domain::DomainError;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/", get(recent_versions))
}

#[derive(Debug, Deserialize)]
struct Params {
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct RecentVersion {
    id: Uuid,
    skill_id: Uuid,
    skill_slug: String,
    namespace_slug: String,
    version: String,
    status: String,
    published_by: Uuid,
    published_at: DateTime<Utc>,
}

async fn recent_versions(
    State(state): State<Arc<AppState>>,
    Query(params): Query<Params>,
) -> Result<Json<Vec<RecentVersion>>, ApiError> {
    let limit = params.limit.unwrap_or(50).clamp(1, 200);
    let rows = sqlx::query(
        r#"SELECT v.id, v.skill_id, s.slug AS skill_slug, n.slug AS namespace_slug,
                  v.version, v.status, v.published_by, v.published_at
           FROM skill_versions v
           JOIN skills s ON s.id = v.skill_id
           JOIN namespaces n ON n.id = s.namespace_id
           ORDER BY v.published_at DESC
           LIMIT $1"#,
    )
    .bind(limit)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| DomainError::Internal(e.to_string()))?;
    Ok(Json(
        rows.iter()
            .map(|r| RecentVersion {
                id: r.get("id"),
                skill_id: r.get("skill_id"),
                skill_slug: r.get("skill_slug"),
                namespace_slug: r.get("namespace_slug"),
                version: r.get("version"),
                status: r.get("status"),
                published_by: r.get("published_by"),
                published_at: r.get("published_at"),
            })
            .collect(),
    ))
}
