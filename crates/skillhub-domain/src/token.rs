//! API token aggregate — prefix-based secure hashing.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::DomainResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub prefix: String,
    pub hash: String,
    pub scopes: Vec<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[async_trait]
pub trait ApiTokenRepository: Send + Sync {
    async fn create(&self, token: &ApiToken) -> DomainResult<()>;
    async fn find_by_prefix(&self, prefix: &str) -> DomainResult<Option<ApiToken>>;
    async fn revoke(&self, id: Uuid) -> DomainResult<()>;
    async fn list_for_user(&self, user_id: Uuid) -> DomainResult<Vec<ApiToken>>;
}
