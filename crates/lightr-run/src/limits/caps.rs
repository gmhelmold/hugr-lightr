//! CapParser (--cap-add/--cap-drop string) — frozen C-05 interface.
//!
//! Minimal Docker-faithful default: validates cap names against Linux uapi
//! (same table as engine/ns/mod.rs); unknown cap = hard error (fail-closed).
//! Apply: native = honest Err (caps are sandbox semantics, not host rlimits);
//! ns = cgroup-v2 pruning + prctl capset (delegated to engine/ns); vz = guest cap
//! set (FFI deferred — ambiguity noted in return card).

use lightr_core::{LightrError, Result};

/// Parser for `--cap-add` / `--cap-drop` tokens.
/// Minimal: trims whitespace, normalizes `CAP_` prefix + case, validates
/// against the Linux capability table (0..=CAP_LAST_CAP).
pub struct CapParser;

impl CapParser {
    /// Parse a single cap token. Returns the normalized name (`CHOWN`, `ALL`, etc.).
    /// Unknown names = honest `Err` (fail-closed per C-05 / WP-#94).
    pub fn parse(s: &str) -> Result<String> {
        let s = s.trim();
        if s.is_empty() {
            return Err(LightrError::InvalidRef(
                "empty capability token".to_string(),
            ));
        }
        let up = s.to_ascii_uppercase();
        let normalized = up.strip_prefix("CAP_").unwrap_or(&up).to_string();
        if normalized.eq_ignore_ascii_case("ALL") {
            return Ok("ALL".to_string());
        }
        // Minimal validation: check against the same 41-cap table used by ns engine.
        const CAP_NAMES: [&str; 41] = [
            "CHOWN", "DAC_OVERRIDE", "DAC_READ_SEARCH", "FOWNER", "FSETID",
            "KILL", "SETGID", "SETUID", "SETPCAP", "LINUX_IMMUTABLE",
            "NET_BIND_SERVICE", "NET_BROADCAST", "NET_ADMIN", "NET_RAW",
            "IPC_LOCK", "IPC_OWNER", "SYS_MODULE", "SYS_RAWIO", "SYS_CHROOT",
            "SYS_PTRACE", "SYS_PACCT", "SYS_ADMIN", "SYS_BOOT", "SYS_NICE",
            "SYS_RESOURCE", "SYS_TIME", "SYS_TTY_CONFIG", "MKNOD", "LEASE",
            "AUDIT_WRITE", "AUDIT_CONTROL", "SETFCAP", "MAC_OVERRIDE", "MAC_ADMIN",
            "SYSLOG", "WAKE_ALARM", "BLOCK_SUSPEND", "AUDIT_READ", "PERFMON",
            "BPF", "CHECKPOINT_RESTORE",
        ];
        if !CAP_NAMES.contains(&normalized.as_str()) {
            return Err(LightrError::InvalidRef(format!(
                "unknown capability: {s} (normalized: {normalized})"
            )));
        }
        Ok(normalized)
    }
}

/// Native engine: capabilities are sandbox semantics; native has no sandbox.
/// Honest `Err` per F-203 / C-05.
pub fn apply_native(_caps: &[String]) -> Result<()> {
    Err(LightrError::InvalidRef(
        "native engine cannot enforce --cap-add/--cap-drop; use --engine ns".to_string(),
    ))
}

/// NS engine: delegated to `engine/ns` (prctl/capset + bounding-set pruning).
/// Minimal stub: returns honest `Err` noting deferred enforcement.
#[cfg(target_os = "linux")]
pub fn apply_cgroup(_caps: &[String]) -> Result<()> {
    // C-05 ambiguity: exact cgroup-v2 device.allow + cap-pruning wiring deferred
    // to engine/ns module; this stub honors the frozen seam.
    Err(LightrError::Io(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "cap enforcement deferred to engine/ns (C-05 ambiguity — minimal default)",
    )))
}

#[cfg(not(target_os = "linux"))]
pub fn apply_cgroup(_caps: &[String]) -> Result<()> {
    Err(LightrError::Io(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "cgroup cap enforcement requires Linux (C-05 minimal default)",
    )))
}

/// VZ engine: guest cap set via FFI shim. Minimal stub.
pub fn apply_vz(_caps: &[String]) -> Result<()> {
    Err(LightrError::Io(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "vz cap FFI deferred (C-05 ambiguity — minimal docker-faithful default)",
    )))
}
