//! Request middleware: principal extraction and department-scope hydration.

pub mod principal;
pub mod scope;

pub use principal::AuthPrincipal;
pub use scope::DeptScope;
