//! `--ssh` parser stub (WP-01, C-04-NEW).
//! Minimal Docker-faithful default: treats `default=` as special id else splits on `=`.
//! Ambiguity: exact `default` semantics vs named id not frozen; default treats string as opaque.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SshEntry {
    pub id: Option<String>,
    pub src: Option<String>,
}

/// Parse a `--ssh` token. Returns `Some` for any non-empty spec (minimal default).
pub fn parse_ssh(spec: &str) -> Option<SshEntry> {
    if spec.trim().is_empty() {
        return None;
    }
    if let Some(rest) = spec.strip_prefix("default=") {
        return Some(SshEntry {
            id: Some("default".to_string()),
            src: Some(rest.trim().to_string()),
        });
    }
    if let Some((k, v)) = spec.split_once('=') {
        Some(SshEntry {
            id: Some(k.trim().to_string()),
            src: Some(v.trim().to_string()),
        })
    } else {
        Some(SshEntry {
            id: None,
            src: Some(spec.trim().to_string()),
        })
    }
}
