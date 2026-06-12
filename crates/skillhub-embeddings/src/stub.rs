//! Deterministic fallback embedder.
//!
//! Uses SHA-256 of the input as a seed; expands into `dim` floats via
//! a feistel-style hash counter then L2-normalises. Not semantic, but
//! stable across processes — good for tests and offline dev.

use async_trait::async_trait;
use sha2::{Digest, Sha256};

use crate::{EmbedResult, Embedder, Embedding, DEFAULT_DIM};

pub struct StubEmbedder {
    dim: usize,
    model: String,
}

impl StubEmbedder {
    pub fn new() -> Self {
        Self::with_dim(DEFAULT_DIM)
    }

    pub fn with_dim(dim: usize) -> Self {
        Self {
            dim,
            model: format!("stub-sha256-{}", dim),
        }
    }
}

impl Default for StubEmbedder {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Embedder for StubEmbedder {
    fn model(&self) -> &str {
        &self.model
    }
    fn dim(&self) -> usize {
        self.dim
    }

    async fn embed(&self, text: &str) -> EmbedResult<Embedding> {
        let content_hash = Embedding::hash_of(text);
        let mut seed = Sha256::digest(text.as_bytes()).to_vec();
        let mut vector = Vec::with_capacity(self.dim);
        let mut counter: u64 = 0;
        while vector.len() < self.dim {
            let mut h = Sha256::new();
            h.update(&seed);
            h.update(counter.to_le_bytes());
            let block = h.finalize();
            for chunk in block.chunks(4) {
                if vector.len() == self.dim {
                    break;
                }
                let bytes = [chunk[0], chunk[1], chunk[2], chunk[3]];
                // Map u32 → [-1, 1)
                let v = (u32::from_le_bytes(bytes) as f32 / u32::MAX as f32) * 2.0 - 1.0;
                vector.push(v);
            }
            seed = block.to_vec();
            counter += 1;
        }
        // L2 normalise so cosine == dot product.
        let norm: f32 = vector.iter().map(|v| v * v).sum::<f32>().sqrt();
        if norm > 0.0 {
            for v in &mut vector {
                *v /= norm;
            }
        }
        Ok(Embedding {
            model: self.model.clone(),
            dim: self.dim,
            vector,
            content_hash,
        })
    }
}
