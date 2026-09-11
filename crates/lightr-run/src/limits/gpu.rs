//! GpuParser (--gpus string) — frozen C-05 interface.
//!
//! Minimal Docker-faithful default: parses `"all"` or device-spec strings.
//! GPU enforcement requires engine-level FFI / device cgroup pruning;
//! native honest-errors immediately (no host GPU sandbox).

use lightr_core::{LightrError, Result};

/// Parsed GPU request from `--gpus`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GpuRequest {
    /// Docker `--gpus all` — expose all host GPUs.
    All,
    /// Device-specific spec (e.g., `"device=0"`, `"device=GPU-abc"`).
    /// Minimal default: stores the raw spec string; exact device-ID parsing
    /// deferred (C-05 ambiguity — see return card).
    Specific(String),
}

/// Parser for `--gpus` tokens.
pub struct GpuParser;

impl GpuParser {
    /// Parse a `--gpus` value. `"all"` (case-insensitive) ⇒ `GpuRequest::All`.
    /// Any other non-empty string ⇒ `GpuRequest::Specific(spec)`. Empty = `Err`.
    pub fn parse(s: &str) -> Result<GpuRequest> {
        let s = s.trim();
        if s.is_empty() {
            return Err(LightrError::InvalidRef(
                "empty --gpus value".to_string(),
            ));
        }
        if s.eq_ignore_ascii_case("all") {
            return Ok(GpuRequest::All);
        }
        // Minimal default: treat any non-`all` token as a device spec.
        // Ambiguity: exact `device=ID,count=N` syntax not fully specified in C-05;
        // this stub preserves the raw spec for downstream engine/ns or vz FFI.
        Ok(GpuRequest::Specific(s.to_string()))
    }
}

/// Native engine: GPUs are sandboxed via VM framework (`vz`) or cgroup device.allow
/// (`ns`); a plain host process has no GPU isolation mechanism. Honest `Err`.
pub fn apply_native(_request: &GpuRequest) -> Result<()> {
    Err(LightrError::InvalidRef(
        "native engine cannot enforce --gpus; use --engine ns (cgroup) or vz (VM GPU)".to_string(),
    ))
}

/// NS engine: writes `devices.allow` / `devices.deny` for GPU char/block nodes.
/// Minimal stub — exact GPU device node mapping deferred (C-05 ambiguity).
#[cfg(target_os = "linux")]
pub fn apply_cgroup(request: &GpuRequest) -> Result<()> {
    let spec_str = match request {
        GpuRequest::All => "all".to_string(),
        GpuRequest::Specific(s) => s.clone(),
    };
    Err(LightrError::Io(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        format!(
            "gpu cgroup enforcement (spec={spec_str}) deferred; minimal docker-faithful default — C-05"
        ),
    )))
}

#[cfg(not(target_os = "linux"))]
pub fn apply_cgroup(request: &GpuRequest) -> Result<()> {
    let spec_str = match request {
        GpuRequest::All => "all".to_string(),
        GpuRequest::Specific(s) => s.clone(),
    };
    let _ = spec_str;
    Err(LightrError::Io(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "cgroup gpu enforcement requires Linux (C-05 minimal default)",
    )))
}

/// VZ engine: GPU access via Virtualization.framework (macOS) or KVM (Linux).
/// Minimal stub: FFI memorySize/cpuCount shim exists; GPU passthrough deferred.
pub fn apply_vz(request: &GpuRequest) -> Result<()> {
    let spec_str = match request {
        GpuRequest::All => "all".to_string(),
        GpuRequest::Specific(s) => s.clone(),
    };
    // C-05 ambiguity: exact VZ GPU FFI mapping (paravirtualized device,
    // Metal passthrough, or host GPU sharing) not frozen; this stub returns
    // honest deferred error noting the gap.
    Err(LightrError::Io(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        format!(
            "vz GPU FFI (spec={spec_str}) deferred; C-05 ambiguity — minimal docker-faithful default"
        ),
    )))
}
