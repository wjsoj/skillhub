//! /cli/* — compatibility surface for the `clawhub` CLI.
//!
//! `clawhub install <namespace>/<slug>` resolves a skill to its latest
//! approved version, bumps the install/download counters, and returns the
//! manifest + download key the client needs.

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;
use sqlx::Row;
use uuid::Uuid;

use crate::error::ApiError;
use crate::state::AppState;
use skillhub_domain::DomainError;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/resolve/:namespace/:slug", get(resolve))
        .route("/install/:namespace/:slug", post(install))
}

#[derive(Debug, Serialize)]
struct ResolveResult {
    skill_id: Uuid,
    namespace: String,
    slug: String,
    display_name: String,
    latest_version: Option<String>,
    storage_key: Option<String>,
    checksum_sha256: Option<String>,
    manifest: serde_json::Value,
    install_command: Option<String>,
    install_count: i64,
}

async fn lookup(
    state: &AppState,
    namespace: &str,
    slug: &str,
) -> Result<ResolveResult, ApiError> {
    let row = sqlx::query(
        r#"SELECT s.id, s.display_name, s.manifest, s.install_command, s.install_count
           FROM skills s JOIN namespaces n ON n.id = s.namespace_id
           WHERE n.slug = $1 AND s.slug = $2"#,
    )
    .bind(namespace)
    .bind(slug)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| DomainError::Internal(e.to_string()))?
    .ok_or_else(|| DomainError::NotFound(format!("{namespace}/{slug}")))?;

    let skill_id: Uuid = row.get("id");

    // Latest approved version, if any.
    let ver = sqlx::query(
        r#"SELECT version, storage_key, checksum_sha256
           FROM skill_versions
           WHERE skill_id = $1 AND status = 'approved'
           ORDER BY published_at DESC LIMIT 1"#,
    )
    .bind(skill_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| DomainError::Internal(e.to_string()))?;

    Ok(ResolveResult {
        skill_id,
        namespace: namespace.to_string(),
        slug: slug.to_string(),
        display_name: row.get("display_name"),
        latest_version: ver.as_ref().map(|r| r.get("version")),
        storage_key: ver.as_ref().map(|r| r.get("storage_key")),
        checksum_sha256: ver.as_ref().map(|r| r.get("checksum_sha256")),
        manifest: row.get("manifest"),
        install_command: row.get("install_command"),
        install_count: row.get("install_count"),
    })
}

async fn resolve(
    State(state): State<Arc<AppState>>,
    Path((namespace, slug)): Path<(String, String)>,
) -> Result<Json<ResolveResult>, ApiError> {
    Ok(Json(lookup(&state, &namespace, &slug).await?))
}

async fn install(
    State(state): State<Arc<AppState>>,
    Path((namespace, slug)): Path<(String, String)>,
) -> Result<Json<ResolveResult>, ApiError> {
    let mut result = lookup(&state, &namespace, &slug).await?;
    // Count the install.
    let n: i64 = sqlx::query_scalar(
        "UPDATE skills SET install_count = install_count + 1, downloads = downloads + 1
         WHERE id = $1 RETURNING install_count",
    )
    .bind(result.skill_id)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| DomainError::Internal(e.to_string()))?;
    result.install_count = n;
    Ok(Json(result))
}
