//! SQLx-backed repository implementations.
//!
//! Each module here implements one of the traits declared in
//! `skillhub-domain`. Keep query strings here so the domain layer
//! stays free of SQL.

pub mod skill_repo;
pub mod namespace_repo;
pub mod user_repo;
pub mod token_repo;
pub mod review_repo;
pub mod audit_repo;

pub mod department_repo;
pub mod collaborator_repo;
pub mod proposal_repo;
pub mod iteration_repo;
pub mod activity_repo;
pub mod embedding_repo;
