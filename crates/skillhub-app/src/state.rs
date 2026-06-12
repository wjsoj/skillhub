use std::sync::Arc;

use skillhub_auth::PolicyEvaluator;
use skillhub_domain::activity::ActivityRepository;
use skillhub_domain::collaborator::CollaboratorRepository;
use skillhub_domain::department::{
    CrossScopeGrantRepository, DepartmentMembershipRepository, DepartmentRepository,
};
use skillhub_domain::embedding::EmbeddingRepository;
use skillhub_domain::iteration::IterationRepository;
use skillhub_domain::proposal::{DraftRepository, ProposalRepository};
use skillhub_embeddings::Embedder;
use skillhub_harness::Harness;
use skillhub_infra::{AppConfig, PgPool, RedisClient};
use skillhub_search::DuplicateDetector;

/// Everything the HTTP layer needs. Built once at startup, cloned via
/// `Arc` into every request. New services (embedder, harness, policy,
/// duplicate detector, repos) live here so handlers can pluck what
/// they need by destructuring.
#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub pool: PgPool,
    pub redis: RedisClient,

    pub policy: PolicyEvaluator,
    pub embedder: Arc<dyn Embedder>,
    pub duplicate_detector: Arc<DuplicateDetector>,
    pub harness: Arc<Harness>,

    pub departments: Arc<dyn DepartmentRepository>,
    pub department_memberships: Arc<dyn DepartmentMembershipRepository>,
    pub cross_grants: Arc<dyn CrossScopeGrantRepository>,
    pub collaborators: Arc<dyn CollaboratorRepository>,
    pub drafts: Arc<dyn DraftRepository>,
    pub proposals: Arc<dyn ProposalRepository>,
    pub iterations: Arc<dyn IterationRepository>,
    pub activity: Arc<dyn ActivityRepository>,
    pub embeddings: Arc<dyn EmbeddingRepository>,
}
