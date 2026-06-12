//! ClawHub-compatible registry API.
//!
//! Implements the HTTP contract the real `clawhub` CLI expects, so it can
//! `search` / `inspect` / `install` skills straight from this registry.
//! Mounted under `/clawhub`, so point the CLI at it with:
//!
//!   clawhub --registry http://<host>/clawhub install <slug>
//!
//! Reads are anonymous (the CLI sends its own token, which we simply ignore
//! unless it happens to be one of *our* JWTs — in which case the caller's
//! visibility applies). Anonymous callers see only `global` skills.
//!
//! Skills are text, so "download" zips up the SKILL.md + manifest.json that
//! live in Postgres — no object store involved. Slugs are flat: a clawhub
//! slug maps to our `skills.slug`, preferring a global, most-installed match.

use std::io::{Cursor, Write};
use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderMap},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use crate::error::ApiError;
use crate::state::AppState;
use skillhub_auth::{jwt, Principal, Role};
use skillhub_domain::DomainError;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v1/whoami", get(whoami))
        .route("/api/v1/search", get(search))
        .route("/api/v1/resolve", get(resolve))
        .route("/api/v1/download", get(download))
        .route("/api/v1/skills/:slug", get(skill_meta))
        .route("/api/v1/skills/:slug/versions", get(version_list))
        .route("/api/v1/skills/:slug/versions/:version", get(version_detail))
        .route("/api/v1/skills/:slug/file", get(skill_file))
}

/* ─────────── auth (optional) ─────────── */

fn role_from_str(s: &str) -> Role {
    match s {
        "super_admin" => Role::SuperAdmin,
        _ => Role::User,
    }
}

/// Best-effort: if the request carries one of *our* Bearer JWTs, resolve it.
/// A clawhub.ai `clh_…` token simply fails to verify and we fall to anonymous.
fn optional_principal(state: &AppState, headers: &HeaderMap) -> Option<Principal> {
    let auth = headers.get("authorization")?.to_str().ok()?;
    let tok = auth.strip_prefix("Bearer ")?.trim();
    let claims = jwt::verify(&state.config.auth.jwt_secret, tok).ok()?;
    Some(Principal {
        user_id: Some(claims.sub.parse().ok()?),
        username: Some(claims.username),
        role: role_from_str(&claims.role),
        scopes: vec![],
    })
}

/* ─────────── skill lookup with visibility ─────────── */

struct SkillRow {
    id: Uuid,
    slug: String,
    display_name: String,
    description: Option<String>,
    manifest: serde_json::Value,
    readme: Option<String>,
    install_count: i64,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

const SKILL_COLS: &str = "s.id, s.slug, s.display_name, s.description, s.manifest, \
                          s.readme, s.install_count, s.created_at, s.updated_at";

fn map_skill(r: &sqlx::postgres::PgRow) -> SkillRow {
    SkillRow {
        id: r.get("id"),
        slug: r.get("slug"),
        display_name: r.get("display_name"),
        description: r.get("description"),
        manifest: r.get("manifest"),
        readme: r.get("readme"),
        install_count: r.get("install_count"),
        created_at: r.get("created_at"),
        updated_at: r.get("updated_at"),
    }
}

/// Visibility binds for the shared `vis_predicate`. Anonymous resolves to
/// `(false, NIL)` — the nil UUID is never a member/collaborator, so the
/// predicate collapses to "global only" without a separate code path.
fn vis_binds(principal: &Option<Principal>) -> (bool, Uuid) {
    match principal {
        Some(p) => (p.role == Role::SuperAdmin, p.user_id.unwrap_or_else(Uuid::nil)),
        None => (false, Uuid::nil()),
    }
}

/// Resolve a flat clawhub slug to one of our skills, honoring visibility.
/// Anonymous → global only. Returns NotFound when nothing visible matches.
async fn resolve_skill(
    state: &AppState,
    slug: &str,
    principal: &Option<Principal>,
) -> Result<SkillRow, ApiError> {
    let (is_super, uid) = vis_binds(principal);
    let sql = format!(
        "SELECT {SKILL_COLS} FROM skills s JOIN namespaces n ON n.id = s.namespace_id \
         WHERE s.slug = $1 AND {} \
         ORDER BY (s.visibility = 'global') DESC, s.install_count DESC LIMIT 1",
        crate::api::authz::vis_predicate(2, 3)
    );
    let row = sqlx::query(&sql)
        .bind(slug)
        .bind(is_super)
        .bind(uid)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?
        .ok_or_else(|| DomainError::NotFound(format!("skill {slug}")))?;
    Ok(map_skill(&row))
}

fn manifest_version(m: &serde_json::Value) -> Option<String> {
    m.get("version").and_then(|v| v.as_str()).map(|s| s.to_string())
}

/// clawhub's schema only accepts `"MIT-0"` or null for license, so anything
/// else (Apache-2.0, MIT, …) is reported as null rather than failing validation.
fn coerce_license(m: &serde_json::Value) -> Option<String> {
    match m.get("license").and_then(|v| v.as_str()) {
        Some("MIT-0") => Some("MIT-0".into()),
        _ => None,
    }
}

/// The skill's latest published version, or a synthetic one derived from the
/// manifest when no `skill_versions` row exists yet (seed skills have none).
async fn latest_version(state: &AppState, sk: &SkillRow) -> (String, DateTime<Utc>) {
    let row = sqlx::query(
        "SELECT version, published_at FROM skill_versions \
         WHERE skill_id = $1 AND status = 'approved' ORDER BY published_at DESC LIMIT 1",
    )
    .bind(sk.id)
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten();
    match row {
        Some(r) => (r.get("version"), r.get("published_at")),
        None => (
            manifest_version(&sk.manifest).unwrap_or_else(|| "0.0.0".into()),
            sk.updated_at,
        ),
    }
}

/* ─────────── responses (camelCase, ms timestamps) ─────────── */

fn ms(t: DateTime<Utc>) -> i64 {
    t.timestamp_millis()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct VersionRef {
    version: String,
    created_at: i64,
    changelog: String,
    license: Option<String>,
}

/// The inner skill object (clawhub wraps it under a `skill` key).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SkillInner {
    slug: String,
    display_name: String,
    summary: Option<String>,
    tags: serde_json::Value,
    stats: serde_json::Value,
    created_at: i64,
    updated_at: i64,
}

/// `GET /api/v1/skills/:slug` → ApiV1SkillResponseSchema (a wrapper, not the
/// skill object directly): `{ skill, latestVersion, owner, moderation? }`.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SkillResponse {
    skill: Option<SkillInner>,
    latest_version: Option<VersionRef>,
    owner: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    moderation: Option<serde_json::Value>,
}

async fn skill_meta(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(slug): Path<String>,
) -> Result<Json<SkillResponse>, ApiError> {
    let principal = optional_principal(&state, &headers);
    let sk = resolve_skill(&state, &slug, &principal).await?;
    let (ver, ver_at) = latest_version(&state, &sk).await;
    Ok(Json(SkillResponse {
        skill: Some(SkillInner {
            slug: sk.slug.clone(),
            display_name: sk.display_name.clone(),
            summary: sk.description.clone(),
            tags: serde_json::json!({ "latest": ver }),
            stats: serde_json::json!({ "installs": sk.install_count }),
            created_at: ms(sk.created_at),
            updated_at: ms(sk.updated_at),
        }),
        latest_version: Some(VersionRef {
            version: ver,
            created_at: ms(ver_at),
            changelog: String::new(),
            license: coerce_license(&sk.manifest),
        }),
        owner: None,
        moderation: None,
    }))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct VersionItem {
    version: String,
    created_at: i64,
    changelog: String,
    changelog_source: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct VersionListResponse {
    items: Vec<VersionItem>,
    next_cursor: Option<String>,
}

async fn version_list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(slug): Path<String>,
) -> Result<Json<VersionListResponse>, ApiError> {
    let principal = optional_principal(&state, &headers);
    let sk = resolve_skill(&state, &slug, &principal).await?;
    let rows = sqlx::query(
        "SELECT version, published_at FROM skill_versions \
         WHERE skill_id = $1 ORDER BY published_at DESC LIMIT 200",
    )
    .bind(sk.id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| DomainError::Internal(e.to_string()))?;

    let items: Vec<VersionItem> = if rows.is_empty() {
        let (ver, at) = latest_version(&state, &sk).await;
        vec![VersionItem {
            version: ver,
            created_at: ms(at),
            changelog: String::new(),
            changelog_source: None,
        }]
    } else {
        rows.iter()
            .map(|r| VersionItem {
                version: r.get("version"),
                created_at: ms(r.get::<DateTime<Utc>, _>("published_at")),
                changelog: String::new(),
                changelog_source: None,
            })
            .collect()
    };
    Ok(Json(VersionListResponse { items, next_cursor: None }))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FileEntry {
    path: String,
    size: i64,
    sha256: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct VersionFull {
    version: String,
    created_at: i64,
    changelog: String,
    license: Option<String>,
    files: Vec<FileEntry>,
    security: serde_json::Value,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct VersionSkillRef {
    slug: String,
    display_name: String,
}

#[derive(Serialize)]
struct VersionDetailResponse {
    version: VersionFull,
    skill: VersionSkillRef,
}

fn materialized_files(sk: &SkillRow) -> Vec<FileEntry> {
    let mut files = vec![FileEntry {
        path: "SKILL.md".into(),
        size: sk.readme.as_deref().unwrap_or("").len() as i64,
        sha256: None,
    }];
    let manifest_str = serde_json::to_string_pretty(&sk.manifest).unwrap_or_default();
    files.push(FileEntry {
        path: "manifest.json".into(),
        size: manifest_str.len() as i64,
        sha256: None,
    });
    files
}

async fn version_detail(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((slug, _version)): Path<(String, String)>,
) -> Result<Json<VersionDetailResponse>, ApiError> {
    let principal = optional_principal(&state, &headers);
    let sk = resolve_skill(&state, &slug, &principal).await?;
    let (ver, at) = latest_version(&state, &sk).await;
    Ok(Json(VersionDetailResponse {
        version: VersionFull {
            version: ver,
            created_at: ms(at),
            changelog: String::new(),
            license: coerce_license(&sk.manifest),
            files: materialized_files(&sk),
            security: serde_json::json!({ "status": "clean", "hasWarnings": false }),
        },
        skill: VersionSkillRef {
            slug: sk.slug.clone(),
            display_name: sk.display_name.clone(),
        },
    }))
}

#[derive(Deserialize)]
struct FileParams {
    path: String,
}

async fn skill_file(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(slug): Path<String>,
    Query(p): Query<FileParams>,
) -> Result<impl IntoResponse, ApiError> {
    let principal = optional_principal(&state, &headers);
    let sk = resolve_skill(&state, &slug, &principal).await?;
    let body = match p.path.as_str() {
        "SKILL.md" => sk.readme.clone().unwrap_or_default(),
        "manifest.json" => serde_json::to_string_pretty(&sk.manifest).unwrap_or_default(),
        other => return Err(DomainError::NotFound(format!("file {other}")).into()),
    };
    Ok(([(header::CONTENT_TYPE, "text/plain; charset=utf-8")], body))
}

#[derive(Deserialize)]
struct DownloadParams {
    slug: String,
}

async fn download(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(p): Query<DownloadParams>,
) -> Result<impl IntoResponse, ApiError> {
    let principal = optional_principal(&state, &headers);
    let sk = resolve_skill(&state, &p.slug, &principal).await?;

    // Count the install (same semantics as the old /cli surface).
    let _ = sqlx::query(
        "UPDATE skills SET install_count = install_count + 1, downloads = downloads + 1 WHERE id = $1",
    )
    .bind(sk.id)
    .execute(&state.pool)
    .await;

    let readme = sk.readme.clone().unwrap_or_default();
    let manifest = serde_json::to_string_pretty(&sk.manifest).unwrap_or_default();

    let zipped = build_zip(&[("SKILL.md", &readme), ("manifest.json", &manifest)])
        .map_err(|e| DomainError::Internal(format!("zip: {e}")))?;

    Ok((
        [
            (header::CONTENT_TYPE, "application/zip".to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{}.zip\"", sk.slug),
            ),
        ],
        zipped,
    ))
}

fn build_zip(files: &[(&str, &str)]) -> zip::result::ZipResult<Vec<u8>> {
    let mut zw = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let opts: zip::write::SimpleFileOptions =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for (name, content) in files {
        zw.start_file(*name, opts)?;
        zw.write_all(content.as_bytes())?;
    }
    Ok(zw.finish()?.into_inner())
}

#[derive(Deserialize)]
struct SearchParams {
    q: Option<String>,
    limit: Option<i64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SearchItem {
    slug: String,
    display_name: String,
    summary: Option<String>,
    version: Option<String>,
    score: f32,
    updated_at: i64,
}

#[derive(Serialize)]
struct SearchResponse {
    results: Vec<SearchItem>,
}

async fn search(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(params): Query<SearchParams>,
) -> Result<Json<SearchResponse>, ApiError> {
    let principal = optional_principal(&state, &headers);
    let (is_super, uid) = vis_binds(&principal);
    let limit = params.limit.unwrap_or(20).clamp(1, 200);
    let q = params.q.unwrap_or_default();
    let q = q.trim();

    let rows = if q.is_empty() {
        let sql = format!(
            "SELECT s.slug, s.display_name, s.description, s.updated_at, s.manifest \
             FROM skills s JOIN namespaces n ON n.id = s.namespace_id \
             WHERE {} ORDER BY s.install_count DESC LIMIT $3",
            crate::api::authz::vis_predicate(1, 2)
        );
        sqlx::query(&sql)
            .bind(is_super)
            .bind(uid)
            .bind(limit)
            .fetch_all(&state.pool)
            .await
    } else {
        let sql = format!(
            "SELECT s.slug, s.display_name, s.description, s.updated_at, s.manifest, \
                    ts_rank_cd(s.search_vector, plainto_tsquery('simple', $1)) AS rank \
             FROM skills s JOIN namespaces n ON n.id = s.namespace_id \
             WHERE s.search_vector @@ plainto_tsquery('simple', $1) AND {} \
             ORDER BY rank DESC, s.install_count DESC LIMIT $4",
            crate::api::authz::vis_predicate(2, 3)
        );
        sqlx::query(&sql)
            .bind(q)
            .bind(is_super)
            .bind(uid)
            .bind(limit)
            .fetch_all(&state.pool)
            .await
    }
    .map_err(|e| DomainError::Internal(e.to_string()))?;

    let results = rows
        .iter()
        .map(|r| SearchItem {
            slug: r.get("slug"),
            display_name: r.get("display_name"),
            summary: r.get("description"),
            version: manifest_version(&r.get::<serde_json::Value, _>("manifest")),
            score: r.try_get("rank").unwrap_or(0.0),
            updated_at: ms(r.get::<DateTime<Utc>, _>("updated_at")),
        })
        .collect();
    Ok(Json(SearchResponse { results }))
}

#[derive(Deserialize)]
struct ResolveParams {
    slug: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ResolveResponse {
    #[serde(rename = "match")]
    matched: Option<serde_json::Value>,
    latest_version: Option<serde_json::Value>,
}

async fn resolve(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(p): Query<ResolveParams>,
) -> Result<Json<ResolveResponse>, ApiError> {
    let principal = optional_principal(&state, &headers);
    let sk = resolve_skill(&state, &p.slug, &principal).await?;
    let (ver, _) = latest_version(&state, &sk).await;
    Ok(Json(ResolveResponse {
        matched: None,
        latest_version: Some(serde_json::json!({ "version": ver })),
    }))
}

async fn whoami(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let p = optional_principal(&state, &headers).ok_or(DomainError::Unauthorized)?;
    Ok(Json(serde_json::json!({ "handle": p.username, "id": p.user_id })))
}
