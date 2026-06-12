//! Storage record for skill embeddings. The embedding *vector* itself
//! lives outside this crate (the value is owned by the search/infra
//! layer with the pgvector type).

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::DomainResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EmbeddingSource {
    Skill,
    Version,
    Draft,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingRecord {
    pub id: Uuid,
    pub skill_id: Uuid,
    pub version_id: Option<Uuid>,
    pub source: EmbeddingSource,
    pub model: String,
    pub dim: i32,
    pub content_hash: String,
    pub text_preview: Option<String>,
    pub updated_at: DateTime<Utc>,
}

/// Hit returned from a similarity query. Score is cosine in [-1, 1];
/// callers typically threshold at ~0.85 for "looks like a duplicate".
/// Carries enough metadata for the policy evaluator to filter without
/// re-hitting the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarityHit {
    pub skill_id: Uuid,
    pub namespace_id: Uuid,
    pub namespace_slug: String,
    pub department_id: Option<Uuid>,
    pub visibility: crate::skill::Visibility,
    pub slug: String,
    pub display_name: String,
    pub description: Option<String>,
    pub score: f32,
    pub matched_on: Vec<String>,
}

#[async_trait]
pub trait EmbeddingRepository: Send + Sync {
    /// Upsert by (skill_id, version_id, source, model). Stores both the
    /// metadata record and the actual vector — implementations pass the
    /// vector through as `Vec<f32>` and translate to pgvector internally.
    async fn upsert(
        &self,
        record: &EmbeddingRecord,
        vector: &[f32],
    ) -> DomainResult<()>;

    /// Cosine top-K. `exclude_skill` skips the calling skill (for "find
    /// duplicates of ME"). `visible_skill_ids = None` means "no filter"
    /// (caller has already enforced visibility upstream).
    async fn similar(
        &self,
        vector: &[f32],
        exclude_skill: Option<Uuid>,
        visible_skill_ids: Option<&[Uuid]>,
        limit: i64,
    ) -> DomainResult<Vec<SimilarityHit>>;
}
