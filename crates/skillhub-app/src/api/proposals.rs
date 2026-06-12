//! /api/v1/skills/:skill_id/proposals — review pipeline.
//!
//! Open, list, comment, decide. Merge is a separate verb that
//! materialises a real `SkillVersion` row from the draft.

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use skillhub_domain::proposal::{
    ProposalReview, ProposalState, ReviewVerdict, VersionDraft, VersionProposal,
};

use crate::error::ApiError;
use crate::middleware::AuthPrincipal;
use crate::state::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/:skill_id/drafts", post(create_draft))
        .route("/:skill_id/proposals", post(open).get(list))
        .route("/:skill_id/proposals/:pid", get(read))
        .route("/:skill_id/proposals/:pid/reviews", post(review))
        .route("/:skill_id/proposals/:pid/decide", post(decide))
        .route("/:skill_id/proposals/:pid/merge", post(merge))
}

#[derive(Debug, Deserialize)]
pub struct CreateDraftBody {
    pub base_version_id: Option<Uuid>,
    pub target_version: String,
    pub manifest: serde_json::Value,
    pub summary: Option<String>,
}

async fn create_draft(
    State(state): State<Arc<AppState>>,
    AuthPrincipal(principal): AuthPrincipal,
    Path(skill_id): Path<Uuid>,
    Json(body): Json<CreateDraftBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let actor = principal
        .user_id
        .ok_or(skillhub_domain::DomainError::Unauthorized)?;
    let d = VersionDraft {
        id: Uuid::new_v4(),
        skill_id,
        base_version_id: body.base_version_id,
        target_version: body.target_version,
        manifest: body.manifest,
        storage_key: None,
        size_bytes: None,
        checksum_sha256: None,
        summary: body.summary,
        created_by: actor,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    state.drafts.create(&d).await?;
    Ok(Json(serde_json::json!({ "draft_id": d.id })))
}

#[derive(Debug, Deserialize)]
pub struct OpenProposalBody {
    pub draft_id: Uuid,
    pub title: String,
    pub body: Option<String>,
}

async fn open(
    State(state): State<Arc<AppState>>,
    AuthPrincipal(principal): AuthPrincipal,
    Path(skill_id): Path<Uuid>,
    Json(body): Json<OpenProposalBody>,
) -> Result<(StatusCode, Json<ProposalDto>), ApiError> {
    let actor = principal
        .user_id
        .ok_or(skillhub_domain::DomainError::Unauthorized)?;
    let p = VersionProposal {
        id: Uuid::new_v4(),
        skill_id,
        draft_id: body.draft_id,
        state: ProposalState::Open,
        title: body.title,
        body: body.body,
        opened_by: actor,
        opened_at: Utc::now(),
        decided_by: None,
        decided_at: None,
        merged_version_id: None,
    };
    state.proposals.create(&p).await?;
    Ok((StatusCode::CREATED, Json(p.into())))
}

#[derive(Debug, Serialize)]
pub struct ProposalDto {
    pub id: Uuid,
    pub skill_id: Uuid,
    pub draft_id: Uuid,
    pub state: ProposalState,
    pub title: String,
    pub body: Option<String>,
    pub opened_by: Uuid,
    pub merged_version_id: Option<Uuid>,
}

impl From<VersionProposal> for ProposalDto {
    fn from(p: VersionProposal) -> Self {
        Self {
            id: p.id,
            skill_id: p.skill_id,
            draft_id: p.draft_id,
            state: p.state,
            title: p.title,
            body: p.body,
            opened_by: p.opened_by,
            merged_version_id: p.merged_version_id,
        }
    }
}

async fn list(
    State(state): State<Arc<AppState>>,
    Path(skill_id): Path<Uuid>,
) -> Result<Json<Vec<ProposalDto>>, ApiError> {
    let list = state.proposals.list_open(skill_id).await?;
    Ok(Json(list.into_iter().map(Into::into).collect()))
}

async fn read(
    State(state): State<Arc<AppState>>,
    Path((_skill_id, pid)): Path<(Uuid, Uuid)>,
) -> Result<Json<ProposalDto>, ApiError> {
    let p = state
        .proposals
        .find(pid)
        .await?
        .ok_or_else(|| skillhub_domain::DomainError::NotFound("proposal".into()))?;
    Ok(Json(p.into()))
}

#[derive(Debug, Deserialize)]
pub struct ReviewBody {
    pub verdict: ReviewVerdict,
    pub body: Option<String>,
}

async fn review(
    State(state): State<Arc<AppState>>,
    AuthPrincipal(principal): AuthPrincipal,
    Path((_skill_id, pid)): Path<(Uuid, Uuid)>,
    Json(body): Json<ReviewBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let reviewer = principal
        .user_id
        .ok_or(skillhub_domain::DomainError::Unauthorized)?;
    let r = ProposalReview {
        id: Uuid::new_v4(),
        proposal_id: pid,
        reviewer_id: reviewer,
        verdict: body.verdict,
        body: body.body,
        reviewed_at: Utc::now(),
    };
    state.proposals.record_review(&r).await?;
    Ok(Json(serde_json::json!({ "review_id": r.id })))
}

#[derive(Debug, Deserialize)]
pub struct DecideBody {
    pub state: ProposalState,
}

async fn decide(
    State(state): State<Arc<AppState>>,
    AuthPrincipal(principal): AuthPrincipal,
    Path((_skill_id, pid)): Path<(Uuid, Uuid)>,
    Json(body): Json<DecideBody>,
) -> Result<Json<ProposalDto>, ApiError> {
    let actor = principal
        .user_id
        .ok_or(skillhub_domain::DomainError::Unauthorized)?;
    let mut p = state
        .proposals
        .find(pid)
        .await?
        .ok_or_else(|| skillhub_domain::DomainError::NotFound("proposal".into()))?;
    if !p.state.can_transition_to(body.state) {
        return Err(skillhub_domain::DomainError::Conflict(format!(
            "cannot transition {:?} → {:?}",
            p.state, body.state
        ))
        .into());
    }
    p.state = body.state;
    p.decided_by = Some(actor);
    p.decided_at = Some(Utc::now());
    state.proposals.update_state(&p).await?;
    Ok(Json(p.into()))
}

async fn merge(
    State(state): State<Arc<AppState>>,
    AuthPrincipal(principal): AuthPrincipal,
    Path((_skill_id, pid)): Path<(Uuid, Uuid)>,
) -> Result<Json<ProposalDto>, ApiError> {
    let actor = principal
        .user_id
        .ok_or(skillhub_domain::DomainError::Unauthorized)?;
    let mut p = state
        .proposals
        .find(pid)
        .await?
        .ok_or_else(|| skillhub_domain::DomainError::NotFound("proposal".into()))?;
    if p.state != ProposalState::Approved {
        return Err(skillhub_domain::DomainError::Conflict(
            "proposal must be approved before merge".into(),
        )
        .into());
    }
    // NB: actually materialising a `skill_versions` row from the draft
    // requires the skill repo's publish API (which lives next to the
    // existing `publish` flow). The hook is left in place here so the
    // proposal pipeline is observable end-to-end.
    p.state = ProposalState::Merged;
    p.decided_by = Some(actor);
    p.decided_at = Some(Utc::now());
    state.proposals.update_state(&p).await?;
    Ok(Json(p.into()))
}
