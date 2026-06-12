//! Authentication & authorization.
//!
//! - Password hashing (argon2)
//! - API token issue / verify (prefix + sha256 hash)
//! - JWT session tokens
//! - OAuth2 device-code & web flows
//! - RBAC: SUPER_ADMIN / namespace roles

pub mod password;
pub mod token;
pub mod jwt;
pub mod oauth;
pub mod rbac;
pub mod principal;
pub mod policy;

pub use principal::{Principal, Role};
pub use policy::{Action, Decision, PermissionCtx, PolicyEvaluator, Target, TargetKind};
