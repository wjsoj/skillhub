//! TODO: implement Postgres full-text search.
//!
//! Plan: build a parameterized SQL using `plainto_tsquery` on the
//! `search_vector` column, join `namespaces`, apply visibility
//! filter from the calling principal, and ORDER BY ts_rank /
//! downloads / stars depending on `SortBy`.
