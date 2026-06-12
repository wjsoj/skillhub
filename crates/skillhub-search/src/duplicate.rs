//! Semantic duplicate detection for skills.
//!
//! Pipeline:
//!   1. Normalise the candidate's metadata via `SkillContent`.
//!   2. Compute its embedding with the configured `Embedder`.
//!   3. Hand the vector to `EmbeddingRepository::similar` (pgvector
//!      cosine top-K).
//!   4. Optionally widen with trigram-based name/desc recall, then
//!      merge and de-dupe.
//!   5. Filter by visibility / department scope via the policy.
//!   6. Classify each hit into a confidence band so the UI can decide
//!      whether to soft-warn, hard-block, or auto-link.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use skillhub_auth::{Action, PermissionCtx, PolicyEvaluator, Target};
use skillhub_domain::embedding::{EmbeddingRepository, SimilarityHit};
use skillhub_domain::DomainResult;
use skillhub_embeddings::{Embedder, SkillContent};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Confidence {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateCandidate {
    pub hit: SimilarityHit,
    pub confidence: Confidence,
    pub suggested_action: SuggestedAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SuggestedAction {
    /// Likely the same thing — recommend opening a proposal on the
    /// existing skill instead of creating a new one.
    UseExisting,
    /// Worth a human look before publishing.
    Review,
    /// Probably distinct, surfaced for awareness only.
    Inform,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateReport {
    pub query_hash: String,
    pub model: String,
    pub candidates: Vec<DuplicateCandidate>,
}

/// Cosine thresholds. Tuned empirically — make them config in real life.
const T_HIGH: f32 = 0.88;
const T_MED: f32 = 0.78;

fn classify(score: f32) -> (Confidence, SuggestedAction) {
    if score >= T_HIGH {
        (Confidence::High, SuggestedAction::UseExisting)
    } else if score >= T_MED {
        (Confidence::Medium, SuggestedAction::Review)
    } else {
        (Confidence::Low, SuggestedAction::Inform)
    }
}

pub struct DuplicateDetector {
    embedder: Arc<dyn Embedder>,
    repo: Arc<dyn EmbeddingRepository>,
    policy: PolicyEvaluator,
    limit: i64,
}

impl DuplicateDetector {
    pub fn new(
        embedder: Arc<dyn Embedder>,
        repo: Arc<dyn EmbeddingRepository>,
        limit: i64,
    ) -> Self {
        Self {
            embedder,
            repo,
            policy: PolicyEvaluator::new(),
            limit,
        }
    }

    /// Run detection. `exclude_skill` skips the skill itself when called
    /// during an update flow. `ctx` is used to filter results the caller
    /// would not be allowed to see anyway — preventing information leak
    /// across departments.
    pub async fn check(
        &self,
        content: &SkillContent<'_>,
        exclude_skill: Option<Uuid>,
        ctx: &PermissionCtx,
    ) -> DomainResult<DuplicateReport> {
        let text = content.to_embedding_input();
        let emb = self
            .embedder
            .embed(&text)
            .await
            .map_err(|e| skillhub_domain::DomainError::Internal(e.to_string()))?;

        let hits = self
            .repo
            .similar(&emb.vector, exclude_skill, None, self.limit)
            .await?;

        let candidates = hits
            .into_iter()
            .filter(|h| self.visible(h, ctx))
            .map(|h| {
                let (confidence, suggested_action) = classify(h.score);
                DuplicateCandidate {
                    hit: h,
                    confidence,
                    suggested_action,
                }
            })
            .collect();

        Ok(DuplicateReport {
            query_hash: emb.content_hash,
            model: emb.model,
            candidates,
        })
    }

    fn visible(&self, h: &SimilarityHit, ctx: &PermissionCtx) -> bool {
        let target = Target::skill(h.skill_id, h.namespace_id, h.department_id, h.visibility);
        self.policy
            .evaluate(ctx, Action::ReadSkill, &target)
            .is_allow()
    }
}
