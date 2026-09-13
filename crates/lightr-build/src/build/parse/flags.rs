//! Flag-token parsing for instructions that take `--key=value` options
//! (FROM `--platform`, COPY/ADD `--from`/`--chown`/`--chmod`, HEALTHCHECK opts).
//!
//! Faithful, minimal: a leading run of `--key=value` tokens is captured into
//! `(key, value)` pairs; everything after the first non-flag token is
//! positional. No interpolation, no quoting beyond whitespace splitting.

/// Split a flag-bearing instruction tail into its leading `--key[=value]` flags
/// and the remaining positional tokens.
///
/// Only `--key[=value]` tokens at the front are treated as flags; once a
/// non-flag token is seen, the rest are positional (Docker requires flags to
/// precede positionals for these instructions).
pub(super) fn split_flags(rest: &str) -> (Vec<(String, String)>, Vec<String>) {
    let mut flags = Vec::new();
    let mut positional = Vec::new();
    let mut in_flags = true;
    for tok in rest.split_ascii_whitespace() {
        if in_flags {
            if let Some(flag) = tok.strip_prefix("--") {
                let (key, value) = flag.split_once('=').unwrap_or((flag, ""));
                flags.push((key.to_string(), value.to_string()));
                continue;
            }
            in_flags = false;
        }
        positional.push(tok.to_string());
    }
    (flags, positional)
}

/// Reject flags not explicitly supported by this instruction before execution.
pub(super) fn validate_flags(
    flags: &[(String, String)],
    instruction: &str,
    allowed: &[&str],
) -> lightr_core::Result<()> {
    for (name, value) in flags {
        if !allowed.contains(&name.as_str()) {
            return Err(lightr_core::LightrError::InvalidManifest(format!(
                "{instruction}: unknown flag --{name}"
            )));
        }
        if value.is_empty() {
            return Err(lightr_core::LightrError::InvalidManifest(format!(
                "{instruction}: flag --{name} requires =<value>"
            )));
        }
    }
    Ok(())
}
