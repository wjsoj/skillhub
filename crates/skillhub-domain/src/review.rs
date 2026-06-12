//! Review & governance — promotion of skill versions across scopes.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::DomainResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewRequest {
    pub id: Uuid,
    pub skill_version_id: Uuid,
    pub kind: ReviewKind,
    pub status: ReviewStatus,
    pub requested_by: Uuid,
    pub reviewed_by: Option<Uuid>,
    pub note: Option<String>,
    pub created_at: DateTime<Utc>,
    pub decided_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReviewKind {
    Publish,
    PromoteToGlobal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReviewStatus {
    Pending,
    Approved,
    Rejected,
}

#[async_trait]
pub trait ReviewRepository: Send + Sync {
    async fn create(&self, review: &ReviewRequest) -> DomainResult<()>;
    async fn list_pending(&self, namespace_id: Option<Uuid>) -> DomainResult<Vec<ReviewRequest>>;
    async fn decide(&self, review: &ReviewRequest) -> DomainResult<()>;
}
