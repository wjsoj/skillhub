use axum::{http::StatusCode, response::{IntoResponse, Response}, Json};
use serde_json::json;
use skillhub_domain::DomainError;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error(transparent)]
    Domain(#[from] DomainError),
    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, msg) = match &self {
            ApiError::Domain(DomainError::NotFound(m)) => (StatusCode::NOT_FOUND, m.clone()),
            ApiError::Domain(DomainError::AlreadyExists(m)) => (StatusCode::CONFLICT, m.clone()),
            ApiError::Domain(DomainError::Validation(m)) => (StatusCode::BAD_REQUEST, m.clone()),
            ApiError::Domain(DomainError::Forbidden(m)) => (StatusCode::FORBIDDEN, m.clone()),
            ApiError::Domain(DomainError::Unauthorized) => (StatusCode::UNAUTHORIZED, "unauthorized".into()),
            ApiError::Domain(DomainError::Conflict(m)) => (StatusCode::CONFLICT, m.clone()),
            ApiError::Domain(DomainError::Internal(m)) => (StatusCode::INTERNAL_SERVER_ERROR, m.clone()),
            ApiError::Anyhow(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        };
        (status, Json(json!({ "error": msg }))).into_response()
    }
}
