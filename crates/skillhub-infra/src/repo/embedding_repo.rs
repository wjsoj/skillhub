//! pgvector-backed skill embedding storage.
//!
//! We don't pull in a pgvector-specific crate; instead vectors are
//! serialised as the textual form pgvector accepts ([f1,f2,...]) and
//! cast in SQL with `::vector`. Top-K queries use cosine distance
//! (`<=>`) and translate distance back to a similarity score.

use async_trait::async_trait;
use sqlx::Row;
use uuid::Uuid;

use skillhub_domain::embedding::{
    EmbeddingRecord, EmbeddingRepository, EmbeddingSource, SimilarityHit,
};
use skillhub_domain::{DomainError, DomainResult};

use crate::db::PgPool;

fn source_str(s: EmbeddingSource) -> &'static str {
    match s {
        EmbeddingSource::Skill => "skill",
        EmbeddingSource::Version => "version",
        EmbeddingSource::Draft => "draft",
    }
}

fn vec_to_pg_text(v: &[f32]) -> String {
    let mut s = String::with_capacity(v.len() * 8);
    s.push('[');
    for (i, x) in v.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push_str(&x.to_string());
    }
    s.push(']');
    s
}

pub struct PgEmbeddingRepo {
    pub pool: PgPool,
}

#[async_trait]
impl EmbeddingRepository for PgEmbeddingRepo {
    async fn upsert(
        &self,
        rec: &EmbeddingRecord,
        vector: &[f32],
    ) -> DomainResult<()> {
        let vtext = vec_to_pg_text(vector);
        sqlx::query(
            "INSERT INTO skill_embeddings
                (id, skill_id, version_id, source, model, dim, embedding,
                 content_hash, text_preview, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7::vector,
                     $8, $9, now())
             ON CONFLICT (skill_id, (COALESCE(version_id, '00000000-0000-0000-0000-000000000000'::uuid)), source, model)
             DO UPDATE SET
                embedding = EXCLUDED.embedding,
                content_hash = EXCLUDED.content_hash,
                text_preview = EXCLUDED.text_preview,
                updated_at = now()",
        )
        .bind(rec.id)
        .bind(rec.skill_id)
        .bind(rec.version_id)
        .bind(source_str(rec.source))
        .bind(&rec.model)
        .bind(rec.dim)
        .bind(vtext)
        .bind(&rec.content_hash)
        .bind(&rec.text_preview)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;
        Ok(())
    }

    async fn similar(
        &self,
        vector: &[f32],
        exclude_skill: Option<Uuid>,
        visible_skill_ids: Option<&[Uuid]>,
        limit: i64,
    ) -> DomainResult<Vec<SimilarityHit>> {
        let vtext = vec_to_pg_text(vector);
        let mut sql = String::from(
            "SELECT s.id AS skill_id,
                    s.namespace_id AS namespace_id,
                    n.slug AS namespace_slug,
                    n.department_id AS department_id,
                    s.visibility AS visibility,
                    s.slug, s.display_name, s.description,
                    1 - (e.embedding <=> $1::vector) AS score
             FROM skill_embeddings e
             JOIN skills s     ON s.id = e.skill_id
             JOIN namespaces n ON n.id = s.namespace_id
             WHERE e.source = 'skill'",
        );
        if exclude_skill.is_some() {
            sql.push_str(" AND s.id <> $2");
        }
        if visible_skill_ids.is_some() {
            // $3 if exclude present, else $2
            let placeholder = if exclude_skill.is_some() { "$3" } else { "$2" };
            sql.push_str(&format!(" AND s.id = ANY({placeholder})"));
        }
        sql.push_str(" ORDER BY e.embedding <=> $1::vector ASC LIMIT ");
        sql.push_str(
            &(if exclude_skill.is_some() && visible_skill_ids.is_some() {
                "$4"
            } else if exclude_skill.is_some() || visible_skill_ids.is_some() {
                "$3"
            } else {
                "$2"
            })
            .to_string(),
        );

        let mut q = sqlx::query(&sql).bind(vtext);
        if let Some(x) = exclude_skill {
            q = q.bind(x);
        }
        if let Some(ids) = visible_skill_ids {
            q = q.bind(ids);
        }
        q = q.bind(limit);

        let rows = q
            .fetch_all(&self.pool)
            .await
            .map_err(|e| DomainError::Internal(e.to_string()))?;
        Ok(rows
            .into_iter()
            .map(|r| {
                let score_f64: f64 = r.get("score");
                let vis_s: String = r.get("visibility");
                let visibility = match vis_s.as_str() {
                    "global" => skillhub_domain::skill::Visibility::Global,
                    "team" => skillhub_domain::skill::Visibility::Team,
                    _ => skillhub_domain::skill::Visibility::Private,
                };
                SimilarityHit {
                    skill_id: r.get("skill_id"),
                    namespace_id: r.get("namespace_id"),
                    namespace_slug: r.get("namespace_slug"),
                    department_id: r.get("department_id"),
                    visibility,
                    slug: r.get("slug"),
                    display_name: r.get("display_name"),
                    description: r.get("description"),
                    score: score_f64 as f32,
                    matched_on: vec!["embedding".into()],
                }
            })
            .collect())
    }
}
