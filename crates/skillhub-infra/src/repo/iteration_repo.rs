//! iteration_jobs / patches / test_runs SQL impl.

use async_trait::async_trait;
use sqlx::Row;
use uuid::Uuid;

use skillhub_domain::iteration::{
    IterationJob, IterationPatch, IterationRepository, IterationState, IterationTestRun, PatchOp,
};
use skillhub_domain::{DomainError, DomainResult};

use crate::db::PgPool;

fn state_str(s: IterationState) -> &'static str {
    match s {
        IterationState::Queued => "queued",
        IterationState::Running => "running",
        IterationState::Succeeded => "succeeded",
        IterationState::Failed => "failed",
        IterationState::Cancelled => "cancelled",
        IterationState::Submitted => "submitted",
    }
}
fn str_state(s: &str) -> IterationState {
    match s {
        "queued" => IterationState::Queued,
        "running" => IterationState::Running,
        "succeeded" => IterationState::Succeeded,
        "failed" => IterationState::Failed,
        "cancelled" => IterationState::Cancelled,
        _ => IterationState::Submitted,
    }
}
fn op_str(o: PatchOp) -> &'static str {
    match o {
        PatchOp::Write => "write",
        PatchOp::Delete => "delete",
        PatchOp::Rename => "rename",
    }
}
fn str_op(s: &str) -> PatchOp {
    match s {
        "delete" => PatchOp::Delete,
        "rename" => PatchOp::Rename,
        _ => PatchOp::Write,
    }
}

pub struct PgIterationRepo {
    pub pool: PgPool,
}

fn row_to_job(r: sqlx::postgres::PgRow) -> IterationJob {
    IterationJob {
        id: r.get("id"),
        skill_id: r.get("skill_id"),
        base_version_id: r.get("base_version_id"),
        started_by: r.get("started_by"),
        agent: r.get("agent"),
        intent: r.get("intent"),
        state: str_state(r.get::<&str, _>("state")),
        workspace_key: r.get("workspace_key"),
        log_uri: r.get("log_uri"),
        error: r.get("error"),
        submitted_proposal: r.get("submitted_proposal"),
        created_at: r.get("created_at"),
        started_at: r.get("started_at"),
        finished_at: r.get("finished_at"),
    }
}

#[async_trait]
impl IterationRepository for PgIterationRepo {
    async fn create(&self, j: &IterationJob) -> DomainResult<()> {
        sqlx::query(
            "INSERT INTO iteration_jobs
                (id, skill_id, base_version_id, started_by, agent, intent,
                 state, workspace_key, log_uri, error, submitted_proposal,
                 created_at, started_at, finished_at)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)",
        )
        .bind(j.id)
        .bind(j.skill_id)
        .bind(j.base_version_id)
        .bind(j.started_by)
        .bind(&j.agent)
        .bind(&j.intent)
        .bind(state_str(j.state))
        .bind(&j.workspace_key)
        .bind(&j.log_uri)
        .bind(&j.error)
        .bind(j.submitted_proposal)
        .bind(j.created_at)
        .bind(j.started_at)
        .bind(j.finished_at)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;
        Ok(())
    }

    async fn update_state(&self, j: &IterationJob) -> DomainResult<()> {
        sqlx::query(
            "UPDATE iteration_jobs SET state=$2, log_uri=$3, error=$4,
                submitted_proposal=$5, started_at=$6, finished_at=$7
             WHERE id=$1",
        )
        .bind(j.id)
        .bind(state_str(j.state))
        .bind(&j.log_uri)
        .bind(&j.error)
        .bind(j.submitted_proposal)
        .bind(j.started_at)
        .bind(j.finished_at)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;
        Ok(())
    }

    async fn find(&self, id: Uuid) -> DomainResult<Option<IterationJob>> {
        let row = sqlx::query("SELECT * FROM iteration_jobs WHERE id=$1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| DomainError::Internal(e.to_string()))?;
        Ok(row.map(row_to_job))
    }

    async fn list_for_skill(&self, skill_id: Uuid) -> DomainResult<Vec<IterationJob>> {
        let rows = sqlx::query(
            "SELECT * FROM iteration_jobs WHERE skill_id=$1 ORDER BY created_at DESC LIMIT 200",
        )
        .bind(skill_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;
        Ok(rows.into_iter().map(row_to_job).collect())
    }

    async fn append_patch(&self, p: &IterationPatch) -> DomainResult<()> {
        sqlx::query(
            "INSERT INTO iteration_patches
                (id, job_id, seq, path, op, new_path, content_sha256, size_bytes, applied_at)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
        )
        .bind(p.id)
        .bind(p.job_id)
        .bind(p.seq)
        .bind(&p.path)
        .bind(op_str(p.op))
        .bind(&p.new_path)
        .bind(&p.content_sha256)
        .bind(p.size_bytes)
        .bind(p.applied_at)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;
        Ok(())
    }

    async fn patches_for(&self, job_id: Uuid) -> DomainResult<Vec<IterationPatch>> {
        let rows = sqlx::query(
            "SELECT id, job_id, seq, path, op, new_path, content_sha256, size_bytes, applied_at
             FROM iteration_patches WHERE job_id=$1 ORDER BY seq",
        )
        .bind(job_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;
        Ok(rows
            .into_iter()
            .map(|r| IterationPatch {
                id: r.get("id"),
                job_id: r.get("job_id"),
                seq: r.get("seq"),
                path: r.get("path"),
                op: str_op(r.get::<&str, _>("op")),
                new_path: r.get("new_path"),
                content_sha256: r.get("content_sha256"),
                size_bytes: r.get("size_bytes"),
                applied_at: r.get("applied_at"),
            })
            .collect())
    }

    async fn append_test_run(&self, t: &IterationTestRun) -> DomainResult<()> {
        sqlx::query(
            "INSERT INTO iteration_test_runs
                (id, job_id, command, exit_code, duration_ms, stdout_uri, stderr_uri,
                 started_at, finished_at)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
        )
        .bind(t.id)
        .bind(t.job_id)
        .bind(&t.command)
        .bind(t.exit_code)
        .bind(t.duration_ms)
        .bind(&t.stdout_uri)
        .bind(&t.stderr_uri)
        .bind(t.started_at)
        .bind(t.finished_at)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;
        Ok(())
    }
}
