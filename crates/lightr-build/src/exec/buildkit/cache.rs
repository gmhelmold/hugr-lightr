//! `--cache-from` parser stub (WP-01, C-04-NEW).
//! Minimal Docker-faithful default: treats spec as image ref or external source string.
//! Ambiguity: exact split syntax (`type=image,ref=...` vs bare image) not frozen; default treats entire token as ref/string.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheFrom(pub String);

/// Parse a `--cache-from` token. Returns `Some` for any non-empty spec (minimal default).
pub fn parse_cache_from(spec: &str) -> Option<CacheFrom> {
    if spec.trim().is_empty() {
        None
    } else {
        Some(CacheFrom(spec.trim().to_string()))
    }
}
