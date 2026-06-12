//! Infrastructure: configuration, database pool, redis, migrations,
//! and concrete SQLx-backed repository implementations.

pub mod config;
pub mod db;
pub mod cache;
pub mod repo;

pub use config::AppConfig;
pub use db::{PgPool, init_pool, run_migrations};
pub use cache::RedisClient;
