//! /api/v1/reviews — a cross-skill review queue.
//!
//! Surfaces version proposals that still need a decision (open or
//! changes-requested), newest first, with enough skill context to render
//! a reviewer's worklist without N+1 lookups.

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
    Router::new().route("/", get(review_queue))
}

#[derive(Debug, Deserialize)]
struct Params {
    /// "open" (default) returns actionable proposals; "all" returns every state.
    state: Option<String>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct ReviewItem {
    proposal_id: Uuid,
    skill_id: Uuid,
    skill_slug: String,
    namespace_slug: String,
    state: String,
    title: String,
    opened_by: Uuid,
    opened_at: DateTime<Utc>,
}

async fn review_queue(
    State(state): State<Arc<AppState>>,
    Query(params): Query<Params>,
) -> Result<Json<Vec<ReviewItem>>, ApiError> {
    let limit = params.limit.unwrap_or(50).clamp(1, 200);
    let actionable = params.state.as_deref() != Some("all");

    let sql = format!(
        r#"SELECT p.id, p.skill_id, s.slug AS skill_slug, n.slug AS namespace_slug,
                  p.state, p.title, p.opened_by, p.opened_at
           FROM version_proposals p
           JOIN skills s ON s.id = p.skill_id
           JOIN namespaces n ON n.id = s.namespace_id
           {}
           ORDER BY p.opened_at DESC
           LIMIT $1"#,
        if actionable {
            "WHERE p.state IN ('open','changes_requested')"
        } else {
            ""
        }
    );

    let rows = sqlx::query(&sql)
        .bind(limit)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

    Ok(Json(
        rows.iter()
            .map(|r| ReviewItem {
                proposal_id: r.get("id"),
                skill_id: r.get("skill_id"),
                skill_slug: r.get("skill_slug"),
                namespace_slug: r.get("namespace_slug"),
                state: r.get("state"),
                title: r.get("title"),
                opened_by: r.get("opened_by"),
                opened_at: r.get("opened_at"),
            })
            .collect(),
    ))
}
