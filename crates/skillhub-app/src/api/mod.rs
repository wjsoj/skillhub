//! HTTP API surface. Routes are grouped by resource and mounted
//! under `/api/v1`. The CLI-compat surface lives under `/cli`.

use std::sync::Arc;

use axum::{routing::get, Json, Router};
use serde_json::json;
use tower_http::{cors::CorsLayer, services::{ServeDir, ServeFile}, trace::TraceLayer};

use crate::state::AppState;

mod auth;
mod namespaces;
mod skills;
mod versions;
mod search;
mod reviews;
mod tokens;
mod users;
mod admin;
mod compat;

mod duplicate;
mod collaborators;
mod orgs;
mod iterations;
mod proposals;
mod meta;
mod authz;

pub fn router(state: Arc<AppState>) -> Router {
    let skills_routes = skills::routes()
        .merge(duplicate::routes())
        .merge(collaborators::routes())
        .merge(iterations::routes())
        .merge(proposals::routes());

    let v1 = Router::new()
        .nest("/auth", auth::routes())
        .nest("/namespaces", namespaces::routes())
        .nest("/skills", skills_routes)
        .nest("/versions", versions::routes())
        .nest("/search", search::routes())
        .nest("/reviews", reviews::routes())
        .nest("/tokens", tokens::routes())
        .nest("/users", users::routes())
        .nest("/admin", admin::routes())
        .nest("/meta", meta::routes())
        .merge(orgs::routes());

    // Serve the React build from web/dist if it exists. The SPA's
    // client-side router (TanStack) owns every non-/api path, so the
    // not-found handler falls back to index.html.
    let web_root = std::env::var("SKILLHUB_WEB_DIST")
        .unwrap_or_else(|_| "web/dist".to_string());
    let serve_dir =
        ServeDir::new(&web_root).fallback(ServeFile::new(format!("{}/index.html", &web_root)));

    Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .merge(meta::root_routes())
        .nest("/api/v1", v1)
        .nest("/cli", compat::routes())
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
        .fallback_service(serve_dir)
}

async fn healthz() -> Json<serde_json::Value> {
    Json(json!({ "status": "ok" }))
}

async fn readyz() -> Json<serde_json::Value> {
    Json(json!({ "status": "ready" }))
}
