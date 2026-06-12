//! /api/v1/admin/* — privileged maintenance endpoints.

use std::sync::Arc;

use axum::{extract::State, routing::post, Json, Router};
use chrono::Utc;
use serde::Serialize;
use sqlx::Row;
use uuid::Uuid;

use skillhub_domain::embedding::{EmbeddingRecord, EmbeddingSource};
use skillhub_embeddings::SkillContent;

use crate::error::ApiError;
use crate::state::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/reindex-embeddings", post(reindex_embeddings))
}

#[derive(Debug, Serialize)]
struct ReindexResult {
    indexed: usize,
    model: String,
    skipped: usize,
}

/// Re-embed every skill in the database. Used by the seed script
/// after bulk-inserting fixture rows — and by operators after
/// switching `Embedder` providers.
async fn reindex_embeddings(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ReindexResult>, ApiError> {
    let rows = sqlx::query(
        "SELECT s.id, s.slug, s.display_name, s.description FROM skills s",
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|e| skillhub_domain::DomainError::Internal(e.to_string()))?;

    let mut indexed = 0usize;
    let mut skipped = 0usize;
    let model = state.embedder.model().to_string();
    let dim = state.embedder.dim();

    for r in rows {
        let id: Uuid = r.get("id");
        let slug: String = r.get("slug");
        let name: String = r.get("display_name");
        let desc: Option<String> = r.get("description");

        let tags: Vec<String> = Vec::new();
        let content = SkillContent {
            display_name: &name,
            slug: &slug,
            description: desc.as_deref(),
            readme: None,
            manifest: None,
            tags: &tags,
        };
        let text = content.to_embedding_input();
        let emb = match state.embedder.embed(&text).await {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!(skill = %id, error = %e, "embed failed");
                skipped += 1;
                continue;
            }
        };
        let record = EmbeddingRecord {
            id: Uuid::new_v4(),
            skill_id: id,
            version_id: None,
            source: EmbeddingSource::Skill,
            model: model.clone(),
            dim: dim as i32,
            content_hash: emb.content_hash.clone(),
            text_preview: Some(text.chars().take(200).collect()),
            updated_at: Utc::now(),
        };
        state
            .embeddings
            .upsert(&record, &emb.vector)
            .await
            .map_err(|e| skillhub_domain::DomainError::Internal(e.to_string()))?;
        indexed += 1;
    }

    Ok(Json(ReindexResult { indexed, model, skipped }))
}
