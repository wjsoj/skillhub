//! Core domain models, value objects, and repository traits.
//!
//! This crate is the pure-domain layer: no DB, no HTTP, no IO.
//! Other crates depend on the traits defined here and provide
//! concrete implementations.

pub mod error;
pub mod skill;
pub mod namespace;
pub mod user;
pub mod token;
pub mod review;
pub mod audit;

pub mod department;
pub mod collaborator;
pub mod proposal;
pub mod iteration;
pub mod activity;
pub mod embedding;

pub use error::{DomainError, DomainResult};
