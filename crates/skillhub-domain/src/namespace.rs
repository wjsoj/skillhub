//! Namespace aggregate: team / global scopes with membership.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::DomainResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Namespace {
    pub id: Uuid,
    pub slug: String,
    pub display_name: String,
    pub scope: NamespaceScope,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NamespaceScope {
    Global,
    Team,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NamespaceRole {
    Owner,
    Admin,
    Member,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamespaceMember {
    pub namespace_id: Uuid,
    pub user_id: Uuid,
    pub role: NamespaceRole,
    pub joined_at: DateTime<Utc>,
}

#[async_trait]
pub trait NamespaceRepository: Send + Sync {
    async fn find_by_slug(&self, slug: &str) -> DomainResult<Option<Namespace>>;
    async fn create(&self, namespace: &Namespace) -> DomainResult<()>;
    async fn add_member(&self, member: &NamespaceMember) -> DomainResult<()>;
    async fn role_of(&self, namespace_id: Uuid, user_id: Uuid) -> DomainResult<Option<NamespaceRole>>;
}
