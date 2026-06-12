//! OpenAI-compatible embedding HTTP client.
//!
//! Works against OpenAI, Azure OpenAI, Ollama (`/v1/embeddings`), vLLM,
//! and any other server speaking the same minimal contract:
//!
//! POST {base}/embeddings
//! { "model": "...", "input": "..." }
//! → { "data": [ { "embedding": [..] } ], "model": "..." }

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::{EmbedError, EmbedResult, Embedder, Embedding};

pub struct HttpEmbedder {
    client: Client,
    base_url: String,
    api_key: Option<String>,
    model: String,
    dim: usize,
}

#[derive(Serialize)]
struct EmbedReq<'a> {
    model: &'a str,
    input: &'a str,
}

#[derive(Deserialize)]
struct EmbedResp {
    data: Vec<EmbedRespItem>,
}

#[derive(Deserialize)]
struct EmbedRespItem {
    embedding: Vec<f32>,
}

impl HttpEmbedder {
    pub fn new(base_url: impl Into<String>, model: impl Into<String>, dim: usize) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("reqwest client"),
            base_url: base_url.into(),
            api_key: None,
            model: model.into(),
            dim,
        }
    }

    pub fn with_api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }
}

#[async_trait]
impl Embedder for HttpEmbedder {
    fn model(&self) -> &str {
        &self.model
    }
    fn dim(&self) -> usize {
        self.dim
    }

    async fn embed(&self, text: &str) -> EmbedResult<Embedding> {
        let url = format!("{}/embeddings", self.base_url.trim_end_matches('/'));
        let mut req = self.client.post(&url).json(&EmbedReq {
            model: &self.model,
            input: text,
        });
        if let Some(k) = &self.api_key {
            req = req.bearer_auth(k);
        }
        let resp = req.send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(EmbedError::Provider(format!("{status}: {body}")));
        }
        let body: EmbedResp = resp.json().await?;
        let item = body
            .data
            .into_iter()
            .next()
            .ok_or_else(|| EmbedError::Provider("empty data".into()))?;
        if item.embedding.len() != self.dim {
            return Err(EmbedError::DimMismatch {
                expected: self.dim,
                got: item.embedding.len(),
            });
        }
        Ok(Embedding {
            model: self.model.clone(),
            dim: self.dim,
            vector: item.embedding,
            content_hash: Embedding::hash_of(text),
        })
    }
}
