//! Text embedding abstraction for semantic duplicate detection.
//!
//! Two backends ship out of the box:
//! - [`HttpEmbedder`] — OpenAI-compatible HTTP endpoint (works with
//!   OpenAI, Ollama, vLLM, BGE, etc.). Configure via env / config.
//! - [`StubEmbedder`] — deterministic hash-folding into a fixed-dim
//!   vector. Offline / CI / unit-test friendly. **Not** semantically
//!   meaningful — use only when you don't have a real model wired up.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub mod http;
pub mod stub;
pub mod normalize;

pub use http::HttpEmbedder;
pub use normalize::SkillContent;
pub use stub::StubEmbedder;

pub const DEFAULT_DIM: usize = 1024;

#[derive(Debug, thiserror::Error)]
pub enum EmbedError {
    #[error("provider error: {0}")]
    Provider(String),
    #[error("network: {0}")]
    Network(#[from] reqwest::Error),
    #[error("dim mismatch: expected {expected}, got {got}")]
    DimMismatch { expected: usize, got: usize },
}

pub type EmbedResult<T> = Result<T, EmbedError>;

/// A single embedded document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Embedding {
    pub model: String,
    pub dim: usize,
    pub vector: Vec<f32>,
    pub content_hash: String,
}

impl Embedding {
    pub fn hash_of(text: &str) -> String {
        let mut h = Sha256::new();
        h.update(text.as_bytes());
        format!("{:x}", h.finalize())
    }
}

#[async_trait]
pub trait Embedder: Send + Sync {
    fn model(&self) -> &str;
    fn dim(&self) -> usize;
    async fn embed(&self, text: &str) -> EmbedResult<Embedding>;
    async fn embed_batch(&self, texts: &[String]) -> EmbedResult<Vec<Embedding>> {
        let mut out = Vec::with_capacity(texts.len());
        for t in texts {
            out.push(self.embed(t).await?);
        }
        Ok(out)
    }
}

/// Cosine similarity between two equal-length vectors. Returns 0.0 if
/// either has zero norm (caller decides how to treat that).
pub fn cosine(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }
    let mut dot = 0.0;
    let mut na = 0.0;
    let mut nb = 0.0;
    for (x, y) in a.iter().zip(b.iter()) {
        dot += x * y;
        na += x * x;
        nb += y * y;
    }
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    dot / (na.sqrt() * nb.sqrt())
}
