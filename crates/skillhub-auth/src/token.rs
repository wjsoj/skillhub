//! Server-managed API tokens.
//!
//! Format: `<conf>_<prefix>_<secret>` e.g. `sk_a1b2c3d4_<64 hex>`.
//! Only the SHA-256 of the full token is stored, alongside the lookup
//! `prefix`. On verify we split out the prefix, look the row up by it,
//! then constant-time compare the hash.

use rand::RngCore;
use sha2::{Digest, Sha256};

pub struct GeneratedToken {
    /// The full token — shown to the user exactly once.
    pub plaintext: String,
    /// The lookup prefix stored in the `prefix` column (unique).
    pub prefix: String,
    /// SHA-256 hex of the full token, stored in the `hash` column.
    pub hash: String,
}

fn rand_hex(bytes: usize) -> String {
    let mut buf = vec![0u8; bytes];
    rand::thread_rng().fill_bytes(&mut buf);
    buf.iter().map(|b| format!("{b:02x}")).collect()
}

/// Hex SHA-256 of a token string.
pub fn hash_token(token: &str) -> String {
    let mut h = Sha256::new();
    h.update(token.as_bytes());
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// Mint a fresh token. `conf_prefix` comes from config (e.g. "sk").
pub fn generate(conf_prefix: &str) -> GeneratedToken {
    let prefix = rand_hex(4); // 8 hex chars
    let secret = rand_hex(32); // 64 hex chars
    let plaintext = format!("{conf_prefix}_{prefix}_{secret}");
    let hash = hash_token(&plaintext);
    GeneratedToken {
        plaintext,
        prefix,
        hash,
    }
}

/// Pull the lookup prefix out of a presented token, if well-formed.
pub fn parse_prefix(token: &str) -> Option<String> {
    let parts: Vec<&str> = token.split('_').collect();
    if parts.len() == 3 && !parts[1].is_empty() {
        Some(parts[1].to_string())
    } else {
        None
    }
}

/// Constant-time compare of the token's hash against the stored hash.
pub fn verify(token: &str, stored_hash: &str) -> bool {
    let computed = hash_token(token);
    if computed.len() != stored_hash.len() {
        return false;
    }
    let mut diff = 0u8;
    for (a, b) in computed.bytes().zip(stored_hash.bytes()) {
        diff |= a ^ b;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_parse_verify_roundtrips() {
        let g = generate("sk");
        assert!(g.plaintext.starts_with("sk_"));
        assert_eq!(parse_prefix(&g.plaintext).as_deref(), Some(g.prefix.as_str()));
        assert!(verify(&g.plaintext, &g.hash));
        assert!(!verify("sk_deadbeef_00", &g.hash));
    }
}
