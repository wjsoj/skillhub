//! /api/v1/skills — read + create + publish. Other write paths
//! (drafts/proposals, iterations, collaborators) live in their own modules.

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::Row;
use uuid::Uuid;

use crate::api::authz;
use crate::error::ApiError;
use crate::middleware::AuthPrincipal;
use crate::state::AppState;
use skillhub_domain::embedding::{EmbeddingRecord, EmbeddingSource};
use skillhub_domain::DomainError;
use skillhub_embeddings::SkillContent;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list_all).post(create_skill))
        // Static path: must be declared so axum prefers it over `/:id`.
        .route("/lookup", get(lookup_by_slug))
        .route("/:id", get(get_one))
        .route("/:id/publish", post(publish_version))
        .route("/:id/versions", get(list_versions))
        .route("/:id/star", get(star_status).post(add_star).delete(remove_star))
}

#[derive(Debug, Serialize)]
pub struct SkillDto {
    pub id: Uuid,
    pub namespace_id: Uuid,
    pub namespace_slug: String,
    pub department_id: Option<Uuid>,
    pub slug: String,
    pub display_name: String,
    pub description: Option<String>,
    pub visibility: String,
    pub manifest: serde_json::Value,
    pub readme: Option<String>,
    pub install_command: Option<String>,
    pub repository_url: Option<String>,
    pub tags: Vec<String>,
    pub downloads: i64,
    pub install_count: i64,
    pub stars: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

const SELECT_SKILL: &str = r#"
    SELECT s.id, s.namespace_id, n.slug AS namespace_slug, n.department_id,
           s.slug, s.display_name, s.description, s.visibility,
           s.manifest, s.readme, s.install_command, s.repository_url,
           s.tags, s.downloads, s.install_count, s.stars,
           s.created_at, s.updated_at
    FROM skills s
    JOIN namespaces n ON n.id = s.namespace_id
"#;

async fn list_all(
    State(state): State<Arc<AppState>>,
    AuthPrincipal(principal): AuthPrincipal,
) -> Result<Json<Vec<SkillDto>>, ApiError> {
    let uid = principal.user_id.ok_or(DomainError::Unauthorized)?;
    let sql = format!(
        "{SELECT_SKILL} WHERE {} ORDER BY s.install_count DESC, s.display_name ASC",
        authz::vis_predicate(1, 2)
    );
    let rows = sqlx::query(&sql)
        .bind(authz::is_super(&principal))
        .bind(uid)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;
    Ok(Json(rows.into_iter().map(row_to_dto).collect()))
}

async fn get_one(
    State(state): State<Arc<AppState>>,
    AuthPrincipal(principal): AuthPrincipal,
    Path(id): Path<Uuid>,
) -> Result<Json<SkillDto>, ApiError> {
    let uid = principal.user_id.ok_or(DomainError::Unauthorized)?;
    // Existence is not leaked: an invisible skill returns the same NotFound.
    let sql = format!(
        "{SELECT_SKILL} WHERE s.id = $3 AND {}",
        authz::vis_predicate(1, 2)
    );
    let row = sqlx::query(&sql)
        .bind(authz::is_super(&principal))
        .bind(uid)
        .bind(id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?
        .ok_or_else(|| DomainError::NotFound(format!("skill {id}")))?;
    Ok(Json(row_to_dto(row)))
}

#[derive(Debug, Deserialize)]
struct LookupParams {
    namespace: String,
    slug: String,
}

/// Resolve a skill by its human-readable `namespace/slug` (for friendly URLs),
/// honoring visibility. Same NotFound for invisible-or-absent skills.
async fn lookup_by_slug(
    State(state): State<Arc<AppState>>,
    AuthPrincipal(principal): AuthPrincipal,
    axum::extract::Query(p): axum::extract::Query<LookupParams>,
) -> Result<Json<SkillDto>, ApiError> {
    let uid = principal.user_id.ok_or(DomainError::Unauthorized)?;
    let sql = format!(
        "{SELECT_SKILL} WHERE n.slug = $3 AND s.slug = $4 AND {}",
        authz::vis_predicate(1, 2)
    );
    let row = sqlx::query(&sql)
        .bind(authz::is_super(&principal))
        .bind(uid)
        .bind(p.namespace.trim())
        .bind(p.slug.trim())
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?
        .ok_or_else(|| DomainError::NotFound(format!("skill {}/{}", p.namespace, p.slug)))?;
    Ok(Json(row_to_dto(row)))
}

#[derive(Debug, Deserialize)]
struct CreateSkillBody {
    /// Either a namespace slug or its UUID string.
    namespace: String,
    slug: String,
    display_name: String,
    description: Option<String>,
    #[serde(default = "default_visibility")]
    visibility: String,
    manifest: Option<serde_json::Value>,
    readme: Option<String>,
    install_command: Option<String>,
    repository_url: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
}

fn default_visibility() -> String {
    "team".into()
}

async fn create_skill(
    State(state): State<Arc<AppState>>,
    AuthPrincipal(principal): AuthPrincipal,
    Json(body): Json<CreateSkillBody>,
) -> Result<Json<SkillDto>, ApiError> {
    let slug = body.slug.trim().to_lowercase();
    if slug.is_empty() || body.display_name.trim().is_empty() {
        return Err(DomainError::Validation("slug and display_name are required".into()).into());
    }
    if !matches!(body.visibility.as_str(), "private" | "team" | "global") {
        return Err(DomainError::Validation("invalid visibility".into()).into());
    }
    // Only super-admins may mint globally-visible skills.
    if body.visibility == "global" && !authz::is_super(&principal) {
        return Err(DomainError::Forbidden("only super-admins can publish global skills".into()).into());
    }

    // Resolve the namespace by slug, falling back to UUID.
    let ns_id: Uuid = {
        let by_slug = sqlx::query_scalar::<_, Uuid>("SELECT id FROM namespaces WHERE slug = $1")
            .bind(body.namespace.trim())
            .fetch_optional(&state.pool)
            .await
            .map_err(|e| DomainError::Internal(e.to_string()))?;
        match by_slug {
            Some(id) => id,
            None => Uuid::parse_str(body.namespace.trim())
                .ok()
                .ok_or_else(|| DomainError::NotFound(format!("namespace '{}'", body.namespace)))?,
        }
    };

    // Authorization: caller must own/admin the namespace (or be super-admin).
    authz::require_namespace_write(&state, &principal, ns_id).await?;

    let dup = sqlx::query("SELECT 1 FROM skills WHERE namespace_id = $1 AND slug = $2")
        .bind(ns_id)
        .bind(&slug)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;
    if dup.is_some() {
        return Err(DomainError::AlreadyExists(format!("skill '{slug}' already exists in namespace")).into());
    }

    let manifest = body.manifest.clone().unwrap_or_else(|| serde_json::json!({}));
    let id: Uuid = sqlx::query_scalar(
        r#"INSERT INTO skills
           (namespace_id, slug, display_name, description, visibility,
            manifest, readme, install_command, repository_url, tags)
           VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
           RETURNING id"#,
    )
    .bind(ns_id)
    .bind(&slug)
    .bind(body.display_name.trim())
    .bind(body.description.as_deref().map(|s| s.trim()).filter(|s| !s.is_empty()))
    .bind(&body.visibility)
    .bind(&manifest)
    .bind(body.readme.as_deref())
    .bind(body.install_command.as_deref().map(|s| s.trim()).filter(|s| !s.is_empty()))
    .bind(body.repository_url.as_deref().map(|s| s.trim()).filter(|s| !s.is_empty()))
    .bind(&body.tags)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| DomainError::Internal(e.to_string()))?;

    // Best-effort: index the new skill so it shows up in duplicate-check
    // and (eventually) semantic search. Failures don't block creation.
    if let Err(e) = embed_skill(
        &state,
        id,
        &body.display_name,
        &slug,
        body.description.as_deref(),
        body.readme.as_deref(),
        &body.tags,
    )
    .await
    {
        tracing::warn!(skill = %id, error = %e, "embed-on-create failed");
    }

    let sql = format!("{SELECT_SKILL} WHERE s.id = $1");
    let row = sqlx::query(&sql)
        .bind(id)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;
    Ok(Json(row_to_dto(row)))
}

#[derive(Debug, Deserialize)]
struct PublishBody {
    version: String,
    manifest: Option<serde_json::Value>,
    #[serde(default)]
    tags: Vec<String>,
}

#[derive(Debug, Serialize)]
struct PublishedVersion {
    version_id: Uuid,
    skill_id: Uuid,
    version: String,
    storage_key: String,
    checksum_sha256: String,
    status: String,
}

async fn publish_version(
    State(state): State<Arc<AppState>>,
    AuthPrincipal(principal): AuthPrincipal,
    Path(skill_id): Path<Uuid>,
    Json(body): Json<PublishBody>,
) -> Result<Json<PublishedVersion>, ApiError> {
    // Authorization: only a maintainer of the skill (or namespace owner/admin,
    // or super-admin) may publish a version.
    let uid = authz::require_skill_publish(&state, &principal, skill_id).await?;
    let version = body.version.trim();
    if semver::Version::parse(version).is_err() {
        return Err(DomainError::Validation(format!("'{version}' is not valid semver")).into());
    }

    // Resolve the skill + its namespace slug (for the storage key).
    let srow = sqlx::query(
        r#"SELECT s.slug, n.slug AS ns_slug, s.manifest
           FROM skills s JOIN namespaces n ON n.id = s.namespace_id
           WHERE s.id = $1"#,
    )
    .bind(skill_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| DomainError::Internal(e.to_string()))?
    .ok_or_else(|| DomainError::NotFound(format!("skill {skill_id}")))?;
    let skill_slug: String = srow.get("slug");
    let ns_slug: String = srow.get("ns_slug");
    let skill_manifest: serde_json::Value = srow.get("manifest");

    let dup = sqlx::query("SELECT 1 FROM skill_versions WHERE skill_id = $1 AND version = $2")
        .bind(skill_id)
        .bind(version)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;
    if dup.is_some() {
        return Err(DomainError::AlreadyExists(format!("version {version} already published")).into());
    }

    let manifest = body.manifest.unwrap_or(skill_manifest);
    let manifest_bytes = serde_json::to_vec(&manifest).unwrap_or_default();
    let checksum: String = {
        let mut h = Sha256::new();
        h.update(&manifest_bytes);
        h.finalize().iter().map(|b| format!("{b:02x}")).collect()
    };
    let storage_key = format!("skills/{ns_slug}/{skill_slug}/{version}.tgz");
    let size = manifest_bytes.len() as i64;

    let version_id: Uuid = sqlx::query_scalar(
        r#"INSERT INTO skill_versions
           (skill_id, version, tags, manifest, storage_key, size_bytes,
            checksum_sha256, status, published_by)
           VALUES ($1,$2,$3,$4,$5,$6,$7,'approved',$8)
           RETURNING id"#,
    )
    .bind(skill_id)
    .bind(version)
    .bind(&body.tags)
    .bind(&manifest)
    .bind(&storage_key)
    .bind(size)
    .bind(&checksum)
    .bind(uid)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| DomainError::Internal(e.to_string()))?;

    // Touch the skill so it sorts as recently updated.
    let _ = sqlx::query("UPDATE skills SET updated_at = now() WHERE id = $1")
        .bind(skill_id)
        .execute(&state.pool)
        .await;

    Ok(Json(PublishedVersion {
        version_id,
        skill_id,
        version: version.to_string(),
        storage_key,
        checksum_sha256: checksum,
        status: "approved".into(),
    }))
}

async fn embed_skill(
    state: &AppState,
    id: Uuid,
    display_name: &str,
    slug: &str,
    description: Option<&str>,
    readme: Option<&str>,
    tags: &[String],
) -> anyhow::Result<()> {
    let content = SkillContent {
        display_name,
        slug,
        description,
        readme,
        manifest: None,
        tags,
    };
    let text = content.to_embedding_input();
    let emb = state.embedder.embed(&text).await?;
    let record = EmbeddingRecord {
        id: Uuid::new_v4(),
        skill_id: id,
        version_id: None,
        source: EmbeddingSource::Skill,
        model: state.embedder.model().to_string(),
        dim: state.embedder.dim() as i32,
        content_hash: emb.content_hash.clone(),
        text_preview: Some(text.chars().take(200).collect()),
        updated_at: Utc::now(),
    };
    state
        .embeddings
        .upsert(&record, &emb.vector)
        .await
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    Ok(())
}

#[derive(Debug, Serialize)]
struct VersionDto {
    id: Uuid,
    version: String,
    tags: Vec<String>,
    status: String,
    checksum_sha256: String,
    size_bytes: i64,
    storage_key: String,
    published_by: Uuid,
    published_at: DateTime<Utc>,
}

async fn list_versions(
    State(state): State<Arc<AppState>>,
    Path(skill_id): Path<Uuid>,
) -> Result<Json<Vec<VersionDto>>, ApiError> {
    let rows = sqlx::query(
        r#"SELECT id, version, tags, status, checksum_sha256, size_bytes,
                  storage_key, published_by, published_at
           FROM skill_versions WHERE skill_id = $1
           ORDER BY published_at DESC"#,
    )
    .bind(skill_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| DomainError::Internal(e.to_string()))?;
    Ok(Json(
        rows.iter()
            .map(|r| VersionDto {
                id: r.get("id"),
                version: r.get("version"),
                tags: r.get("tags"),
                status: r.get("status"),
                checksum_sha256: r.get("checksum_sha256"),
                size_bytes: r.get("size_bytes"),
                storage_key: r.get("storage_key"),
                published_by: r.get("published_by"),
                published_at: r.get("published_at"),
            })
            .collect(),
    ))
}

#[derive(Debug, Serialize)]
struct StarStatus {
    starred: bool,
    stars: i64,
}

async fn recount_stars(state: &AppState, skill_id: Uuid) -> Result<i64, ApiError> {
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM skill_stars WHERE skill_id = $1")
        .bind(skill_id)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;
    let _ = sqlx::query("UPDATE skills SET stars = $1 WHERE id = $2")
        .bind(n)
        .bind(skill_id)
        .execute(&state.pool)
        .await;
    Ok(n)
}

async fn star_status(
    State(state): State<Arc<AppState>>,
    AuthPrincipal(principal): AuthPrincipal,
    Path(skill_id): Path<Uuid>,
) -> Result<Json<StarStatus>, ApiError> {
    let uid = principal.user_id.ok_or(DomainError::Unauthorized)?;
    let starred = sqlx::query("SELECT 1 FROM skill_stars WHERE skill_id = $1 AND user_id = $2")
        .bind(skill_id)
        .bind(uid)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?
        .is_some();
    let stars: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM skill_stars WHERE skill_id = $1")
        .bind(skill_id)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;
    Ok(Json(StarStatus { starred, stars }))
}

async fn add_star(
    State(state): State<Arc<AppState>>,
    AuthPrincipal(principal): AuthPrincipal,
    Path(skill_id): Path<Uuid>,
) -> Result<Json<StarStatus>, ApiError> {
    let uid = principal.user_id.ok_or(DomainError::Unauthorized)?;
    sqlx::query(
        "INSERT INTO skill_stars (skill_id, user_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(skill_id)
    .bind(uid)
    .execute(&state.pool)
    .await
    .map_err(|e| DomainError::Internal(e.to_string()))?;
    let stars = recount_stars(&state, skill_id).await?;
    Ok(Json(StarStatus { starred: true, stars }))
}

async fn remove_star(
    State(state): State<Arc<AppState>>,
    AuthPrincipal(principal): AuthPrincipal,
    Path(skill_id): Path<Uuid>,
) -> Result<Json<StarStatus>, ApiError> {
    let uid = principal.user_id.ok_or(DomainError::Unauthorized)?;
    sqlx::query("DELETE FROM skill_stars WHERE skill_id = $1 AND user_id = $2")
        .bind(skill_id)
        .bind(uid)
        .execute(&state.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;
    let stars = recount_stars(&state, skill_id).await?;
    Ok(Json(StarStatus { starred: false, stars }))
}

fn row_to_dto(r: sqlx::postgres::PgRow) -> SkillDto {
    SkillDto {
        id: r.get("id"),
        namespace_id: r.get("namespace_id"),
        namespace_slug: r.get("namespace_slug"),
        department_id: r.get("department_id"),
        slug: r.get("slug"),
        display_name: r.get("display_name"),
        description: r.get("description"),
        visibility: r.get("visibility"),
        manifest: r.get("manifest"),
        readme: r.get("readme"),
        install_command: r.get("install_command"),
        repository_url: r.get("repository_url"),
        tags: r.get("tags"),
        downloads: r.get("downloads"),
        install_count: r.get("install_count"),
        stars: r.get("stars"),
        created_at: r.get("created_at"),
        updated_at: r.get("updated_at"),
    }
}
