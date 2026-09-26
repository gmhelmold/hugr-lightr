//! CRI seam vocabulary — TRANSCRIBED from lightr-cri @ seam-contract-v1.1.
//!
//! Provenance: these shapes are EXACT copies of the frozen seam types defined
//! in `lightr-cri/crates/lightr-cri-backend/src/{lib.rs,vocab.rs}` and the
//! semantics in `lightr-cri/docs/contract/seam-contract-v1.1.md` (FROZEN
//! 2026-06-12). They are TRANSCRIBED, NOT a git/path dependency — the
//! wire-level seam is proven later by the shared conformance vectors
//! (WP-CRI-VECTORS), never by a crate import. Drift is caught by those
//! vectors, not by the compiler. Do not "improve" these shapes here.

use std::collections::BTreeMap;

// ── §A Vocabulary additions (v1.1) ───────────────────────────────────────────

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DnsConfig {
    pub servers: Vec<String>,
    pub searches: Vec<String>,
    pub options: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Protocol {
    Tcp,
    Udp,
    Sctp,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PortMapping {
    pub protocol: Protocol,
    pub container_port: i32,
    pub host_port: i32,
    #[serde(default)]
    pub host_ip: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AuthConfig {
    pub username: String,
    pub password: String,
    pub auth: String,
    pub server_address: String,
}

// ── Identifiers ──────────────────────────────────────────────────────────────

#[derive(
    Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(transparent)]
pub struct SandboxId(pub String);

#[derive(
    Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(transparent)]
pub struct ContainerId(pub String);

// ── Sandbox plane ────────────────────────────────────────────────────────────

/// One `/etc/hosts` entry requested for every container in a sandbox.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct HostAlias {
    pub ip: String,
    pub hostnames: Vec<String>,
}

impl HostAlias {
    pub(crate) fn validate(&self) -> std::result::Result<(), &'static str> {
        if self.ip.parse::<std::net::IpAddr>().is_err() {
            return Err("host alias IP must be a valid IPv4 or IPv6 address");
        }
        if self.hostnames.is_empty() || self.hostnames.iter().any(|name| !valid_hostname(name)) {
            return Err("host alias must contain valid hostnames");
        }
        Ok(())
    }
}

impl<'de> serde::Deserialize<'de> for HostAlias {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        #[derive(serde::Deserialize)]
        struct RawHostAlias {
            ip: String,
            #[serde(default)]
            hostnames: Vec<String>,
        }

        let raw = RawHostAlias::deserialize(deserializer)?;
        let alias = Self {
            ip: raw.ip,
            hostnames: raw.hostnames,
        };
        alias.validate().map_err(serde::de::Error::custom)?;
        Ok(alias)
    }
}

fn valid_hostname(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 253
        && name.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        })
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SandboxConfig {
    pub name: String,
    pub uid: String,
    pub namespace: String,
    pub attempt: u32,
    #[serde(default)]
    pub labels: BTreeMap<String, String>,
    #[serde(default)]
    pub annotations: BTreeMap<String, String>,
    #[serde(default)]
    pub log_directory: String,
    // v1.1 additions (all serde(default) — old state files load unchanged)
    #[serde(default)]
    pub hostname: String,
    #[serde(default)]
    pub host_network: bool,
    #[serde(default)]
    pub dns: Option<DnsConfig>,
    #[serde(default)]
    pub port_mappings: Vec<PortMapping>,
    /// v1.3: `/etc/hosts` aliases, applied by a future descriptor planner.
    #[serde(default)]
    pub host_aliases: Vec<HostAlias>,
}

impl SandboxConfig {
    pub(crate) fn validate_for_ingress(&self) -> std::result::Result<(), &'static str> {
        self.host_aliases.iter().try_for_each(HostAlias::validate)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SandboxState {
    Ready,
    NotReady,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SandboxStatus {
    pub id: SandboxId,
    pub config: SandboxConfig,
    pub state: SandboxState,
    pub created_at_nanos: i64,
    // v1.1 additions
    #[serde(default)]
    pub ip: Option<String>,
    #[serde(default)]
    pub netns_path: Option<String>,
}

// ── Container plane ──────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Mount {
    pub container_path: String,
    pub host_path: String,
    pub readonly: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ContainerConfig {
    pub name: String,
    pub attempt: u32,
    /// CAS vocabulary: a ref name or digest-hex into the image plane.
    pub image_ref: String,
    pub command: Vec<String>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub working_dir: String,
    #[serde(default)]
    pub envs: Vec<(String, String)>,
    #[serde(default)]
    pub mounts: Vec<Mount>,
    #[serde(default)]
    pub labels: BTreeMap<String, String>,
    #[serde(default)]
    pub annotations: BTreeMap<String, String>,
    #[serde(default)]
    pub log_path: String,
    // v1.1 additions
    #[serde(default)]
    pub tty: bool,
    #[serde(default)]
    pub stdin: bool,
    // v1.2 additive security context. `serde(default)` keeps v1.1 state and
    // vectors compatible; None means runtime default/unset.
    #[serde(default)]
    pub security: Option<SecurityContext>,
}

impl ContainerConfig {
    pub(crate) fn validate_for_ingress(&self) -> std::result::Result<(), &'static str> {
        self.security
            .as_ref()
            .map_or(Ok(()), SecurityContext::validate)
    }
}

/// v1.2 security-context subset mirrored from CRI
/// `LinuxContainerSecurityContext`. Profiles and capability sets cross the
/// canonical shell seam and reach the ns engine; unsupported platform/privilege
/// combinations still fail closed at the engine boundary.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct SecurityContext {
    /// AppArmor profile, enforced by the ns engine at container start.
    #[serde(default)]
    pub apparmor: Option<SecurityProfile>,
    /// Seccomp OCI profile, compiled and installed by the ns engine.
    #[serde(default)]
    pub seccomp: Option<SecurityProfile>,
    /// Linux capability add/drop, applied by the ns engine.
    #[serde(default)]
    pub capabilities: Option<Capabilities>,
    /// v1.3: numeric UID. `None` preserves image/runtime default.
    #[serde(default)]
    pub run_as_user: Option<u64>,
    /// v1.3: numeric GID; valid only when `run_as_user` is set.
    #[serde(default)]
    pub run_as_group: Option<u64>,
}

impl SecurityContext {
    pub(crate) fn validate(&self) -> std::result::Result<(), &'static str> {
        if self.run_as_group.is_some() && self.run_as_user.is_none() {
            return Err("run_as_group requires run_as_user");
        }
        Ok(())
    }
}

impl<'de> serde::Deserialize<'de> for SecurityContext {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        #[derive(serde::Deserialize)]
        struct RawSecurityContext {
            #[serde(default)]
            apparmor: Option<SecurityProfile>,
            #[serde(default)]
            seccomp: Option<SecurityProfile>,
            #[serde(default)]
            capabilities: Option<Capabilities>,
            #[serde(default)]
            run_as_user: Option<u64>,
            #[serde(default)]
            run_as_group: Option<u64>,
        }

        let raw = RawSecurityContext::deserialize(deserializer)?;
        let context = Self {
            apparmor: raw.apparmor,
            seccomp: raw.seccomp,
            capabilities: raw.capabilities,
            run_as_user: raw.run_as_user,
            run_as_group: raw.run_as_group,
        };
        context.validate().map_err(serde::de::Error::custom)?;
        Ok(context)
    }
}

/// Mirrors CRI `SecurityProfile`: a profile kind + an optional localhost ref
/// (the profile name/path when `Localhost`).
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SecurityProfile {
    pub profile_type: ProfileType,
    /// The loaded profile name (AppArmor) or path (seccomp) when `Localhost`;
    /// empty otherwise.
    #[serde(default)]
    pub localhost_ref: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ProfileType {
    /// The runtime's default profile.
    RuntimeDefault,
    /// No profile (explicitly unconfined).
    Unconfined,
    /// A specific loaded profile named by `localhost_ref`.
    Localhost,
}

/// CRI capability add/drop sets (`CAP_*` without the `CAP_` prefix, CRI style).
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Capabilities {
    #[serde(default)]
    pub add: Vec<String>,
    #[serde(default)]
    pub drop: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v13_local_vocab_round_trips_and_v12_records_default() {
        let sandbox: SandboxConfig = serde_json::from_str(
            r#"{"name":"pod","uid":"uid","namespace":"ns","attempt":0,"host_aliases":[{"ip":"10.0.0.10","hostnames":["cache.internal"]}]}"#,
        )
        .expect("v1.3 sandbox parses");
        assert_eq!(sandbox.host_aliases[0].hostnames, ["cache.internal"]);

        let old: crate::sandbox::SandboxRecord = serde_json::from_str(
            r#"{"id":"sandbox","config":{"name":"pod","uid":"uid","namespace":"ns","attempt":0},"state":"Ready","created_at_nanos":0}"#,
        )
        .expect("v1.2 persisted sandbox record parses");
        assert!(old.config.host_aliases.is_empty());
        let old: crate::util::ContainerRecord = serde_json::from_str(
            r#"{"id":"container","sandbox":"sandbox","config":{"name":"ctr","attempt":0,"image_ref":"example:v12","command":[]},"state":"Created","created_at_nanos":0,"started_at_nanos":0,"finished_at_nanos":0,"exit_code":0}"#,
        )
        .expect("v1.2 persisted container record parses");
        assert_eq!(old.config.security, None);
        assert!(serde_json::from_str::<SecurityContext>(r#"{"run_as_group":1000}"#).is_err());
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ContainerState {
    Created,
    Running,
    Exited,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ContainerStatus {
    pub id: ContainerId,
    pub sandbox: SandboxId,
    pub config: ContainerConfig,
    pub state: ContainerState,
    pub created_at_nanos: i64,
    /// 0 = never started
    pub started_at_nanos: i64,
    /// 0 = still running / never started
    pub finished_at_nanos: i64,
    /// valid only when state == Exited
    pub exit_code: i32,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub message: String,
}

// ── Records ──────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecResult {
    pub exit_code: i32,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContainerStatsRec {
    pub id: ContainerId,
    pub timestamp_nanos: i64,
    pub cpu_usage_core_nanos: u64,
    pub memory_working_set_bytes: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PulledImage {
    pub ref_name: String,
    pub root_hex: String,
    pub total_size: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageRecord {
    pub id: String,
    pub ref_name: String,
    pub size: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FsInfo {
    pub timestamp_nanos: i64,
    pub mountpoint: String,
    pub used_bytes: u64,
    pub inodes_used: u64,
}

// ── Streaming sessions (v1.1 §B) ─────────────────────────────────────────────

/// Exit waiter — consumed once.
pub trait ExitWaiter: Send {
    fn wait(self: Box<Self>) -> Result<i32>;
}

/// Live I/O of an exec or attach session. tty=true → stdout carries the
/// pty stream, stderr is None, pty_master enables TIOCSWINSZ resize.
pub struct StreamSession {
    pub stdin: Option<std::fs::File>,
    pub stdout: Option<std::fs::File>,
    pub stderr: Option<std::fs::File>,
    pub pty_master: Option<std::fs::File>,
    pub waiter: Box<dyn ExitWaiter>,
}

// ── Errors ───────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum BackendError {
    NotFound(String),
    AlreadyExists(String),
    InvalidArgument(String),
    FailedPrecondition(String),
    /// image present & referenced by live container (RemoveImage refusal)
    InUse(String),
    Internal(String),
    Io(std::io::Error),
}

impl std::fmt::Display for BackendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackendError::NotFound(m) => write!(f, "not found: {m}"),
            BackendError::AlreadyExists(m) => write!(f, "already exists: {m}"),
            BackendError::InvalidArgument(m) => write!(f, "invalid argument: {m}"),
            BackendError::FailedPrecondition(m) => write!(f, "failed precondition: {m}"),
            BackendError::InUse(m) => write!(f, "in use: {m}"),
            BackendError::Internal(m) => write!(f, "internal: {m}"),
            BackendError::Io(e) => write!(f, "io: {e}"),
        }
    }
}

impl std::error::Error for BackendError {}

impl From<std::io::Error> for BackendError {
    fn from(e: std::io::Error) -> Self {
        BackendError::Io(e)
    }
}

pub type Result<T> = std::result::Result<T, BackendError>;

// ── Filters ──────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SandboxFilter {
    pub id: Option<SandboxId>,
    pub state: Option<SandboxState>,
    pub label_selector: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ContainerFilter {
    pub id: Option<ContainerId>,
    pub sandbox: Option<SandboxId>,
    pub state: Option<ContainerState>,
    pub label_selector: BTreeMap<String, String>,
}
