//! /api/v1/skills/:skill_id/iterations  — AI-friendly iteration harness.
//!
//! Designed so an agent with an `iteration:write` scoped token can:
//!   POST   /                    — open a job
//!   GET    /:job_id             — read state
//!   POST   /:job_id/patches     — apply a single patch
//!   POST   /:job_id/run-tests   — execute a test command in the sandbox
//!   POST   /:job_id/submit      — package workspace into a Draft + Proposal
//!   POST   /:job_id/cancel      — cancel a running job
//!
//! State transitions are validated server-side via the harness state
//! machine; agents only see opaque next-step affordances.

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use skillhub_domain::iteration::{IterationJob, IterationState, PatchOp};
use skillhub_domain::proposal::{ProposalState, VersionDraft, VersionProposal};

use crate::error::ApiError;
use crate::middleware::AuthPrincipal;
use crate::state::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/:skill_id/iterations", post(open).get(list))
        .route("/:skill_id/iterations/:job_id", get(read))
        .route("/:skill_id/iterations/:job_id/patches", post(push_patch))
        .route("/:skill_id/iterations/:job_id/run-tests", post(run_tests))
        .route("/:skill_id/iterations/:job_id/submit", post(submit))
        .route("/:skill_id/iterations/:job_id/cancel", post(cancel))
}

#[derive(Debug, Deserialize)]
pub struct OpenBody {
    pub agent: String,
    pub intent: String,
    pub base_version_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct JobDto {
    pub id: Uuid,
    pub state: IterationState,
    pub agent: String,
    pub intent: String,
    pub started_at: Option<chrono::DateTime<Utc>>,
    pub finished_at: Option<chrono::DateTime<Utc>>,
    pub submitted_proposal: Option<Uuid>,
    pub error: Option<String>,
}

impl From<IterationJob> for JobDto {
    fn from(j: IterationJob) -> Self {
        Self {
            id: j.id,
            state: j.state,
            agent: j.agent,
            intent: j.intent,
            started_at: j.started_at,
            finished_at: j.finished_at,
            submitted_proposal: j.submitted_proposal,
            error: j.error,
        }
    }
}

async fn open(
    State(state): State<Arc<AppState>>,
    AuthPrincipal(principal): AuthPrincipal,
    Path(skill_id): Path<Uuid>,
    Json(body): Json<OpenBody>,
) -> Result<(StatusCode, Json<JobDto>), ApiError> {
    let actor = principal
        .user_id
        .ok_or(skillhub_domain::DomainError::Unauthorized)?;
    let id = Uuid::new_v4();
    let job = IterationJob {
        id,
        skill_id,
        base_version_id: body.base_version_id,
        started_by: actor,
        agent: body.agent,
        intent: body.intent,
        state: IterationState::Queued,
        workspace_key: format!("iterations/{}", id),
        log_uri: None,
        error: None,
        submitted_proposal: None,
        created_at: Utc::now(),
        started_at: None,
        finished_at: None,
    };
    state.iterations.create(&job).await?;
    state.harness.ensure_workspace(id).map_err(map_harness)?;
    let job = state
        .harness
        .transition(job, IterationState::Running)
        .map_err(map_harness)?;
    state.iterations.update_state(&job).await?;
    Ok((StatusCode::CREATED, Json(job.into())))
}

async fn list(
    State(state): State<Arc<AppState>>,
    Path(skill_id): Path<Uuid>,
) -> Result<Json<Vec<JobDto>>, ApiError> {
    let list = state.iterations.list_for_skill(skill_id).await?;
    Ok(Json(list.into_iter().map(Into::into).collect()))
}

async fn read(
    State(state): State<Arc<AppState>>,
    Path((_skill_id, job_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<JobDto>, ApiError> {
    let job = state
        .iterations
        .find(job_id)
        .await?
        .ok_or_else(|| skillhub_domain::DomainError::NotFound(format!("job {job_id}")))?;
    Ok(Json(job.into()))
}

#[derive(Debug, Deserialize)]
pub struct PatchBody {
    pub seq: i32,
    pub path: String,
    pub op: PatchOp,
    /// base64 content for writes.
    pub data_b64: Option<String>,
    pub new_path: Option<String>,
}

async fn push_patch(
    State(state): State<Arc<AppState>>,
    Path((_skill_id, job_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<PatchBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    let data = match (body.op, body.data_b64.as_ref()) {
        (PatchOp::Write, Some(b)) => Some(bytes::Bytes::from(
            STANDARD
                .decode(b)
                .map_err(|e| skillhub_domain::DomainError::Validation(e.to_string()))?,
        )),
        _ => None,
    };
    let input = skillhub_harness::PatchInput {
        path: body.path,
        op: body.op,
        data,
        new_path: body.new_path,
    };
    let rec = state
        .harness
        .apply_patch(job_id, body.seq, input)
        .await
        .map_err(map_harness)?;
    state.iterations.append_patch(&rec).await?;
    Ok(Json(serde_json::json!({ "patch_id": rec.id })))
}

#[derive(Debug, Deserialize)]
pub struct RunTestsBody {
    pub command: String,
}

async fn run_tests(
    State(state): State<Arc<AppState>>,
    Path((_skill_id, job_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<RunTestsBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let (outcome, record) = state
        .harness
        .run_tests(job_id, &body.command, None)
        .await
        .map_err(map_harness)?;
    state.iterations.append_test_run(&record).await?;
    Ok(Json(serde_json::json!({
        "command": outcome.command,
        "exit_code": outcome.exit_code,
        "duration_ms": outcome.duration_ms,
        "timed_out": outcome.timed_out,
        "stdout": outcome.stdout,
        "stderr": outcome.stderr,
    })))
}

#[derive(Debug, Deserialize)]
pub struct SubmitBody {
    pub target_version: String,
    pub summary: Option<String>,
    pub title: String,
    pub body: Option<String>,
}

async fn submit(
    State(state): State<Arc<AppState>>,
    AuthPrincipal(principal): AuthPrincipal,
    Path((skill_id, job_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<SubmitBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let actor = principal
        .user_id
        .ok_or(skillhub_domain::DomainError::Unauthorized)?;
    let mut job = state
        .iterations
        .find(job_id)
        .await?
        .ok_or_else(|| skillhub_domain::DomainError::NotFound("job".into()))?;
    if job.state != IterationState::Running {
        // Allow submit only from a successful run.
        job = state
            .harness
            .transition(job, IterationState::Succeeded)
            .map_err(map_harness)?;
    } else {
        job = state
            .harness
            .transition(job, IterationState::Succeeded)
            .map_err(map_harness)?;
    }

    // Snapshot files. In a fuller impl we'd zip and upload to the
    // ObjectStore; for now we record the snapshot size in the manifest.
    let snapshot = state.harness.snapshot(job_id).await.map_err(map_harness)?;
    let mut total: u64 = 0;
    for v in snapshot.values() {
        total += v.len() as u64;
    }

    let draft = VersionDraft {
        id: Uuid::new_v4(),
        skill_id,
        base_version_id: job.base_version_id,
        target_version: body.target_version,
        manifest: serde_json::json!({
            "source": "iteration",
            "iteration_id": job_id,
            "files": snapshot.len(),
            "bytes": total,
        }),
        storage_key: None,
        size_bytes: Some(total as i64),
        checksum_sha256: None,
        summary: body.summary,
        created_by: actor,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    state.drafts.create(&draft).await?;

    let proposal = VersionProposal {
        id: Uuid::new_v4(),
        skill_id,
        draft_id: draft.id,
        state: ProposalState::Open,
        title: body.title,
        body: body.body,
        opened_by: actor,
        opened_at: Utc::now(),
        decided_by: None,
        decided_at: None,
        merged_version_id: None,
    };
    state.proposals.create(&proposal).await?;

    job.submitted_proposal = Some(proposal.id);
    job = state
        .harness
        .transition(job, IterationState::Submitted)
        .map_err(map_harness)?;
    state.iterations.update_state(&job).await?;

    Ok(Json(serde_json::json!({
        "draft_id": draft.id,
        "proposal_id": proposal.id,
        "job_state": job.state,
    })))
}

async fn cancel(
    State(state): State<Arc<AppState>>,
    Path((_skill_id, job_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<JobDto>, ApiError> {
    let job = state
        .iterations
        .find(job_id)
        .await?
        .ok_or_else(|| skillhub_domain::DomainError::NotFound("job".into()))?;
    let job = state
        .harness
        .transition(job, IterationState::Cancelled)
        .map_err(map_harness)?;
    state.iterations.update_state(&job).await?;
    Ok(Json(job.into()))
}

fn map_harness(e: skillhub_harness::HarnessError) -> ApiError {
    ApiError::Domain(skillhub_domain::DomainError::Internal(e.to_string()))
}
