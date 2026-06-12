//! Search: Postgres full-text search backend.
//!
//! Indexing is done via a generated `tsvector` column with a GIN
//! index (see migrations). This crate exposes a typed query API
//! that maps to `to_tsquery` + ranking + filters.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
    pub namespace: Option<String>,
    pub min_downloads: Option<i64>,
    pub sort: SortBy,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortBy {
    #[default]
    Relevance,
    Downloads,
    Stars,
    Recent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    pub skill_id: Uuid,
    pub namespace_slug: String,
    pub slug: String,
    pub display_name: String,
    pub description: Option<String>,
    pub downloads: i64,
    pub stars: i64,
    pub rank: f32,
}

pub mod pg_search;
pub mod duplicate;

pub use duplicate::{
    Confidence, DuplicateCandidate, DuplicateDetector, DuplicateReport, SuggestedAction,
};
