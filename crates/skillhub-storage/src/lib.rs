//! Pluggable object storage for skill packages.
//!
//! `ObjectStore` is the trait every backend implements. `local`
//! stores files under a root directory (dev). `s3` targets S3 or
//! MinIO (production). `policy` validates uploaded packages
//! against the publish allowlist.

use async_trait::async_trait;
use bytes::Bytes;

pub mod local;
pub mod s3;
pub mod policy;

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("not found: {0}")]
    NotFound(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("backend error: {0}")]
    Backend(String),
}

pub type StorageResult<T> = Result<T, StorageError>;

#[async_trait]
pub trait ObjectStore: Send + Sync {
    async fn put(&self, key: &str, body: Bytes, content_type: &str) -> StorageResult<()>;
    async fn get(&self, key: &str) -> StorageResult<Bytes>;
    async fn delete(&self, key: &str) -> StorageResult<()>;
    async fn presign_get(&self, key: &str, ttl_secs: u64) -> StorageResult<String>;
}
