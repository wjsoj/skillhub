//! Drafts and proposals — the change-review pipeline that sits in front
//! of publishing a `SkillVersion`.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::DomainResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionDraft {
    pub id: Uuid,
    pub skill_id: Uuid,
    pub base_version_id: Option<Uuid>,
    pub target_version: String,
    pub manifest: serde_json::Value,
    pub storage_key: Option<String>,
    pub size_bytes: Option<i64>,
    pub checksum_sha256: Option<String>,
    pub summary: Option<String>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposalState {
    Open,
    ChangesRequested,
    Approved,
    Rejected,
    Merged,
    Withdrawn,
}

impl ProposalState {
    pub fn terminal(self) -> bool {
        matches!(self, Self::Merged | Self::Rejected | Self::Withdrawn)
    }
    pub fn can_transition_to(self, next: Self) -> bool {
        use ProposalState::*;
        match (self, next) {
            (Open, ChangesRequested | Approved | Rejected | Withdrawn) => true,
            (ChangesRequested, Open | Approved | Rejected | Withdrawn) => true,
            (Approved, Merged | Rejected | Withdrawn) => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionProposal {
    pub id: Uuid,
    pub skill_id: Uuid,
    pub draft_id: Uuid,
    pub state: ProposalState,
    pub title: String,
    pub body: Option<String>,
    pub opened_by: Uuid,
    pub opened_at: DateTime<Utc>,
    pub decided_by: Option<Uuid>,
    pub decided_at: Option<DateTime<Utc>>,
    pub merged_version_id: Option<Uuid>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewVerdict {
    Comment,
    Approve,
    RequestChanges,
    Reject,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposalReview {
    pub id: Uuid,
    pub proposal_id: Uuid,
    pub reviewer_id: Uuid,
    pub verdict: ReviewVerdict,
    pub body: Option<String>,
    pub reviewed_at: DateTime<Utc>,
}

#[async_trait]
pub trait DraftRepository: Send + Sync {
    async fn create(&self, d: &VersionDraft) -> DomainResult<()>;
    async fn update(&self, d: &VersionDraft) -> DomainResult<()>;
    async fn find(&self, id: Uuid) -> DomainResult<Option<VersionDraft>>;
    async fn list_for_skill(&self, skill_id: Uuid) -> DomainResult<Vec<VersionDraft>>;
}

#[async_trait]
pub trait ProposalRepository: Send + Sync {
    async fn create(&self, p: &VersionProposal) -> DomainResult<()>;
    async fn update_state(&self, p: &VersionProposal) -> DomainResult<()>;
    async fn find(&self, id: Uuid) -> DomainResult<Option<VersionProposal>>;
    async fn list_open(&self, skill_id: Uuid) -> DomainResult<Vec<VersionProposal>>;
    async fn record_review(&self, r: &ProposalReview) -> DomainResult<()>;
    async fn reviews_for(&self, proposal_id: Uuid) -> DomainResult<Vec<ProposalReview>>;
}
