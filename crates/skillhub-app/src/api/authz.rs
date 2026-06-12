//! Shared authorization helpers for the HTTP handlers.
//!
//! Two concerns live here:
//!   * `vis_predicate` — a SQL fragment that limits a `skills s` row set to
//!     what the caller may see (global to everyone; team to namespace
//!     members; private to collaborators; everything to super-admins).
//!   * `require_*` — write-path guards that reject callers without an
//!     owning/maintaining role on the target namespace or skill.

use std::sync::Arc;

use skillhub_auth::{Principal, Role};
use sqlx::Row;
use uuid::Uuid;

use crate::error::ApiError;
use crate::state::AppState;
use skillhub_domain::DomainError;

/// Build the visibility WHERE-fragment. `p_super` / `p_uid` are the 1-based
/// bind positions of the `is_super: bool` and `uid: Uuid` parameters in the
/// surrounding query. Always references the table alias `s`.
pub fn vis_predicate(p_super: usize, p_uid: usize) -> String {
    format!(
        "(${p_super} \
         OR s.visibility = 'global' \
         OR (s.visibility = 'team' AND EXISTS \
             (SELECT 1 FROM namespace_members m WHERE m.namespace_id = s.namespace_id AND m.user_id = ${p_uid})) \
         OR (s.visibility = 'private' AND EXISTS \
             (SELECT 1 FROM skill_collaborators c WHERE c.skill_id = s.id AND c.user_id = ${p_uid})))"
    )
}

pub fn is_super(p: &Principal) -> bool {
    p.role == Role::SuperAdmin
}

/// Require that the caller can create skills in `ns_id` (super-admin, or an
/// owner/admin of the namespace).
pub async fn require_namespace_write(
    state: &AppState,
    principal: &Principal,
    ns_id: Uuid,
) -> Result<Uuid, ApiError> {
    let uid = principal.user_id.ok_or(DomainError::Unauthorized)?;
    if is_super(principal) {
        return Ok(uid);
    }
    let ok = sqlx::query(
        "SELECT 1 FROM namespace_members \
         WHERE namespace_id = $1 AND user_id = $2 AND role IN ('owner','admin')",
    )
    .bind(ns_id)
    .bind(uid)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| DomainError::Internal(e.to_string()))?
    .is_some();
    if ok {
        Ok(uid)
    } else {
        Err(DomainError::Forbidden("not a namespace owner/admin".into()).into())
    }
}

/// Require that the caller can publish versions of `skill_id` (super-admin,
/// a skill maintainer/writer, or an owner/admin of the skill's namespace).
pub async fn require_skill_publish(
    state: &AppState,
    principal: &Principal,
    skill_id: Uuid,
) -> Result<Uuid, ApiError> {
    let uid = principal.user_id.ok_or(DomainError::Unauthorized)?;
    if is_super(principal) {
        return Ok(uid);
    }
    let ok = sqlx::query(
        "SELECT 1 FROM skill_collaborators \
            WHERE skill_id = $1 AND user_id = $2 AND role IN ('maintainer','writer') \
         UNION \
         SELECT 1 FROM namespace_members nm JOIN skills s ON s.namespace_id = nm.namespace_id \
            WHERE s.id = $1 AND nm.user_id = $2 AND nm.role IN ('owner','admin')",
    )
    .bind(skill_id)
    .bind(uid)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| DomainError::Internal(e.to_string()))?
    .is_some();
    if ok {
        Ok(uid)
    } else {
        Err(DomainError::Forbidden("not a maintainer of this skill".into()).into())
    }
}
