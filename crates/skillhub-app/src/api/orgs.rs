//! /api/v1/orgs and /api/v1/departments
//!
//! Minimal CRUD around the organisation tree. Department creation
//! eagerly updates the closure table so policy queries stay O(1).

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use skillhub_domain::department::{
    CrossScopeGrant, Department, DepartmentMembership, DepartmentRole, GrantScope,
};

use crate::error::ApiError;
use crate::middleware::AuthPrincipal;
use crate::state::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/orgs/:org_id/departments", get(list).post(create))
        .route(
            "/departments/:id/members",
            post(add_member).get(list_members),
        )
        .route("/grants", post(create_grant))
}

#[derive(Debug, Deserialize)]
pub struct CreateDeptBody {
    pub slug: String,
    pub name: String,
    pub parent_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct DepartmentDto {
    pub id: Uuid,
    pub org_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub slug: String,
    pub name: String,
}

impl From<Department> for DepartmentDto {
    fn from(d: Department) -> Self {
        Self {
            id: d.id,
            org_id: d.org_id,
            parent_id: d.parent_id,
            slug: d.slug,
            name: d.name,
        }
    }
}

async fn list(
    State(state): State<Arc<AppState>>,
    Path(org_id): Path<Uuid>,
) -> Result<Json<Vec<DepartmentDto>>, ApiError> {
    let list = state.departments.list_org(org_id).await?;
    Ok(Json(list.into_iter().map(Into::into).collect()))
}

async fn create(
    State(state): State<Arc<AppState>>,
    Path(org_id): Path<Uuid>,
    Json(body): Json<CreateDeptBody>,
) -> Result<Json<DepartmentDto>, ApiError> {
    let d = Department {
        id: Uuid::new_v4(),
        org_id,
        parent_id: body.parent_id,
        slug: body.slug,
        name: body.name,
        created_at: Utc::now(),
    };
    state.departments.create(&d).await?;
    Ok(Json(d.into()))
}

#[derive(Debug, Deserialize)]
pub struct AddMemberBody {
    pub user_id: Uuid,
    pub role: DepartmentRole,
}

async fn add_member(
    State(state): State<Arc<AppState>>,
    AuthPrincipal(principal): AuthPrincipal,
    Path(dept_id): Path<Uuid>,
    Json(body): Json<AddMemberBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let m = DepartmentMembership {
        department_id: dept_id,
        user_id: body.user_id,
        role: body.role,
        granted_by: principal.user_id,
        joined_at: Utc::now(),
    };
    state.department_memberships.upsert(&m).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn list_members(
    State(_state): State<Arc<AppState>>,
    Path(_dept_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // TODO: add list_for_department to the repository
    Ok(Json(serde_json::json!({ "members": [] })))
}

#[derive(Debug, Deserialize)]
pub struct CreateGrantBody {
    pub grantee_department_id: Option<Uuid>,
    pub grantee_user_id: Option<Uuid>,
    pub target_department_id: Option<Uuid>,
    pub target_namespace_id: Option<Uuid>,
    pub target_skill_id: Option<Uuid>,
    pub scope: GrantScope,
    pub reason: String,
    pub expires_at: Option<chrono::DateTime<Utc>>,
}

async fn create_grant(
    State(state): State<Arc<AppState>>,
    AuthPrincipal(principal): AuthPrincipal,
    Json(body): Json<CreateGrantBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let grantor = principal
        .user_id
        .ok_or(skillhub_domain::DomainError::Unauthorized)?;
    let grant = CrossScopeGrant {
        id: Uuid::new_v4(),
        grantee_department_id: body.grantee_department_id,
        grantee_user_id: body.grantee_user_id,
        target_department_id: body.target_department_id,
        target_namespace_id: body.target_namespace_id,
        target_skill_id: body.target_skill_id,
        scope: body.scope,
        reason: body.reason,
        granted_by: grantor,
        granted_at: Utc::now(),
        expires_at: body.expires_at,
        revoked_at: None,
    };
    state.cross_grants.create(&grant).await?;
    Ok(Json(serde_json::json!({ "id": grant.id })))
}
