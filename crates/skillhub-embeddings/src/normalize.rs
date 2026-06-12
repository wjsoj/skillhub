//! Canonicalise heterogeneous skill metadata into a single embeddable string.
//!
//! Embedding quality depends on consistent input. We lowercase, strip
//! excessive whitespace, and order fields deterministically. Manifest
//! JSON is summarised — keys/strings are emitted, deeply nested
//! structures are flattened to depth 3 to avoid blowing the context.

use serde::Serialize;

#[derive(Debug, Default, Clone, Serialize)]
pub struct SkillContent<'a> {
    pub display_name: &'a str,
    pub slug: &'a str,
    pub description: Option<&'a str>,
    pub readme: Option<&'a str>,
    pub manifest: Option<&'a serde_json::Value>,
    pub tags: &'a [String],
}

impl<'a> SkillContent<'a> {
    pub fn to_embedding_input(&self) -> String {
        let mut buf = String::new();

        push_kv(&mut buf, "name", self.display_name);
        push_kv(&mut buf, "slug", self.slug);
        if let Some(d) = self.description {
            push_kv(&mut buf, "description", d);
        }
        if !self.tags.is_empty() {
            push_kv(&mut buf, "tags", &self.tags.join(", "));
        }
        if let Some(m) = self.manifest {
            let mut sink = String::new();
            summarise_json(m, 0, 3, &mut sink);
            if !sink.is_empty() {
                push_kv(&mut buf, "manifest", &sink);
            }
        }
        if let Some(r) = self.readme {
            let trimmed = truncate(r, 4_000);
            push_kv(&mut buf, "readme", &trimmed);
        }

        normalize_whitespace(&buf.to_lowercase())
    }
}

fn push_kv(buf: &mut String, k: &str, v: &str) {
    buf.push_str(k);
    buf.push_str(": ");
    buf.push_str(v.trim());
    buf.push('\n');
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_owned()
    } else {
        s.chars().take(max).collect()
    }
}

fn normalize_whitespace(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut last_ws = false;
    for ch in s.chars() {
        if ch.is_whitespace() {
            if !last_ws {
                out.push(' ');
            }
            last_ws = true;
        } else {
            out.push(ch);
            last_ws = false;
        }
    }
    out.trim().to_owned()
}

fn summarise_json(v: &serde_json::Value, depth: usize, max_depth: usize, out: &mut String) {
    if depth > max_depth {
        return;
    }
    match v {
        serde_json::Value::String(s) => {
            out.push_str(s);
            out.push(' ');
        }
        serde_json::Value::Number(n) => {
            out.push_str(&n.to_string());
            out.push(' ');
        }
        serde_json::Value::Bool(b) => {
            out.push_str(if *b { "true " } else { "false " });
        }
        serde_json::Value::Array(arr) => {
            for item in arr.iter().take(32) {
                summarise_json(item, depth + 1, max_depth, out);
            }
        }
        serde_json::Value::Object(map) => {
            for (k, v) in map.iter().take(64) {
                out.push_str(k);
                out.push(':');
                out.push(' ');
                summarise_json(v, depth + 1, max_depth, out);
            }
        }
        serde_json::Value::Null => {}
    }
}
