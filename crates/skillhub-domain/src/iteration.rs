//! AI-driven iteration jobs against a skill.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::DomainResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IterationState {
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
    Submitted,
}

impl IterationState {
    pub fn terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::Cancelled | Self::Submitted)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IterationJob {
    pub id: Uuid,
    pub skill_id: Uuid,
    pub base_version_id: Option<Uuid>,
    pub started_by: Uuid,
    pub agent: String,
    pub intent: String,
    pub state: IterationState,
    pub workspace_key: String,
    pub log_uri: Option<String>,
    pub error: Option<String>,
    pub submitted_proposal: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PatchOp {
    Write,
    Delete,
    Rename,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IterationPatch {
    pub id: Uuid,
    pub job_id: Uuid,
    pub seq: i32,
    pub path: String,
    pub op: PatchOp,
    pub new_path: Option<String>,
    pub content_sha256: Option<String>,
    pub size_bytes: Option<i64>,
    pub applied_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IterationTestRun {
    pub id: Uuid,
    pub job_id: Uuid,
    pub command: String,
    pub exit_code: Option<i32>,
    pub duration_ms: Option<i32>,
    pub stdout_uri: Option<String>,
    pub stderr_uri: Option<String>,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
}

#[async_trait]
pub trait IterationRepository: Send + Sync {
    async fn create(&self, j: &IterationJob) -> DomainResult<()>;
    async fn update_state(&self, j: &IterationJob) -> DomainResult<()>;
    async fn find(&self, id: Uuid) -> DomainResult<Option<IterationJob>>;
    async fn list_for_skill(&self, skill_id: Uuid) -> DomainResult<Vec<IterationJob>>;
    async fn append_patch(&self, p: &IterationPatch) -> DomainResult<()>;
    async fn patches_for(&self, job_id: Uuid) -> DomainResult<Vec<IterationPatch>>;
    async fn append_test_run(&self, t: &IterationTestRun) -> DomainResult<()>;
}
