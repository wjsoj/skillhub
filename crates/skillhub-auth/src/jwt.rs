//! JWT session tokens (HS256).

use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Default session lifetime.
pub const DEFAULT_TTL_HOURS: i64 = 24 * 7;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject — the user id.
    pub sub: String,
    pub username: String,
    /// "super_admin" | "user" | "service_account".
    pub role: String,
    /// Issued-at (unix seconds).
    pub iat: i64,
    /// Expiry (unix seconds).
    pub exp: i64,
}

/// Sign a session token for a user.
pub fn issue(
    secret: &str,
    user_id: Uuid,
    username: &str,
    role: &str,
    ttl_hours: i64,
) -> anyhow::Result<String> {
    let now = Utc::now();
    let claims = Claims {
        sub: user_id.to_string(),
        username: username.to_string(),
        role: role.to_string(),
        iat: now.timestamp(),
        exp: (now + Duration::hours(ttl_hours)).timestamp(),
    };
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )?;
    Ok(token)
}

/// Verify and decode a session token. Returns the claims if valid and unexpired.
pub fn verify(secret: &str, token: &str) -> anyhow::Result<Claims> {
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )?;
    Ok(data.claims)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issue_then_verify_roundtrips() {
        let id = Uuid::new_v4();
        let t = issue("test-secret", id, "ada", "user", 1).unwrap();
        let c = verify("test-secret", &t).unwrap();
        assert_eq!(c.sub, id.to_string());
        assert_eq!(c.username, "ada");
        assert_eq!(c.role, "user");
        assert!(verify("wrong-secret", &t).is_err());
    }
}
