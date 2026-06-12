//! /api/v1/tokens — personal API tokens for the authenticated user.
//!
//! The plaintext token is returned exactly once, on creation. Afterwards
//! only its prefix and SHA-256 hash are stored.

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use crate::error::ApiError;
use crate::middleware::AuthPrincipal;
use crate::state::AppState;
use skillhub_auth::token;
use skillhub_domain::DomainError;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list_tokens).post(create_token))
        .route("/:id", axum::routing::delete(revoke_token))
}

#[derive(Debug, Serialize)]
struct TokenDto {
    id: Uuid,
    name: String,
    prefix: String,
    scopes: Vec<String>,
    expires_at: Option<DateTime<Utc>>,
    last_used_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

fn row_to_dto(r: &sqlx::postgres::PgRow) -> TokenDto {
    TokenDto {
        id: r.get("id"),
        name: r.get("name"),
        prefix: r.get("prefix"),
        scopes: r.get("scopes"),
        expires_at: r.get("expires_at"),
        last_used_at: r.get("last_used_at"),
        created_at: r.get("created_at"),
    }
}

async fn list_tokens(
    State(state): State<Arc<AppState>>,
    AuthPrincipal(principal): AuthPrincipal,
) -> Result<Json<Vec<TokenDto>>, ApiError> {
    let uid = principal.user_id.ok_or(DomainError::Unauthorized)?;
    let rows = sqlx::query(
        r#"SELECT id, name, prefix, scopes, expires_at, last_used_at, created_at
           FROM api_tokens WHERE user_id = $1 ORDER BY created_at DESC"#,
    )
    .bind(uid)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| DomainError::Internal(e.to_string()))?;
    Ok(Json(rows.iter().map(row_to_dto).collect()))
}

#[derive(Debug, Deserialize)]
struct CreateBody {
    name: String,
    #[serde(default)]
    scopes: Vec<String>,
    expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
struct CreatedToken {
    id: Uuid,
    name: String,
    prefix: String,
    /// Shown once — store it now, it cannot be retrieved later.
    token: String,
    scopes: Vec<String>,
    expires_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

async fn create_token(
    State(state): State<Arc<AppState>>,
    AuthPrincipal(principal): AuthPrincipal,
    Json(body): Json<CreateBody>,
) -> Result<Json<CreatedToken>, ApiError> {
    let uid = principal.user_id.ok_or(DomainError::Unauthorized)?;
    let name = body.name.trim();
    if name.is_empty() {
        return Err(DomainError::Validation("token name is required".into()).into());
    }
    let minted = token::generate(&state.config.auth.token_prefix);

    let row = sqlx::query(
        r#"INSERT INTO api_tokens (user_id, name, prefix, hash, scopes, expires_at)
           VALUES ($1, $2, $3, $4, $5, $6)
           RETURNING id, created_at"#,
    )
    .bind(uid)
    .bind(name)
    .bind(&minted.prefix)
    .bind(&minted.hash)
    .bind(&body.scopes)
    .bind(body.expires_at)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| DomainError::Internal(e.to_string()))?;

    Ok(Json(CreatedToken {
        id: row.get("id"),
        name: name.to_string(),
        prefix: minted.prefix,
        token: minted.plaintext,
        scopes: body.scopes,
        expires_at: body.expires_at,
        created_at: row.get("created_at"),
    }))
}

async fn revoke_token(
    State(state): State<Arc<AppState>>,
    AuthPrincipal(principal): AuthPrincipal,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let uid = principal.user_id.ok_or(DomainError::Unauthorized)?;
    let res = sqlx::query("DELETE FROM api_tokens WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(uid)
        .execute(&state.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;
    if res.rows_affected() == 0 {
        return Err(DomainError::NotFound(format!("token {id}")).into());
    }
    Ok(Json(serde_json::json!({ "revoked": true })))
}
