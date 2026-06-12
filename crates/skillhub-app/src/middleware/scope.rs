//! Department scope hydration.
//!
//! Given a `Principal`, fetch:
//!   * all departments the user transitively belongs to (closure table),
//!   * all live cross-scope grants targeted at the user or those depts.
//!
//! Produces a fully-populated `PermissionCtx` that the policy evaluator
//! can act on without further IO.

use std::collections::HashSet;
use std::sync::Arc;

use skillhub_auth::{PermissionCtx, Principal};
use skillhub_domain::department::{
    CrossScopeGrant, CrossScopeGrantRepository, DepartmentMembershipRepository,
    DepartmentRepository,
};
use skillhub_domain::DomainResult;

pub struct DeptScope;

impl DeptScope {
    /// Build the per-request permission context from the live database.
    pub async fn hydrate(
        principal: &Principal,
        departments: Arc<dyn DepartmentRepository>,
        memberships: Arc<dyn DepartmentMembershipRepository>,
        grants: Arc<dyn CrossScopeGrantRepository>,
    ) -> DomainResult<PermissionCtx> {
        let mut ctx = PermissionCtx::default();
        let Some(uid) = principal.user_id else {
            return Ok(ctx);
        };
        ctx.user_id = Some(uid);

        // Departments the user belongs to, and their descendants.
        let direct = memberships.list_user(uid).await?;
        let mut member_set: HashSet<_> = HashSet::new();
        for m in &direct {
            for d in departments.descendants(m.department_id).await? {
                member_set.insert(d);
            }
        }
        ctx.member_department_ids = member_set;

        // Grants attached to the user directly + grants for the user's home depts.
        let mut all_grants: Vec<CrossScopeGrant> = grants.list_for_user(uid).await?;
        if !direct.is_empty() {
            let dept_ids: Vec<_> = direct.iter().map(|m| m.department_id).collect();
            all_grants.extend(grants.list_for_departments(&dept_ids).await?);
        }
        for g in all_grants {
            if let Some(d) = g.target_department_id {
                ctx.granted_departments.push((d, g.scope));
            }
            if let Some(n) = g.target_namespace_id {
                ctx.granted_namespaces.push((n, g.scope));
            }
            if let Some(s) = g.target_skill_id {
                ctx.granted_skills.push((s, g.scope));
            }
        }

        Ok(ctx)
    }
}
