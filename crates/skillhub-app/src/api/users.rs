//! /api/v1/users — directory lookup (read-only).

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::Row;
use uuid::Uuid;

use crate::api::authz;
use crate::error::ApiError;
use crate::middleware::AuthPrincipal;
use crate::state::AppState;
use skillhub_auth::Principal;
use skillhub_domain::DomainError;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list_users))
        .route("/:id", get(get_user))
}

#[derive(Debug, Serialize)]
struct UserDto {
    id: Uuid,
    username: String,
    /// Only populated for the user themselves or a super-admin; otherwise null.
    email: Option<String>,
    display_name: Option<String>,
    is_super_admin: bool,
    created_at: DateTime<Utc>,
}

/// Map a row, redacting email unless the caller is allowed to see it.
fn row_to_dto(r: &sqlx::postgres::PgRow, caller: &Principal) -> UserDto {
    let id: Uuid = r.get("id");
    let may_see_email = authz::is_super(caller) || caller.user_id == Some(id);
    UserDto {
        id,
        username: r.get("username"),
        email: if may_see_email { r.get("email") } else { None },
        display_name: r.get("display_name"),
        is_super_admin: r.get("is_super_admin"),
        created_at: r.get("created_at"),
    }
}

async fn list_users(
    State(state): State<Arc<AppState>>,
    AuthPrincipal(principal): AuthPrincipal,
) -> Result<Json<Vec<UserDto>>, ApiError> {
    let rows = sqlx::query(
        r#"SELECT id, username, email, display_name, is_super_admin, created_at
           FROM users ORDER BY username ASC"#,
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|e| DomainError::Internal(e.to_string()))?;
    Ok(Json(rows.iter().map(|r| row_to_dto(r, &principal)).collect()))
}

async fn get_user(
    State(state): State<Arc<AppState>>,
    AuthPrincipal(principal): AuthPrincipal,
    Path(id): Path<Uuid>,
) -> Result<Json<UserDto>, ApiError> {
    let row = sqlx::query(
        r#"SELECT id, username, email, display_name, is_super_admin, created_at
           FROM users WHERE id = $1"#,
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| DomainError::Internal(e.to_string()))?
    .ok_or_else(|| DomainError::NotFound(format!("user {id}")))?;
    Ok(Json(row_to_dto(&row, &principal)))
}
