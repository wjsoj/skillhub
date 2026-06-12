//! Organization + department tree + cross-scope grants.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::DomainResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Organization {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Department {
    pub id: Uuid,
    pub org_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub slug: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DepartmentRole {
    Director,
    Manager,
    Member,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepartmentMembership {
    pub department_id: Uuid,
    pub user_id: Uuid,
    pub role: DepartmentRole,
    pub granted_by: Option<Uuid>,
    pub joined_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GrantScope {
    Read,
    Write,
    Admin,
}

/// Cross-department grant. Exactly one of `grantee_*` is set, exactly one of `target_*` is set.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossScopeGrant {
    pub id: Uuid,
    pub grantee_department_id: Option<Uuid>,
    pub grantee_user_id: Option<Uuid>,
    pub target_department_id: Option<Uuid>,
    pub target_namespace_id: Option<Uuid>,
    pub target_skill_id: Option<Uuid>,
    pub scope: GrantScope,
    pub reason: String,
    pub granted_by: Uuid,
    pub granted_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
}

impl CrossScopeGrant {
    pub fn is_active(&self, now: DateTime<Utc>) -> bool {
        if self.revoked_at.is_some() {
            return false;
        }
        match self.expires_at {
            Some(exp) => exp > now,
            None => true,
        }
    }
}

#[async_trait]
pub trait DepartmentRepository: Send + Sync {
    async fn find(&self, id: Uuid) -> DomainResult<Option<Department>>;
    async fn find_by_slug(&self, org_id: Uuid, slug: &str) -> DomainResult<Option<Department>>;
    async fn create(&self, dept: &Department) -> DomainResult<()>;
    async fn list_org(&self, org_id: Uuid) -> DomainResult<Vec<Department>>;

    /// All descendants of `root` (inclusive of root).
    async fn descendants(&self, root: Uuid) -> DomainResult<Vec<Uuid>>;
    /// All ancestors of `node` (inclusive of node), nearest-first.
    async fn ancestors(&self, node: Uuid) -> DomainResult<Vec<Uuid>>;
    /// Insert closure rows so `node` is reachable from all ancestors of `parent` (or itself if root).
    async fn rewire_closure(&self, node: Uuid, parent: Option<Uuid>) -> DomainResult<()>;
}

#[async_trait]
pub trait DepartmentMembershipRepository: Send + Sync {
    async fn upsert(&self, m: &DepartmentMembership) -> DomainResult<()>;
    async fn remove(&self, department_id: Uuid, user_id: Uuid) -> DomainResult<()>;
    async fn list_user(&self, user_id: Uuid) -> DomainResult<Vec<DepartmentMembership>>;
}

#[async_trait]
pub trait CrossScopeGrantRepository: Send + Sync {
    async fn create(&self, grant: &CrossScopeGrant) -> DomainResult<()>;
    async fn revoke(&self, id: Uuid) -> DomainResult<()>;
    async fn list_for_user(&self, user_id: Uuid) -> DomainResult<Vec<CrossScopeGrant>>;
    async fn list_for_departments(&self, dept_ids: &[Uuid]) -> DomainResult<Vec<CrossScopeGrant>>;
}
