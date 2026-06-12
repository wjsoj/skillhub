//! Per-skill collaborator membership, decoupled from namespace.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::DomainResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CollaboratorRole {
    Maintainer,
    Writer,
    Reader,
}

impl CollaboratorRole {
    pub fn rank(self) -> u8 {
        match self {
            Self::Reader => 1,
            Self::Writer => 2,
            Self::Maintainer => 3,
        }
    }
    pub fn at_least(self, min: Self) -> bool {
        self.rank() >= min.rank()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collaborator {
    pub skill_id: Uuid,
    pub user_id: Uuid,
    pub role: CollaboratorRole,
    pub added_by: Uuid,
    pub added_at: DateTime<Utc>,
}

#[async_trait]
pub trait CollaboratorRepository: Send + Sync {
    async fn upsert(&self, c: &Collaborator) -> DomainResult<()>;
    async fn remove(&self, skill_id: Uuid, user_id: Uuid) -> DomainResult<()>;
    async fn list_for_skill(&self, skill_id: Uuid) -> DomainResult<Vec<Collaborator>>;
    async fn role_of(&self, skill_id: Uuid, user_id: Uuid) -> DomainResult<Option<CollaboratorRole>>;
}
