//! Resolve a `Principal` from incoming headers.
//!
//! Three sources are accepted, checked in order:
//!  1. `Authorization: Bearer <jwt>` — decoded HS256 session token.
//!  2. `Authorization: ApiToken <conf>_<prefix>_<secret>` — server-managed API token.
//!  3. `X-Mock-User-Id: <uuid>` — dev-only convenience, gated by the
//!     `local` profile in spirit (kept for the demo identity gate / E2E).

use std::sync::Arc;

use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
};
use skillhub_auth::{jwt, token, Principal, Role};
use sqlx::Row;

use crate::state::AppState;

pub struct AuthPrincipal(pub Principal);

fn role_from_str(s: &str) -> Role {
    match s {
        "super_admin" => Role::SuperAdmin,
        _ => Role::User,
    }
}

fn unauthorized(msg: &str) -> Response {
    (StatusCode::UNAUTHORIZED, msg.to_string()).into_response()
}

#[async_trait]
impl FromRequestParts<Arc<AppState>> for AuthPrincipal {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        // 1 + 2: Authorization header.
        if let Some(auth) = parts
            .headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
        {
            if let Some(tok) = auth.strip_prefix("Bearer ") {
                let claims = jwt::verify(&state.config.auth.jwt_secret, tok.trim())
                    .map_err(|_| unauthorized("invalid or expired token"))?;
                let user_id = claims
                    .sub
                    .parse()
                    .map_err(|_| unauthorized("malformed token subject"))?;
                return Ok(Self(Principal {
                    user_id: Some(user_id),
                    username: Some(claims.username),
                    role: role_from_str(&claims.role),
                    scopes: vec![],
                }));
            }

            if let Some(tok) = auth.strip_prefix("ApiToken ") {
                let principal = verify_api_token(state, tok.trim()).await?;
                return Ok(Self(principal));
            }
        }

        // 3: dev mock header — only honored when explicitly enabled in config.
        // In any non-local deployment this stays off, so the header is ignored
        // and the request falls through to Unauthorized.
        if state.config.auth.allow_mock_header {
            if let Some(uid) = parts
                .headers
                .get("x-mock-user-id")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| uuid::Uuid::parse_str(s).ok())
            {
                return Ok(Self(Principal {
                    user_id: Some(uid),
                    username: parts
                        .headers
                        .get("x-mock-username")
                        .and_then(|v| v.to_str().ok())
                        .map(|s| s.to_string()),
                    role: Role::User,
                    scopes: vec![],
                }));
            }
        }

        Err(unauthorized("missing credentials"))
    }
}

async fn verify_api_token(state: &Arc<AppState>, presented: &str) -> Result<Principal, Response> {
    let prefix = token::parse_prefix(presented).ok_or_else(|| unauthorized("malformed api token"))?;

    let row = sqlx::query(
        r#"SELECT t.id, t.user_id, t.hash, t.scopes, t.expires_at,
                  u.username, u.is_super_admin
           FROM api_tokens t JOIN users u ON u.id = t.user_id
           WHERE t.prefix = $1"#,
    )
    .bind(&prefix)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?
    .ok_or_else(|| unauthorized("unknown api token"))?;

    let stored_hash: String = row.get("hash");
    if !token::verify(presented, &stored_hash) {
        return Err(unauthorized("invalid api token"));
    }

    let expires_at: Option<chrono::DateTime<chrono::Utc>> = row.get("expires_at");
    if let Some(exp) = expires_at {
        if exp < chrono::Utc::now() {
            return Err(unauthorized("api token expired"));
        }
    }

    // Best-effort last-used stamp; ignore failures.
    let token_id: uuid::Uuid = row.get("id");
    let _ = sqlx::query("UPDATE api_tokens SET last_used_at = now() WHERE id = $1")
        .bind(token_id)
        .execute(&state.pool)
        .await;

    let is_super: bool = row.get("is_super_admin");
    let scopes: Vec<String> = row.get("scopes");
    Ok(Principal {
        user_id: Some(row.get("user_id")),
        username: Some(row.get("username")),
        role: if is_super { Role::SuperAdmin } else { Role::User },
        scopes,
    })
}
