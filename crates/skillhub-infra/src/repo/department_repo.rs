//! Department + closure-table + cross-scope grant SQL impls.

use async_trait::async_trait;
use sqlx::Row;
use uuid::Uuid;

use skillhub_domain::department::{
    CrossScopeGrant, CrossScopeGrantRepository, Department, DepartmentMembership,
    DepartmentMembershipRepository, DepartmentRepository, DepartmentRole, GrantScope,
};
use skillhub_domain::{DomainError, DomainResult};

use crate::db::PgPool;

pub struct PgDepartmentRepo {
    pub pool: PgPool,
}

fn map_sqlx(e: sqlx::Error) -> DomainError {
    DomainError::Internal(e.to_string())
}

fn role_to_str(r: DepartmentRole) -> &'static str {
    match r {
        DepartmentRole::Director => "director",
        DepartmentRole::Manager => "manager",
        DepartmentRole::Member => "member",
    }
}

fn str_to_role(s: &str) -> DepartmentRole {
    match s {
        "director" => DepartmentRole::Director,
        "manager" => DepartmentRole::Manager,
        _ => DepartmentRole::Member,
    }
}

fn scope_to_str(g: GrantScope) -> &'static str {
    match g {
        GrantScope::Read => "read",
        GrantScope::Write => "write",
        GrantScope::Admin => "admin",
    }
}

fn str_to_scope(s: &str) -> GrantScope {
    match s {
        "admin" => GrantScope::Admin,
        "write" => GrantScope::Write,
        _ => GrantScope::Read,
    }
}

#[async_trait]
impl DepartmentRepository for PgDepartmentRepo {
    async fn find(&self, id: Uuid) -> DomainResult<Option<Department>> {
        let row = sqlx::query(
            "SELECT id, org_id, parent_id, slug, name, created_at FROM departments WHERE id=$1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.map(|r| Department {
            id: r.get("id"),
            org_id: r.get("org_id"),
            parent_id: r.get("parent_id"),
            slug: r.get("slug"),
            name: r.get("name"),
            created_at: r.get("created_at"),
        }))
    }

    async fn find_by_slug(&self, org_id: Uuid, slug: &str) -> DomainResult<Option<Department>> {
        let row = sqlx::query(
            "SELECT id, org_id, parent_id, slug, name, created_at
             FROM departments WHERE org_id=$1 AND slug=$2",
        )
        .bind(org_id)
        .bind(slug)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.map(|r| Department {
            id: r.get("id"),
            org_id: r.get("org_id"),
            parent_id: r.get("parent_id"),
            slug: r.get("slug"),
            name: r.get("name"),
            created_at: r.get("created_at"),
        }))
    }

    async fn create(&self, dept: &Department) -> DomainResult<()> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx)?;
        sqlx::query(
            "INSERT INTO departments (id, org_id, parent_id, slug, name, created_at)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(dept.id)
        .bind(dept.org_id)
        .bind(dept.parent_id)
        .bind(&dept.slug)
        .bind(&dept.name)
        .bind(dept.created_at)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        // self row
        sqlx::query(
            "INSERT INTO department_closure (ancestor_id, descendant_id, depth)
             VALUES ($1, $1, 0)",
        )
        .bind(dept.id)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        if let Some(parent) = dept.parent_id {
            // ancestors of parent + new node
            sqlx::query(
                "INSERT INTO department_closure (ancestor_id, descendant_id, depth)
                 SELECT ancestor_id, $2, depth + 1 FROM department_closure
                 WHERE descendant_id = $1",
            )
            .bind(parent)
            .bind(dept.id)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx)?;
        }
        tx.commit().await.map_err(map_sqlx)?;
        Ok(())
    }

    async fn list_org(&self, org_id: Uuid) -> DomainResult<Vec<Department>> {
        let rows = sqlx::query(
            "SELECT id, org_id, parent_id, slug, name, created_at
             FROM departments WHERE org_id=$1 ORDER BY slug",
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows
            .into_iter()
            .map(|r| Department {
                id: r.get("id"),
                org_id: r.get("org_id"),
                parent_id: r.get("parent_id"),
                slug: r.get("slug"),
                name: r.get("name"),
                created_at: r.get("created_at"),
            })
            .collect())
    }

    async fn descendants(&self, root: Uuid) -> DomainResult<Vec<Uuid>> {
        let rows = sqlx::query(
            "SELECT descendant_id FROM department_closure WHERE ancestor_id=$1",
        )
        .bind(root)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(|r| r.get("descendant_id")).collect())
    }

    async fn ancestors(&self, node: Uuid) -> DomainResult<Vec<Uuid>> {
        let rows = sqlx::query(
            "SELECT ancestor_id FROM department_closure WHERE descendant_id=$1 ORDER BY depth ASC",
        )
        .bind(node)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(|r| r.get("ancestor_id")).collect())
    }

    async fn rewire_closure(&self, _node: Uuid, _parent: Option<Uuid>) -> DomainResult<()> {
        // Moving a subtree: out of scope for the initial implementation.
        Err(DomainError::Internal("rewire_closure not implemented".into()))
    }
}

pub struct PgDepartmentMembershipRepo {
    pub pool: PgPool,
}

#[async_trait]
impl DepartmentMembershipRepository for PgDepartmentMembershipRepo {
    async fn upsert(&self, m: &DepartmentMembership) -> DomainResult<()> {
        sqlx::query(
            "INSERT INTO department_memberships (department_id, user_id, role, granted_by, joined_at)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (department_id, user_id) DO UPDATE
                SET role = EXCLUDED.role, granted_by = EXCLUDED.granted_by",
        )
        .bind(m.department_id)
        .bind(m.user_id)
        .bind(role_to_str(m.role))
        .bind(m.granted_by)
        .bind(m.joined_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    async fn remove(&self, department_id: Uuid, user_id: Uuid) -> DomainResult<()> {
        sqlx::query("DELETE FROM department_memberships WHERE department_id=$1 AND user_id=$2")
            .bind(department_id)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(map_sqlx)?;
        Ok(())
    }

    async fn list_user(&self, user_id: Uuid) -> DomainResult<Vec<DepartmentMembership>> {
        let rows = sqlx::query(
            "SELECT department_id, user_id, role, granted_by, joined_at
             FROM department_memberships WHERE user_id=$1",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows
            .into_iter()
            .map(|r| DepartmentMembership {
                department_id: r.get("department_id"),
                user_id: r.get("user_id"),
                role: str_to_role(r.get::<&str, _>("role")),
                granted_by: r.get("granted_by"),
                joined_at: r.get("joined_at"),
            })
            .collect())
    }
}

pub struct PgCrossScopeGrantRepo {
    pub pool: PgPool,
}

fn row_to_grant(r: sqlx::postgres::PgRow) -> CrossScopeGrant {
    CrossScopeGrant {
        id: r.get("id"),
        grantee_department_id: r.get("grantee_department_id"),
        grantee_user_id: r.get("grantee_user_id"),
        target_department_id: r.get("target_department_id"),
        target_namespace_id: r.get("target_namespace_id"),
        target_skill_id: r.get("target_skill_id"),
        scope: str_to_scope(r.get::<&str, _>("scope")),
        reason: r.get("reason"),
        granted_by: r.get("granted_by"),
        granted_at: r.get("granted_at"),
        expires_at: r.get("expires_at"),
        revoked_at: r.get("revoked_at"),
    }
}

#[async_trait]
impl CrossScopeGrantRepository for PgCrossScopeGrantRepo {
    async fn create(&self, g: &CrossScopeGrant) -> DomainResult<()> {
        sqlx::query(
            "INSERT INTO cross_scope_grants
                (id, grantee_department_id, grantee_user_id,
                 target_department_id, target_namespace_id, target_skill_id,
                 scope, reason, granted_by, granted_at, expires_at, revoked_at)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)",
        )
        .bind(g.id)
        .bind(g.grantee_department_id)
        .bind(g.grantee_user_id)
        .bind(g.target_department_id)
        .bind(g.target_namespace_id)
        .bind(g.target_skill_id)
        .bind(scope_to_str(g.scope))
        .bind(&g.reason)
        .bind(g.granted_by)
        .bind(g.granted_at)
        .bind(g.expires_at)
        .bind(g.revoked_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    async fn revoke(&self, id: Uuid) -> DomainResult<()> {
        sqlx::query("UPDATE cross_scope_grants SET revoked_at = now() WHERE id=$1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_sqlx)?;
        Ok(())
    }

    async fn list_for_user(&self, user_id: Uuid) -> DomainResult<Vec<CrossScopeGrant>> {
        let rows = sqlx::query(
            "SELECT * FROM cross_scope_grants
             WHERE grantee_user_id = $1 AND revoked_at IS NULL
               AND (expires_at IS NULL OR expires_at > now())",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(row_to_grant).collect())
    }

    async fn list_for_departments(&self, dept_ids: &[Uuid]) -> DomainResult<Vec<CrossScopeGrant>> {
        if dept_ids.is_empty() {
            return Ok(vec![]);
        }
        let rows = sqlx::query(
            "SELECT * FROM cross_scope_grants
             WHERE grantee_department_id = ANY($1) AND revoked_at IS NULL
               AND (expires_at IS NULL OR expires_at > now())",
        )
        .bind(dept_ids)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(row_to_grant).collect())
    }
}
