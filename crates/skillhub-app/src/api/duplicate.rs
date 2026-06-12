//! POST /api/v1/skills/check-duplicate
//!
//! Body: { display_name, slug, description?, readme?, manifest?, tags? }
//! Returns: { query_hash, model, candidates: [...] }
//!
//! Used by clients (UI / CLI / AI) *before* publishing, to surface
//! semantically similar skills. The detector enforces visibility
//! through the principal's `PermissionCtx`, so users never see hits
//! they wouldn't otherwise be allowed to read.

use std::sync::Arc;

use axum::{extract::State, routing::post, Json, Router};
use serde::Deserialize;

use skillhub_embeddings::SkillContent;
use skillhub_search::DuplicateReport;

use crate::error::ApiError;
use crate::middleware::AuthPrincipal;
use crate::middleware::DeptScope;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct CheckBody {
    pub display_name: String,
    pub slug: String,
    pub description: Option<String>,
    pub readme: Option<String>,
    pub manifest: Option<serde_json::Value>,
    #[serde(default)]
    pub tags: Vec<String>,
    /// Skill ID to exclude from results (when re-checking an existing skill).
    pub exclude_skill_id: Option<uuid::Uuid>,
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/check-duplicate", post(check))
}

async fn check(
    State(state): State<Arc<AppState>>,
    AuthPrincipal(principal): AuthPrincipal,
    Json(body): Json<CheckBody>,
) -> Result<Json<DuplicateReport>, ApiError> {
    let ctx = DeptScope::hydrate(
        &principal,
        state.departments.clone(),
        state.department_memberships.clone(),
        state.cross_grants.clone(),
    )
    .await?;

    let content = SkillContent {
        display_name: &body.display_name,
        slug: &body.slug,
        description: body.description.as_deref(),
        readme: body.readme.as_deref(),
        manifest: body.manifest.as_ref(),
        tags: &body.tags,
    };

    // The repository hydrates each hit with namespace_id /
    // department_id / visibility, so the detector can directly evaluate
    // `ReadSkill` on every candidate against the calling principal.
    let report = state
        .duplicate_detector
        .check(&content, body.exclude_skill_id, &ctx)
        .await?;
    Ok(Json(report))
}
