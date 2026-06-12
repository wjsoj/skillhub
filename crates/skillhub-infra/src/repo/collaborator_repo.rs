//! skill_collaborators SQL impl.

use async_trait::async_trait;
use sqlx::Row;
use uuid::Uuid;

use skillhub_domain::collaborator::{Collaborator, CollaboratorRepository, CollaboratorRole};
use skillhub_domain::{DomainError, DomainResult};

use crate::db::PgPool;

fn role_str(r: CollaboratorRole) -> &'static str {
    match r {
        CollaboratorRole::Maintainer => "maintainer",
        CollaboratorRole::Writer => "writer",
        CollaboratorRole::Reader => "reader",
    }
}
fn str_role(s: &str) -> CollaboratorRole {
    match s {
        "maintainer" => CollaboratorRole::Maintainer,
        "writer" => CollaboratorRole::Writer,
        _ => CollaboratorRole::Reader,
    }
}

pub struct PgCollaboratorRepo {
    pub pool: PgPool,
}

#[async_trait]
impl CollaboratorRepository for PgCollaboratorRepo {
    async fn upsert(&self, c: &Collaborator) -> DomainResult<()> {
        sqlx::query(
            "INSERT INTO skill_collaborators (skill_id, user_id, role, added_by, added_at)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (skill_id, user_id) DO UPDATE
                SET role = EXCLUDED.role, added_by = EXCLUDED.added_by",
        )
        .bind(c.skill_id)
        .bind(c.user_id)
        .bind(role_str(c.role))
        .bind(c.added_by)
        .bind(c.added_at)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;
        Ok(())
    }

    async fn remove(&self, skill_id: Uuid, user_id: Uuid) -> DomainResult<()> {
        sqlx::query("DELETE FROM skill_collaborators WHERE skill_id=$1 AND user_id=$2")
            .bind(skill_id)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Internal(e.to_string()))?;
        Ok(())
    }

    async fn list_for_skill(&self, skill_id: Uuid) -> DomainResult<Vec<Collaborator>> {
        let rows = sqlx::query(
            "SELECT skill_id, user_id, role, added_by, added_at
             FROM skill_collaborators WHERE skill_id=$1",
        )
        .bind(skill_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;
        Ok(rows
            .into_iter()
            .map(|r| Collaborator {
                skill_id: r.get("skill_id"),
                user_id: r.get("user_id"),
                role: str_role(r.get::<&str, _>("role")),
                added_by: r.get("added_by"),
                added_at: r.get("added_at"),
            })
            .collect())
    }

    async fn role_of(
        &self,
        skill_id: Uuid,
        user_id: Uuid,
    ) -> DomainResult<Option<CollaboratorRole>> {
        let row = sqlx::query("SELECT role FROM skill_collaborators WHERE skill_id=$1 AND user_id=$2")
            .bind(skill_id)
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| DomainError::Internal(e.to_string()))?;
        Ok(row.map(|r| str_role(r.get::<&str, _>("role"))))
    }
}
