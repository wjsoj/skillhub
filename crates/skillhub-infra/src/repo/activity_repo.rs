use async_trait::async_trait;
use sqlx::Row;
use uuid::Uuid;

use skillhub_domain::activity::{ActivityEvent, ActivityRepository};
use skillhub_domain::{DomainError, DomainResult};

use crate::db::PgPool;

pub struct PgActivityRepo {
    pub pool: PgPool,
}

#[async_trait]
impl ActivityRepository for PgActivityRepo {
    async fn append(&self, e: &ActivityEvent) -> DomainResult<()> {
        sqlx::query(
            "INSERT INTO activity_events
                (id, skill_id, namespace_id, actor_id, verb, payload, occurred_at)
             VALUES ($1,$2,$3,$4,$5,$6,$7)",
        )
        .bind(e.id)
        .bind(e.skill_id)
        .bind(e.namespace_id)
        .bind(e.actor_id)
        .bind(&e.verb)
        .bind(&e.payload)
        .bind(e.occurred_at)
        .execute(&self.pool)
        .await
        .map_err(|er| DomainError::Internal(er.to_string()))?;
        Ok(())
    }

    async fn for_skill(&self, skill_id: Uuid, limit: i64) -> DomainResult<Vec<ActivityEvent>> {
        let rows = sqlx::query(
            "SELECT id, skill_id, namespace_id, actor_id, verb, payload, occurred_at
             FROM activity_events WHERE skill_id=$1 ORDER BY occurred_at DESC LIMIT $2",
        )
        .bind(skill_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|er| DomainError::Internal(er.to_string()))?;
        Ok(rows
            .into_iter()
            .map(|r| ActivityEvent {
                id: r.get("id"),
                skill_id: r.get("skill_id"),
                namespace_id: r.get("namespace_id"),
                actor_id: r.get("actor_id"),
                verb: r.get("verb"),
                payload: r.get("payload"),
                occurred_at: r.get("occurred_at"),
            })
            .collect())
    }
}
