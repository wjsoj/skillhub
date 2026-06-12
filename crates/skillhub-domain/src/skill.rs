//! Skill package aggregate: the central domain entity.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use semver::Version;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::DomainResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub id: Uuid,
    pub namespace_id: Uuid,
    pub slug: String,
    pub display_name: String,
    pub description: Option<String>,
    pub visibility: Visibility,
    pub downloads: i64,
    pub stars: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Visibility {
    Private,
    Team,
    Global,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillVersion {
    pub id: Uuid,
    pub skill_id: Uuid,
    pub version: Version,
    pub tags: Vec<String>,
    pub manifest: serde_json::Value,
    pub storage_key: String,
    pub size_bytes: i64,
    pub checksum_sha256: String,
    pub published_at: DateTime<Utc>,
    pub published_by: Uuid,
    pub status: VersionStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VersionStatus {
    Pending,
    Approved,
    Rejected,
    Yanked,
}

#[async_trait]
pub trait SkillRepository: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> DomainResult<Option<Skill>>;
    async fn find_by_slug(&self, namespace: &str, slug: &str) -> DomainResult<Option<Skill>>;
    async fn create(&self, skill: &Skill) -> DomainResult<()>;
    async fn update(&self, skill: &Skill) -> DomainResult<()>;
    async fn delete(&self, id: Uuid) -> DomainResult<()>;
}

#[async_trait]
pub trait SkillVersionRepository: Send + Sync {
    async fn list_versions(&self, skill_id: Uuid) -> DomainResult<Vec<SkillVersion>>;
    async fn find_version(&self, skill_id: Uuid, version: &Version) -> DomainResult<Option<SkillVersion>>;
    async fn resolve_tag(&self, skill_id: Uuid, tag: &str) -> DomainResult<Option<SkillVersion>>;
    async fn publish(&self, version: &SkillVersion) -> DomainResult<()>;
    async fn yank(&self, id: Uuid) -> DomainResult<()>;
}
