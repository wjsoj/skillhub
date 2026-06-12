//! Central policy evaluator.
//!
//! Every authorization decision in the system goes through
//! [`PolicyEvaluator::evaluate`]. The evaluator is **default deny**:
//! if no rule explicitly grants the requested `Action` on the
//! `Target`, the decision is `Decision::Deny`.
//!
//! Resolution order (first match wins, but more privileged sources
//! can *upgrade* a weaker permission):
//!
//! 1. `super_admin` — global allow.
//! 2. Skill collaborator role (when target is a skill or its children).
//! 3. Namespace membership role (when target lives in that namespace).
//! 4. Department inheritance — a user's role at department `D` covers
//!    every descendant of `D`. The user's *home* department subtree
//!    is free; anything outside requires a grant.
//! 5. Cross-scope grants — explicit allow with scope (`read`/`write`/`admin`).
//! 6. `Visibility::Global` skills are readable by anyone in the same org
//!    (but writes still require a non-visibility path).
//!
//! Every denial returns the unmet requirement, so middlewares can write
//! a precise audit entry.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use skillhub_domain::collaborator::CollaboratorRole;
use skillhub_domain::department::GrantScope;
use skillhub_domain::namespace::NamespaceRole;
use skillhub_domain::skill::Visibility;

/// What the caller wants to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    // Read
    ReadSkill,
    ReadNamespace,
    ReadDepartment,
    SearchSkill,
    ReadActivity,

    // Write — skill scope
    CreateSkill,
    UpdateSkillMetadata,
    PublishVersion,
    YankVersion,
    OpenDraft,
    OpenProposal,
    ReviewProposal,
    MergeProposal,
    StartIteration,
    PushIterationPatch,
    SubmitIteration,
    AddCollaborator,
    RemoveCollaborator,

    // Write — namespace / department scope
    CreateNamespace,
    AddNamespaceMember,
    CreateDepartment,
    AddDepartmentMember,
    GrantCrossScope,
    RevokeCrossScope,
}

impl Action {
    /// Minimum grant scope required to satisfy this action via a
    /// cross-scope grant (when the user has no native role).
    pub fn min_grant_scope(self) -> GrantScope {
        use Action::*;
        match self {
            ReadSkill | ReadNamespace | ReadDepartment | SearchSkill | ReadActivity => {
                GrantScope::Read
            }
            CreateSkill
            | UpdateSkillMetadata
            | PublishVersion
            | YankVersion
            | OpenDraft
            | OpenProposal
            | ReviewProposal
            | StartIteration
            | PushIterationPatch
            | SubmitIteration => GrantScope::Write,
            MergeProposal
            | AddCollaborator
            | RemoveCollaborator
            | CreateNamespace
            | AddNamespaceMember
            | CreateDepartment
            | AddDepartmentMember
            | GrantCrossScope
            | RevokeCrossScope => GrantScope::Admin,
        }
    }
}

/// Resource being acted on. The evaluator never reads the database —
/// the caller fills in pre-loaded context (namespace_id, department_id,
/// visibility, etc.).
#[derive(Debug, Clone)]
pub struct Target {
    pub kind: TargetKind,
    pub skill_id: Option<Uuid>,
    pub namespace_id: Option<Uuid>,
    pub department_id: Option<Uuid>,
    pub visibility: Option<Visibility>,
    pub org_id: Option<Uuid>,
}

impl Target {
    pub fn skill(
        skill_id: Uuid,
        namespace_id: Uuid,
        department_id: Option<Uuid>,
        visibility: Visibility,
    ) -> Self {
        Self {
            kind: TargetKind::Skill,
            skill_id: Some(skill_id),
            namespace_id: Some(namespace_id),
            department_id,
            visibility: Some(visibility),
            org_id: None,
        }
    }

    pub fn namespace(namespace_id: Uuid, department_id: Option<Uuid>) -> Self {
        Self {
            kind: TargetKind::Namespace,
            skill_id: None,
            namespace_id: Some(namespace_id),
            department_id,
            visibility: None,
            org_id: None,
        }
    }

    pub fn department(department_id: Uuid, org_id: Uuid) -> Self {
        Self {
            kind: TargetKind::Department,
            skill_id: None,
            namespace_id: None,
            department_id: Some(department_id),
            visibility: None,
            org_id: Some(org_id),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetKind {
    Skill,
    Namespace,
    Department,
}

/// Everything we know about who is asking. Built once per request by
/// the middleware and stored in the axum extension map.
#[derive(Debug, Clone, Default)]
pub struct PermissionCtx {
    pub user_id: Option<Uuid>,
    pub is_super_admin: bool,

    /// Departments the user is a (transitive) member of. Already
    /// resolved through the closure table.
    pub member_department_ids: HashSet<Uuid>,

    /// Departments reachable via cross-scope grants (per scope).
    pub granted_departments: Vec<(Uuid, GrantScope)>,
    pub granted_namespaces: Vec<(Uuid, GrantScope)>,
    pub granted_skills: Vec<(Uuid, GrantScope)>,

    /// Namespace memberships: (namespace_id, role).
    pub namespace_roles: Vec<(Uuid, NamespaceRole)>,

    /// Per-skill collaborator role.
    pub skill_roles: Vec<(Uuid, CollaboratorRole)>,
}

impl PermissionCtx {
    pub fn anonymous() -> Self {
        Self::default()
    }

    fn namespace_role(&self, ns: Uuid) -> Option<NamespaceRole> {
        self.namespace_roles.iter().find(|(n, _)| *n == ns).map(|(_, r)| *r)
    }

    fn skill_role(&self, skill: Uuid) -> Option<CollaboratorRole> {
        self.skill_roles.iter().find(|(s, _)| *s == skill).map(|(_, r)| *r)
    }

    fn highest_grant<T: Copy + PartialEq>(
        list: &[(T, GrantScope)],
        key: T,
    ) -> Option<GrantScope> {
        list.iter()
            .filter(|(k, _)| *k == key)
            .map(|(_, s)| *s)
            .max_by_key(|s| match s {
                GrantScope::Read => 1u8,
                GrantScope::Write => 2,
                GrantScope::Admin => 3,
            })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Decision {
    Allow { reason: String },
    Deny { reason: String },
}

impl Decision {
    pub fn is_allow(&self) -> bool {
        matches!(self, Decision::Allow { .. })
    }
}

#[derive(Debug, Default, Clone)]
pub struct PolicyEvaluator;

impl PolicyEvaluator {
    pub fn new() -> Self {
        Self
    }

    pub fn evaluate(&self, ctx: &PermissionCtx, action: Action, target: &Target) -> Decision {
        // 1. super admin: blanket allow.
        if ctx.is_super_admin {
            return Decision::Allow {
                reason: "super_admin".into(),
            };
        }

        // 2. Skill collaborator role (when target carries a skill).
        if let Some(skill_id) = target.skill_id {
            if let Some(role) = ctx.skill_role(skill_id) {
                if collaborator_role_satisfies(action, role) {
                    return Decision::Allow {
                        reason: format!("collaborator:{:?}", role),
                    };
                }
            }
        }

        // 3. Namespace role.
        if let Some(ns) = target.namespace_id {
            if let Some(role) = ctx.namespace_role(ns) {
                if namespace_role_satisfies(action, role) {
                    return Decision::Allow {
                        reason: format!("namespace:{:?}", role),
                    };
                }
            }
        }

        // 4. Department inheritance.
        if let Some(dept) = target.department_id {
            if ctx.member_department_ids.contains(&dept) {
                if department_member_satisfies(action) {
                    return Decision::Allow {
                        reason: "department:member".into(),
                    };
                }
            }
        }

        // 5. Cross-scope grants.
        let need = action.min_grant_scope();
        let grant_satisfied = |g: GrantScope| grant_covers(g, need);
        if let Some(skill_id) = target.skill_id {
            if let Some(g) = PermissionCtx::highest_grant(&ctx.granted_skills, skill_id) {
                if grant_satisfied(g) {
                    return Decision::Allow {
                        reason: format!("grant:skill:{:?}", g),
                    };
                }
            }
        }
        if let Some(ns) = target.namespace_id {
            if let Some(g) = PermissionCtx::highest_grant(&ctx.granted_namespaces, ns) {
                if grant_satisfied(g) {
                    return Decision::Allow {
                        reason: format!("grant:namespace:{:?}", g),
                    };
                }
            }
        }
        if let Some(dept) = target.department_id {
            if let Some(g) = PermissionCtx::highest_grant(&ctx.granted_departments, dept) {
                if grant_satisfied(g) {
                    return Decision::Allow {
                        reason: format!("grant:department:{:?}", g),
                    };
                }
            }
        }

        // 6. Visibility fallback (READ only).
        if let Some(Visibility::Global) = target.visibility {
            if matches!(action, Action::ReadSkill | Action::SearchSkill) {
                return Decision::Allow {
                    reason: "visibility:global".into(),
                };
            }
        }

        Decision::Deny {
            reason: format!(
                "no rule allowed {:?} on {:?} (skill={:?}, ns={:?}, dept={:?})",
                action, target.kind, target.skill_id, target.namespace_id, target.department_id
            ),
        }
    }
}

fn collaborator_role_satisfies(action: Action, role: CollaboratorRole) -> bool {
    use Action::*;
    let min = match action {
        ReadSkill | ReadActivity => CollaboratorRole::Reader,
        OpenDraft | OpenProposal | StartIteration | PushIterationPatch | SubmitIteration
        | ReviewProposal | UpdateSkillMetadata => CollaboratorRole::Writer,
        PublishVersion | YankVersion | MergeProposal | AddCollaborator | RemoveCollaborator => {
            CollaboratorRole::Maintainer
        }
        _ => return false,
    };
    role.at_least(min)
}

fn namespace_role_satisfies(action: Action, role: NamespaceRole) -> bool {
    use Action::*;
    let rank = match role {
        NamespaceRole::Owner => 3,
        NamespaceRole::Admin => 2,
        NamespaceRole::Member => 1,
    };
    let need = match action {
        ReadSkill | ReadNamespace | SearchSkill | ReadActivity => 1,
        CreateSkill | UpdateSkillMetadata | OpenDraft | OpenProposal | StartIteration
        | PushIterationPatch | SubmitIteration | PublishVersion => 1,
        ReviewProposal | YankVersion | MergeProposal | AddCollaborator | RemoveCollaborator
        | AddNamespaceMember => 2,
        CreateNamespace => 3,
        _ => return false,
    };
    rank >= need
}

fn department_member_satisfies(action: Action) -> bool {
    use Action::*;
    matches!(
        action,
        ReadSkill | ReadNamespace | ReadDepartment | SearchSkill | ReadActivity
    )
}

fn grant_covers(have: GrantScope, need: GrantScope) -> bool {
    fn rank(g: GrantScope) -> u8 {
        match g {
            GrantScope::Read => 1,
            GrantScope::Write => 2,
            GrantScope::Admin => 3,
        }
    }
    rank(have) >= rank(need)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target_skill(visibility: Visibility) -> Target {
        Target::skill(
            Uuid::nil(),
            Uuid::nil(),
            Some(Uuid::from_u128(42)),
            visibility,
        )
    }

    #[test]
    fn anonymous_denied_by_default() {
        let pe = PolicyEvaluator::new();
        let ctx = PermissionCtx::anonymous();
        let d = pe.evaluate(&ctx, Action::ReadSkill, &target_skill(Visibility::Private));
        assert!(!d.is_allow());
    }

    #[test]
    fn global_visibility_grants_read() {
        let pe = PolicyEvaluator::new();
        let ctx = PermissionCtx::anonymous();
        let d = pe.evaluate(&ctx, Action::ReadSkill, &target_skill(Visibility::Global));
        assert!(d.is_allow());
    }

    #[test]
    fn department_member_can_read_but_not_publish() {
        let pe = PolicyEvaluator::new();
        let mut ctx = PermissionCtx::anonymous();
        ctx.member_department_ids.insert(Uuid::from_u128(42));
        let read = pe.evaluate(&ctx, Action::ReadSkill, &target_skill(Visibility::Private));
        let publish = pe.evaluate(&ctx, Action::PublishVersion, &target_skill(Visibility::Private));
        assert!(read.is_allow());
        assert!(!publish.is_allow());
    }

    #[test]
    fn collaborator_writer_can_open_proposal_but_not_merge() {
        let pe = PolicyEvaluator::new();
        let mut ctx = PermissionCtx::anonymous();
        ctx.skill_roles.push((Uuid::nil(), CollaboratorRole::Writer));
        let open = pe.evaluate(&ctx, Action::OpenProposal, &target_skill(Visibility::Private));
        let merge = pe.evaluate(&ctx, Action::MergeProposal, &target_skill(Visibility::Private));
        assert!(open.is_allow());
        assert!(!merge.is_allow());
    }

    #[test]
    fn cross_dept_grant_unlocks_write() {
        let pe = PolicyEvaluator::new();
        let mut ctx = PermissionCtx::anonymous();
        ctx.granted_departments.push((Uuid::from_u128(42), GrantScope::Write));
        let d = pe.evaluate(&ctx, Action::PublishVersion, &target_skill(Visibility::Private));
        assert!(d.is_allow());
    }

    #[test]
    fn super_admin_overrides_all() {
        let pe = PolicyEvaluator::new();
        let mut ctx = PermissionCtx::anonymous();
        ctx.is_super_admin = true;
        let d = pe.evaluate(&ctx, Action::GrantCrossScope, &target_skill(Visibility::Private));
        assert!(d.is_allow());
    }
}
