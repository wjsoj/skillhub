//! Resolve a `Principal` from incoming headers.
//!
//! Three sources are accepted, checked in order:
//!  1. `Authorization: Bearer <jwt>` — decoded session token.
//!  2. `Authorization: ApiToken sk_<prefix>_<secret>` — server-managed API token.
//!  3. `X-Mock-User-Id: <uuid>` — dev-only, gated by `local` profile.
//!
//! For now this is a stub that only handles the dev header; JWT and
//! API-token verification will land alongside the auth crate impls.

use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
};
use skillhub_auth::Principal;

pub struct AuthPrincipal(pub Principal);

#[async_trait]
impl<S> FromRequestParts<S> for AuthPrincipal
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
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
                role: skillhub_auth::Role::User,
                scopes: vec![],
            }));
        }
        Err((StatusCode::UNAUTHORIZED, "missing credentials").into_response())
    }
}
