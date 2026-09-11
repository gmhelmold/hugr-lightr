//! `--secret` parser stub (WP-01, C-04-NEW).
//! Minimal Docker-faithful default: splits on `,` and `=` to extract `id` and optional `src`.
//! Ambiguity: `src` may be a file path or store ref; default treats `src` as opaque string.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretEntry {
    pub id: String,
    pub src: Option<String>,
}

/// Parse a `--secret` token. Returns `Some` if `id=` present (minimal default); else `None`.
pub fn parse_secret(spec: &str) -> Option<SecretEntry> {
    let mut id = String::new();
    let mut src: Option<String> = None;
    for part in spec.split(',') {
        let part = part.trim();
        if let Some((k, v)) = part.split_once('=') {
            match k.trim() {
                "id" => id = v.trim().to_string(),
                "src" => src = Some(v.trim().to_string()),
                _ => {}
            }
        }
    }
    if id.is_empty() {
        None
    } else {
        Some(SecretEntry { id, src })
    }
}
