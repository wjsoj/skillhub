use std::sync::Arc;
use axum::Router;
use crate::state::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
}
