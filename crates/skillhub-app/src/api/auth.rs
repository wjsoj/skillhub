//! /api/v1/auth — registration, login, and current-identity.

use std::sync::Arc;

use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use crate::error::ApiError;
use crate::middleware::AuthPrincipal;
use crate::state::AppState;
use skillhub_auth::{jwt, password};
use skillhub_domain::DomainError;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/me", get(me))
}

#[derive(Debug, Serialize)]
pub struct UserDto {
    pub id: Uuid,
    pub username: String,
    pub email: Option<String>,
    pub display_name: Option<String>,
    pub is_super_admin: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct AuthResponse {
    token: String,
    user: UserDto,
}

fn role_str(is_super: bool) -> &'static str {
    if is_super {
        "super_admin"
    } else {
        "user"
    }
}

fn issue_for(state: &AppState, u: &UserDto) -> Result<String, ApiError> {
    jwt::issue(
        &state.config.auth.jwt_secret,
        u.id,
        &u.username,
        role_str(u.is_super_admin),
        jwt::DEFAULT_TTL_HOURS,
    )
    .map_err(|e| DomainError::Internal(e.to_string()).into())
}

fn row_to_user(r: &sqlx::postgres::PgRow) -> UserDto {
    UserDto {
        id: r.get("id"),
        username: r.get("username"),
        email: r.get("email"),
        display_name: r.get("display_name"),
        is_super_admin: r.get("is_super_admin"),
        created_at: r.get("created_at"),
    }
}

#[derive(Debug, Deserialize)]
struct RegisterBody {
    username: String,
    password: String,
    email: Option<String>,
    display_name: Option<String>,
}

async fn register(
    State(state): State<Arc<AppState>>,
    Json(body): Json<RegisterBody>,
) -> Result<Json<AuthResponse>, ApiError> {
    let username = body.username.trim().to_lowercase();
    if username.len() < 2 {
        return Err(DomainError::Validation("username too short".into()).into());
    }
    if body.password.len() < 8 {
        return Err(DomainError::Validation("password must be at least 8 characters".into()).into());
    }

    let exists = sqlx::query("SELECT 1 FROM users WHERE username = $1")
        .bind(&username)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;
    if exists.is_some() {
        return Err(DomainError::AlreadyExists(format!("username '{username}' is taken")).into());
    }

    let hash = password::hash_password(&body.password)
        .map_err(|e| DomainError::Internal(e.to_string()))?;

    let row = sqlx::query(
        r#"INSERT INTO users (username, email, display_name, password_hash)
           VALUES ($1, $2, $3, $4)
           RETURNING id, username, email, display_name, is_super_admin, created_at"#,
    )
    .bind(&username)
    .bind(body.email.as_deref().map(|s| s.trim()))
    .bind(
        body.display_name
            .as_deref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty()),
    )
    .bind(&hash)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| DomainError::Internal(e.to_string()))?;

    let user = row_to_user(&row);
    let token = issue_for(&state, &user)?;
    Ok(Json(AuthResponse { token, user }))
}

#[derive(Debug, Deserialize)]
struct LoginBody {
    username: String,
    password: String,
}

async fn login(
    State(state): State<Arc<AppState>>,
    Json(body): Json<LoginBody>,
) -> Result<Json<AuthResponse>, ApiError> {
    let username = body.username.trim().to_lowercase();
    let row = sqlx::query(
        r#"SELECT id, username, email, display_name, is_super_admin, created_at, password_hash
           FROM users WHERE username = $1"#,
    )
    .bind(&username)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| DomainError::Internal(e.to_string()))?
    .ok_or(DomainError::Unauthorized)?;

    let stored: Option<String> = row.get("password_hash");
    let stored = stored.ok_or(DomainError::Unauthorized)?;
    if !password::verify_password(&body.password, &stored) {
        return Err(DomainError::Unauthorized.into());
    }

    let user = row_to_user(&row);
    let token = issue_for(&state, &user)?;
    Ok(Json(AuthResponse { token, user }))
}

async fn me(
    State(state): State<Arc<AppState>>,
    AuthPrincipal(principal): AuthPrincipal,
) -> Result<Json<UserDto>, ApiError> {
    let id = principal.user_id.ok_or(DomainError::Unauthorized)?;
    let row = sqlx::query(
        r#"SELECT id, username, email, display_name, is_super_admin, created_at
           FROM users WHERE id = $1"#,
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| DomainError::Internal(e.to_string()))?
    .ok_or(DomainError::Unauthorized)?;
    Ok(Json(row_to_user(&row)))
}
