//! Agent-facing self-description surface.
//!
//! SkillHub treats agents as first-class API consumers. A thin connector
//! skill bootstraps them to these endpoints; the docs themselves are
//! compiled into the binary and rendered with this deployment's base URL,
//! so they can never drift from what the running server implements:
//!
//!   GET /llms.txt              — tiny discovery index (llms.txt convention)
//!   GET /agents.md             — full agent guide (markdown)
//!   GET /skill.md              — installable connector skill (SKILL.md)
//!   GET /api/v1/meta/manifest  — machine-readable capability manifest

use std::sync::Arc;

use axum::{
    extract::State,
    http::header,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde_json::json;

use crate::state::AppState;

const AGENT_GUIDE: &str = include_str!("../../../../docs/agent-guide.md");
const CONNECTOR_SKILL: &str = include_str!("../../../../skill/skillhub/SKILL.md");

/// Discovery docs mounted at the server root.
pub fn root_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/llms.txt", get(llms_txt))
        .route("/agents.md", get(agents_md))
        .route("/skill.md", get(skill_md))
}

/// Mounted under `/api/v1/meta`.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/manifest", get(manifest))
}

fn base_url(state: &AppState) -> &str {
    state.config.server.public_base_url.trim_end_matches('/')
}

const MARKDOWN: (header::HeaderName, &str) =
    (header::CONTENT_TYPE, "text/markdown; charset=utf-8");
const PLAIN: (header::HeaderName, &str) =
    (header::CONTENT_TYPE, "text/plain; charset=utf-8");

async fn llms_txt(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let base = base_url(&state);
    let body = format!(
        "# SkillHub\n\n\
         > Self-hosted agent skill registry: publish, version, review, discover and\n\
         > install agent skills. Agents are first-class API consumers here.\n\n\
         ## Docs\n\n\
         - [Agent Guide]({base}/agents.md): complete API guide written for agents — auth, endpoints, workflows\n\
         - [Capability Manifest]({base}/api/v1/meta/manifest): machine-readable auth modes + implemented capabilities\n\
         - [Connector Skill]({base}/skill.md): installable SKILL.md that teaches an agent to use this registry\n\n\
         ## API\n\n\
         - [Health]({base}/healthz): liveness probe\n\
         - [Skills catalog]({base}/api/v1/skills): public, no auth required\n"
    );
    ([PLAIN], body)
}

async fn agents_md(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    ([MARKDOWN], AGENT_GUIDE.replace("{{BASE_URL}}", base_url(&state)))
}

async fn skill_md(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    ([MARKDOWN], CONNECTOR_SKILL.replace("{{BASE_URL}}", base_url(&state)))
}

/// The runtime truth about what this instance supports. Agents are told to
/// trust this over any cached copy of the guide, so keep it in sync with
/// the routers in this module's siblings.
async fn manifest(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let base = base_url(&state);
    Json(json!({
        "name": "skillhub",
        "version": env!("CARGO_PKG_VERSION"),
        "api_version": "v1",
        "base_url": base,
        "docs": {
            "index": format!("{base}/llms.txt"),
            "agent_guide": format!("{base}/agents.md"),
            "connector_skill": format!("{base}/skill.md"),
        },
        "auth": {
            "modes": ["mock"],
            "planned": ["jwt", "api_token"],
            "headers": {
                "mock": "X-Mock-User-Id: <user uuid> (optional X-Mock-Username)",
                "jwt": "Authorization: Bearer <jwt>",
                "api_token": "Authorization: ApiToken sk_<prefix>_<secret>",
            },
        },
        "capabilities": {
            "skills_catalog": true,
            "duplicate_check": true,
            "collaborators": true,
            "drafts_proposals": true,
            "iterations": true,
            "departments_grants": true,
            "admin_reindex": true,
            "search": false,
            "auth_endpoints": false,
            "api_tokens": false,
            "users": false,
            "versions": false,
            "reviews": false,
            "namespaces": false,
            "cli_compat": false,
        },
        "embedding": {
            "model": state.embedder.model(),
            "dim": state.embedder.dim(),
        },
    }))
}
