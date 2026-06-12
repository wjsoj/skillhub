//! Per-skill activity timeline. Append-only.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::DomainResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityEvent {
    pub id: Uuid,
    pub skill_id: Option<Uuid>,
    pub namespace_id: Option<Uuid>,
    pub actor_id: Option<Uuid>,
    pub verb: String,
    pub payload: serde_json::Value,
    pub occurred_at: DateTime<Utc>,
}

#[async_trait]
pub trait ActivityRepository: Send + Sync {
    async fn append(&self, e: &ActivityEvent) -> DomainResult<()>;
    async fn for_skill(&self, skill_id: Uuid, limit: i64) -> DomainResult<Vec<ActivityEvent>>;
}
