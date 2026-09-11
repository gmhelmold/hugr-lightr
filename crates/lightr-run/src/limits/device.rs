//! DeviceParser (--device string) — frozen C-05 interface.
//!
//! Minimal Docker-faithful default: parses `HOST:CONTAINER[:rw]` mapping.
//! Ambiguity noted: exact host-device access mode (read-only vs read-write)
//! and device-node creation semantics deferred to lead review.

use lightr_core::{LightrError, Result};

/// Parsed device mapping from `--device` token.
pub struct DeviceMapping {
    pub host_path: String,
    pub container_path: String,
    pub mode: DeviceMode,
}

/// Minimal mode default: `rw` if omitted (Docker-faithful), else `r` / `rw`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeviceMode {
    ReadOnly,
    ReadWrite,
}

impl DeviceMode {
    fn parse(s: &str) -> Result<Self> {
        match s.to_ascii_lowercase().as_str() {
            "r" => Ok(Self::ReadOnly),
            "rw" => Ok(Self::ReadWrite),
            _ => Err(LightrError::InvalidRef(format!(
                "invalid device mode: {s} (expected r or rw)"
            ))),
        }
    }
}

/// Parser for `--device` tokens.
pub struct DeviceParser;

impl DeviceParser {
    /// Parse a device token. Format: `HOST:CONTAINER` or `HOST:CONTAINER:r` / `rw`.
    /// Empty host or container = honest `Err`.
    pub fn parse(s: &str) -> Result<DeviceMapping> {
        let s = s.trim();
        if s.is_empty() {
            return Err(LightrError::InvalidRef(
                "empty --device value".to_string(),
            ));
        }
        let parts: Vec<&str> = s.split(':').collect();
        match parts.len() {
            2 => {
                // HOST:CONTAINER (no mode) ⇒ default rw (docker-faithful)
                let host_path = parts[0].trim().to_string();
                let container_path = parts[1].trim().to_string();
                if host_path.is_empty() || container_path.is_empty() {
                    return Err(LightrError::InvalidRef(format!(
                        "device mapping missing host or container path: {s}"
                    )));
                }
                Ok(DeviceMapping {
                    host_path,
                    container_path,
                    mode: DeviceMode::ReadWrite,
                })
            }
            3 => {
                let host_path = parts[0].trim().to_string();
                let container_path = parts[1].trim().to_string();
                let mode_str = parts[2].trim();
                if host_path.is_empty() || container_path.is_empty() {
                    return Err(LightrError::InvalidRef(format!(
                        "device mapping missing host or container path: {s}"
                    )));
                }
                Ok(DeviceMapping {
                    host_path,
                    container_path,
                    mode: DeviceMode::parse(mode_str)?,
                })
            }
            _ => Err(LightrError::InvalidRef(format!(
                "invalid --device format: {s} (expected HOST:CONTAINER[:rw])"
            ))),
        }
    }
}

/// Native engine: device mapping requires cgroup device.allow + bind mounts,
/// unavailable to a plain host process. Honest `Err`.
pub fn apply_native(_mapping: &DeviceMapping) -> Result<()> {
    Err(LightrError::InvalidRef(
        "native engine cannot enforce --device; use --engine ns (cgroup device.allow)".to_string(),
    ))
}

/// NS engine: writes `devices.allow` / `devices.deny` in the transient cgroup.
/// Minimal stub — exact device.allow + bind-mount wiring deferred (C-05 ambiguity).
#[cfg(target_os = "linux")]
pub fn apply_cgroup(mapping: &DeviceMapping) -> Result<()> {
    // Minimal docker-faithful default: note that device.allow needs the exact
    // device node (char/block) and that bind-mount of the container path is
    // handled separately by the mount layer. This stub returns honest deferred.
    let _ = mapping;
    Err(LightrError::Io(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        format!(
            "device enforcement ({:?}:{:?}:{:?}) deferred to engine/ns (C-05 ambiguity — minimal default)",
            mapping.host_path, mapping.container_path, mapping.mode
        ),
    )))
}

#[cfg(not(target_os = "linux"))]
pub fn apply_cgroup(mapping: &DeviceMapping) -> Result<()> {
    let _ = mapping;
    Err(LightrError::Io(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "cgroup device enforcement requires Linux (C-05 minimal default)",
    )))
}

/// VZ engine: host device access is mediated by the VM framework; minimal stub.
pub fn apply_vz(mapping: &DeviceMapping) -> Result<()> {
    let _ = mapping;
    Err(LightrError::Io(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "vz device FFI deferred (C-05 ambiguity — minimal docker-faithful default)",
    )))
}
