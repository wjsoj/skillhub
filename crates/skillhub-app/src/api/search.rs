//! /api/v1/search — Postgres full-text search over skills.
//!
//! Ranks `plainto_tsquery` matches against the weighted `search_vector`
//! (display_name > slug > description) with `ts_rank_cd`, falling back to
//! a plain install- count listing when no query is given.

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

use crate::api::authz;
use crate::error::ApiError;
use crate::middleware::AuthPrincipal;
use crate::state::AppState;
use skillhub_domain::DomainError;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/", get(search))
}

#[derive(Debug, Deserialize)]
struct SearchParams {
    q: Option<String>,
    namespace: Option<String>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct SearchHit {
    id: Uuid,
    namespace_id: Uuid,
    namespace_slug: String,
    department_id: Option<Uuid>,
    slug: String,
    display_name: String,
    description: Option<String>,
    visibility: String,
    manifest: serde_json::Value,
    readme: Option<String>,
    install_command: Option<String>,
    repository_url: Option<String>,
    tags: Vec<String>,
    downloads: i64,
    install_count: i64,
    stars: i64,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    score: f32,
}

fn row_to_hit(r: &sqlx::postgres::PgRow, score: f32) -> SearchHit {
    SearchHit {
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
        score,
    }
}

const COLS: &str = r#"
    s.id, s.namespace_id, n.slug AS namespace_slug, n.department_id,
    s.slug, s.display_name, s.description, s.visibility,
    s.manifest, s.readme, s.install_command, s.repository_url,
    s.tags, s.downloads, s.install_count, s.stars,
    s.created_at, s.updated_at
"#;

async fn search(
    State(state): State<Arc<AppState>>,
    AuthPrincipal(principal): AuthPrincipal,
    Query(params): Query<SearchParams>,
) -> Result<Json<Vec<SearchHit>>, ApiError> {
    let uid = principal.user_id.ok_or(DomainError::Unauthorized)?;
    let is_super = authz::is_super(&principal);
    let limit = params.limit.unwrap_or(50).clamp(1, 200);
    let q = params.q.unwrap_or_default();
    let q = q.trim();
    let ns = params.namespace.as_deref().map(|s| s.trim()).filter(|s| !s.is_empty());

    // No query → plain listing (visibility-filtered, optionally namespace-filtered).
    if q.is_empty() {
        let sql = format!(
            r#"SELECT {COLS}
               FROM skills s JOIN namespaces n ON n.id = s.namespace_id
               WHERE ($1::text IS NULL OR n.slug = $1)
                 AND {vis}
               ORDER BY s.install_count DESC, s.display_name ASC
               LIMIT $2"#,
            vis = authz::vis_predicate(3, 4)
        );
        let rows = sqlx::query(&sql)
            .bind(ns)
            .bind(limit)
            .bind(is_super)
            .bind(uid)
            .fetch_all(&state.pool)
            .await
            .map_err(|e| DomainError::Internal(e.to_string()))?;
        return Ok(Json(rows.iter().map(|r| row_to_hit(r, 0.0)).collect()));
    }

    // FTS ranked query. `$1` (the tsquery text) is referenced twice — the
    // same bound parameter, which Postgres allows.
    let sql = format!(
        r#"SELECT {COLS},
                  ts_rank_cd(s.search_vector, plainto_tsquery('simple', $1)) AS rank
           FROM skills s
           JOIN namespaces n ON n.id = s.namespace_id
           WHERE s.search_vector @@ plainto_tsquery('simple', $1)
             AND ($2::text IS NULL OR n.slug = $2)
             AND {vis}
           ORDER BY rank DESC, s.install_count DESC
           LIMIT $3"#,
        vis = authz::vis_predicate(4, 5)
    );
    let rows = sqlx::query(&sql)
        .bind(q)
        .bind(ns)
        .bind(limit)
        .bind(is_super)
        .bind(uid)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

    let hits = rows
        .iter()
        .map(|r| {
            let rank: f32 = r.try_get("rank").unwrap_or(0.0);
            row_to_hit(r, rank)
        })
        .collect();
    Ok(Json(hits))
}
