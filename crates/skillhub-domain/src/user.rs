//! User aggregate + OAuth identity linkage.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::DomainResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: Option<String>,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub is_super_admin: bool,
    pub password_hash: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OauthIdentity {
    pub id: Uuid,
    pub user_id: Uuid,
    pub provider: String,
    pub subject: String,
    pub linked_at: DateTime<Utc>,
}

#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> DomainResult<Option<User>>;
    async fn find_by_username(&self, username: &str) -> DomainResult<Option<User>>;
    async fn find_by_oauth(&self, provider: &str, subject: &str) -> DomainResult<Option<User>>;
    async fn create(&self, user: &User) -> DomainResult<()>;
    async fn link_oauth(&self, identity: &OauthIdentity) -> DomainResult<()>;
    async fn merge(&self, source: Uuid, target: Uuid) -> DomainResult<()>;
}
