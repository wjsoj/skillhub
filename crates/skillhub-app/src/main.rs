use std::sync::Arc;

use anyhow::Context;
use axum::Router;
use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

mod api;
mod error;
mod middleware;
mod state;

use skillhub_auth::PolicyEvaluator;
use skillhub_embeddings::{Embedder, HttpEmbedder, StubEmbedder, DEFAULT_DIM};
use skillhub_harness::{Harness, HarnessConfig};
use skillhub_infra::repo::activity_repo::PgActivityRepo;
use skillhub_infra::repo::collaborator_repo::PgCollaboratorRepo;
use skillhub_infra::repo::department_repo::{
    PgCrossScopeGrantRepo, PgDepartmentMembershipRepo, PgDepartmentRepo,
};
use skillhub_infra::repo::embedding_repo::PgEmbeddingRepo;
use skillhub_infra::repo::iteration_repo::PgIterationRepo;
use skillhub_infra::repo::proposal_repo::{PgDraftRepo, PgProposalRepo};
use skillhub_infra::{init_pool, run_migrations, AppConfig, PgPool, RedisClient};
use skillhub_search::DuplicateDetector;
use state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    let config = AppConfig::from_env().context("loading configuration")?;
    tracing::info!(host = %config.server.host, port = config.server.port, "starting skillhub");

    let pool = init_pool(&config.database.url, config.database.max_connections).await?;
    run_migrations(&pool).await?;
    bootstrap_admin(&pool, &config).await?;

    let redis = RedisClient::connect(&config.redis.url).await?;

    let embedder: Arc<dyn Embedder> = match std::env::var("SKILLHUB__EMBEDDER__URL").ok() {
        Some(url) => {
            let model = std::env::var("SKILLHUB__EMBEDDER__MODEL")
                .unwrap_or_else(|_| "text-embedding-3-small".into());
            let mut e = HttpEmbedder::new(url, model, DEFAULT_DIM);
            if let Ok(k) = std::env::var("SKILLHUB__EMBEDDER__API_KEY") {
                e = e.with_api_key(k);
            }
            Arc::new(e)
        }
        None => {
            tracing::warn!("no SKILLHUB__EMBEDDER__URL set; using deterministic stub embedder");
            Arc::new(StubEmbedder::new())
        }
    };

    let embeddings = Arc::new(PgEmbeddingRepo { pool: pool.clone() });
    let duplicate_detector = Arc::new(DuplicateDetector::new(
        embedder.clone(),
        embeddings.clone(),
        20,
    ));

    let harness = Arc::new(Harness::new(HarnessConfig::default()));

    let state = Arc::new(AppState {
        config: config.clone(),
        pool: pool.clone(),
        redis,
        policy: PolicyEvaluator::new(),
        embedder,
        duplicate_detector,
        harness,
        departments: Arc::new(PgDepartmentRepo { pool: pool.clone() }),
        department_memberships: Arc::new(PgDepartmentMembershipRepo { pool: pool.clone() }),
        cross_grants: Arc::new(PgCrossScopeGrantRepo { pool: pool.clone() }),
        collaborators: Arc::new(PgCollaboratorRepo { pool: pool.clone() }),
        drafts: Arc::new(PgDraftRepo { pool: pool.clone() }),
        proposals: Arc::new(PgProposalRepo { pool: pool.clone() }),
        iterations: Arc::new(PgIterationRepo { pool: pool.clone() }),
        activity: Arc::new(PgActivityRepo { pool: pool.clone() }),
        embeddings,
    });

    let app: Router = api::router(state.clone());

    let addr = format!("{}:{}", config.server.host, config.server.port);
    let listener = TcpListener::bind(&addr).await?;
    tracing::info!(%addr, "listening");
    axum::serve(listener, app).await?;

    Ok(())
}

/// Ensure a usable super-admin account exists with a password, so a fresh
/// deployment can be logged into out of the box. Idempotent: re-running
/// only refreshes the admin's password hash and super-admin flag.
async fn bootstrap_admin(pool: &PgPool, config: &AppConfig) -> anyhow::Result<()> {
    let auth = &config.auth;
    if !auth.bootstrap_admin_enabled {
        return Ok(());
    }
    let hash = skillhub_auth::password::hash_password(&auth.bootstrap_admin_password)?;
    sqlx::query(
        r#"INSERT INTO users (username, email, display_name, is_super_admin, password_hash)
           VALUES ($1, $2, 'Platform Admin', true, $3)
           ON CONFLICT (username) DO UPDATE
             SET is_super_admin = true, password_hash = EXCLUDED.password_hash"#,
    )
    .bind(&auth.bootstrap_admin_username)
    .bind(format!("{}@local", auth.bootstrap_admin_username))
    .bind(&hash)
    .execute(pool)
    .await?;
    tracing::info!(user = %auth.bootstrap_admin_username, "bootstrapped super-admin account");
    Ok(())
}

fn init_tracing() {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(tracing_subscriber::fmt::layer())
        .init();
}
