//! Drafts + proposals + reviews SQL impls.

use async_trait::async_trait;
use sqlx::Row;
use uuid::Uuid;

use skillhub_domain::proposal::{
    DraftRepository, ProposalRepository, ProposalReview, ProposalState, ReviewVerdict,
    VersionDraft, VersionProposal,
};
use skillhub_domain::{DomainError, DomainResult};

use crate::db::PgPool;

fn state_str(s: ProposalState) -> &'static str {
    match s {
        ProposalState::Open => "open",
        ProposalState::ChangesRequested => "changes_requested",
        ProposalState::Approved => "approved",
        ProposalState::Rejected => "rejected",
        ProposalState::Merged => "merged",
        ProposalState::Withdrawn => "withdrawn",
    }
}
fn str_state(s: &str) -> ProposalState {
    match s {
        "open" => ProposalState::Open,
        "changes_requested" => ProposalState::ChangesRequested,
        "approved" => ProposalState::Approved,
        "rejected" => ProposalState::Rejected,
        "merged" => ProposalState::Merged,
        _ => ProposalState::Withdrawn,
    }
}
fn verdict_str(v: ReviewVerdict) -> &'static str {
    match v {
        ReviewVerdict::Comment => "comment",
        ReviewVerdict::Approve => "approve",
        ReviewVerdict::RequestChanges => "request_changes",
        ReviewVerdict::Reject => "reject",
    }
}
fn str_verdict(s: &str) -> ReviewVerdict {
    match s {
        "approve" => ReviewVerdict::Approve,
        "request_changes" => ReviewVerdict::RequestChanges,
        "reject" => ReviewVerdict::Reject,
        _ => ReviewVerdict::Comment,
    }
}

pub struct PgDraftRepo {
    pub pool: PgPool,
}

#[async_trait]
impl DraftRepository for PgDraftRepo {
    async fn create(&self, d: &VersionDraft) -> DomainResult<()> {
        sqlx::query(
            "INSERT INTO version_drafts
                (id, skill_id, base_version_id, target_version, manifest,
                 storage_key, size_bytes, checksum_sha256, summary,
                 created_by, created_at, updated_at)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)",
        )
        .bind(d.id)
        .bind(d.skill_id)
        .bind(d.base_version_id)
        .bind(&d.target_version)
        .bind(&d.manifest)
        .bind(&d.storage_key)
        .bind(d.size_bytes)
        .bind(&d.checksum_sha256)
        .bind(&d.summary)
        .bind(d.created_by)
        .bind(d.created_at)
        .bind(d.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;
        Ok(())
    }

    async fn update(&self, d: &VersionDraft) -> DomainResult<()> {
        sqlx::query(
            "UPDATE version_drafts SET
                target_version=$2, manifest=$3, storage_key=$4,
                size_bytes=$5, checksum_sha256=$6, summary=$7, updated_at=now()
             WHERE id=$1",
        )
        .bind(d.id)
        .bind(&d.target_version)
        .bind(&d.manifest)
        .bind(&d.storage_key)
        .bind(d.size_bytes)
        .bind(&d.checksum_sha256)
        .bind(&d.summary)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;
        Ok(())
    }

    async fn find(&self, id: Uuid) -> DomainResult<Option<VersionDraft>> {
        let row = sqlx::query("SELECT * FROM version_drafts WHERE id=$1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| DomainError::Internal(e.to_string()))?;
        Ok(row.map(row_to_draft))
    }

    async fn list_for_skill(&self, skill_id: Uuid) -> DomainResult<Vec<VersionDraft>> {
        let rows = sqlx::query(
            "SELECT * FROM version_drafts WHERE skill_id=$1 ORDER BY updated_at DESC",
        )
        .bind(skill_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;
        Ok(rows.into_iter().map(row_to_draft).collect())
    }
}

fn row_to_draft(r: sqlx::postgres::PgRow) -> VersionDraft {
    VersionDraft {
        id: r.get("id"),
        skill_id: r.get("skill_id"),
        base_version_id: r.get("base_version_id"),
        target_version: r.get("target_version"),
        manifest: r.get("manifest"),
        storage_key: r.get("storage_key"),
        size_bytes: r.get("size_bytes"),
        checksum_sha256: r.get("checksum_sha256"),
        summary: r.get("summary"),
        created_by: r.get("created_by"),
        created_at: r.get("created_at"),
        updated_at: r.get("updated_at"),
    }
}

pub struct PgProposalRepo {
    pub pool: PgPool,
}

fn row_to_proposal(r: sqlx::postgres::PgRow) -> VersionProposal {
    VersionProposal {
        id: r.get("id"),
        skill_id: r.get("skill_id"),
        draft_id: r.get("draft_id"),
        state: str_state(r.get::<&str, _>("state")),
        title: r.get("title"),
        body: r.get("body"),
        opened_by: r.get("opened_by"),
        opened_at: r.get("opened_at"),
        decided_by: r.get("decided_by"),
        decided_at: r.get("decided_at"),
        merged_version_id: r.get("merged_version_id"),
    }
}

#[async_trait]
impl ProposalRepository for PgProposalRepo {
    async fn create(&self, p: &VersionProposal) -> DomainResult<()> {
        sqlx::query(
            "INSERT INTO version_proposals
                (id, skill_id, draft_id, state, title, body, opened_by, opened_at,
                 decided_by, decided_at, merged_version_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)",
        )
        .bind(p.id)
        .bind(p.skill_id)
        .bind(p.draft_id)
        .bind(state_str(p.state))
        .bind(&p.title)
        .bind(&p.body)
        .bind(p.opened_by)
        .bind(p.opened_at)
        .bind(p.decided_by)
        .bind(p.decided_at)
        .bind(p.merged_version_id)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;
        Ok(())
    }

    async fn update_state(&self, p: &VersionProposal) -> DomainResult<()> {
        sqlx::query(
            "UPDATE version_proposals SET state=$2, decided_by=$3, decided_at=$4, merged_version_id=$5
             WHERE id=$1",
        )
        .bind(p.id)
        .bind(state_str(p.state))
        .bind(p.decided_by)
        .bind(p.decided_at)
        .bind(p.merged_version_id)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;
        Ok(())
    }

    async fn find(&self, id: Uuid) -> DomainResult<Option<VersionProposal>> {
        let row = sqlx::query("SELECT * FROM version_proposals WHERE id=$1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| DomainError::Internal(e.to_string()))?;
        Ok(row.map(row_to_proposal))
    }

    async fn list_open(&self, skill_id: Uuid) -> DomainResult<Vec<VersionProposal>> {
        let rows = sqlx::query(
            "SELECT * FROM version_proposals
             WHERE skill_id=$1 AND state IN ('open','changes_requested','approved')
             ORDER BY opened_at DESC",
        )
        .bind(skill_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;
        Ok(rows.into_iter().map(row_to_proposal).collect())
    }

    async fn record_review(&self, r: &ProposalReview) -> DomainResult<()> {
        sqlx::query(
            "INSERT INTO proposal_reviews (id, proposal_id, reviewer_id, verdict, body, reviewed_at)
             VALUES ($1,$2,$3,$4,$5,$6)",
        )
        .bind(r.id)
        .bind(r.proposal_id)
        .bind(r.reviewer_id)
        .bind(verdict_str(r.verdict))
        .bind(&r.body)
        .bind(r.reviewed_at)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;
        Ok(())
    }

    async fn reviews_for(&self, proposal_id: Uuid) -> DomainResult<Vec<ProposalReview>> {
        let rows = sqlx::query(
            "SELECT id, proposal_id, reviewer_id, verdict, body, reviewed_at
             FROM proposal_reviews WHERE proposal_id=$1 ORDER BY reviewed_at",
        )
        .bind(proposal_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;
        Ok(rows
            .into_iter()
            .map(|r| ProposalReview {
                id: r.get("id"),
                proposal_id: r.get("proposal_id"),
                reviewer_id: r.get("reviewer_id"),
                verdict: str_verdict(r.get::<&str, _>("verdict")),
                body: r.get("body"),
                reviewed_at: r.get("reviewed_at"),
            })
            .collect())
    }
}
