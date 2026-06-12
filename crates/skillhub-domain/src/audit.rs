//! Audit log — append-only record of governance-relevant actions.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::DomainResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: Uuid,
    pub actor_id: Option<Uuid>,
    pub action: String,
    pub target_type: String,
    pub target_id: Option<String>,
    pub payload: serde_json::Value,
    pub occurred_at: DateTime<Utc>,
}

#[async_trait]
pub trait AuditRepository: Send + Sync {
    async fn append(&self, entry: &AuditEntry) -> DomainResult<()>;
    async fn query(&self, action: Option<&str>, limit: i64) -> DomainResult<Vec<AuditEntry>>;
}
