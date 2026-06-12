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

use crate::error::ApiError;
use crate::state::AppState;
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
    email: Option<String>,
    display_name: Option<String>,
    is_super_admin: bool,
    created_at: DateTime<Utc>,
}

fn row_to_dto(r: &sqlx::postgres::PgRow) -> UserDto {
    UserDto {
        id: r.get("id"),
        username: r.get("username"),
        email: r.get("email"),
        display_name: r.get("display_name"),
        is_super_admin: r.get("is_super_admin"),
        created_at: r.get("created_at"),
    }
}

async fn list_users(State(state): State<Arc<AppState>>) -> Result<Json<Vec<UserDto>>, ApiError> {
    let rows = sqlx::query(
        r#"SELECT id, username, email, display_name, is_super_admin, created_at
           FROM users ORDER BY username ASC"#,
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|e| DomainError::Internal(e.to_string()))?;
    Ok(Json(rows.iter().map(row_to_dto).collect()))
}

async fn get_user(
    State(state): State<Arc<AppState>>,
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
    Ok(Json(row_to_dto(&row)))
}
