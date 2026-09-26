//! The seam crate: vocabulary types + the `CriBackend` trait.
//! FROZEN per docs/spec/build-spec-r0.md §3 and docs/contract/seam-contract-v1.md.
//! v1.1 additions per docs/contract/seam-contract-v1.1.md (FROZEN 2026-06-12)
//! plus the additive v1.2 security-context and v1.3 identity/host-alias seams.

pub mod vocab;

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

/// One `/etc/hosts` entry requested for every container in a sandbox.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct HostAlias {
    pub ip: String,
    pub hostnames: Vec<String>,
}

impl HostAlias {
    fn validate(&self) -> std::result::Result<(), &'static str> {
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

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AuthConfig {
    pub username: String,
    pub password: String,
    pub auth: String,
    pub server_address: String,
}

// ── §B Streaming sessions (v1.1) ─────────────────────────────────────────────

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
    // v1.2 security context; absent means runtime default/unset.
    #[serde(default)]
    pub security: Option<SecurityContext>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct SecurityContext {
    #[serde(default)]
    pub apparmor: Option<SecurityProfile>,
    #[serde(default)]
    pub seccomp: Option<SecurityProfile>,
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
    fn validate(&self) -> std::result::Result<(), &'static str> {
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

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SecurityProfile {
    pub profile_type: ProfileType,
    #[serde(default)]
    pub localhost_ref: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ProfileType {
    RuntimeDefault,
    Unconfined,
    Localhost,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Capabilities {
    #[serde(default)]
    pub add: Vec<String>,
    #[serde(default)]
    pub drop: Vec<String>,
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

/// The seam. Synchronous on purpose: the real backend (hugr-lightr crates)
/// is sync; the shell bridges via spawn_blocking. Object-safe.
///
/// State law (vectors encode this): sandbox Ready→NotReady (stop)→gone
/// (remove). `create_container` requires the sandbox Ready (else
/// FailedPrecondition). Container Created→Running→Exited; `start` only from Created;
/// `stop` from Running (→Exited) or no-op from Created/Exited; `remove`
/// refused (FailedPrecondition) while Running; removing a sandbox
/// stops+removes its containers. All transitions persist BEFORE the call
/// returns (crash-only law).
pub trait CriBackend: Send + Sync + 'static {
    // sandbox plane
    fn run_sandbox(&self, cfg: SandboxConfig) -> Result<SandboxId>;
    /// idempotent
    fn stop_sandbox(&self, id: &SandboxId) -> Result<()>;
    /// idempotent; implies stop; removes its containers
    fn remove_sandbox(&self, id: &SandboxId) -> Result<()>;
    fn sandbox_status(&self, id: &SandboxId) -> Result<SandboxStatus>;
    fn list_sandboxes(&self, filter: &SandboxFilter) -> Result<Vec<SandboxStatus>>;

    // container plane
    fn create_container(&self, sandbox: &SandboxId, cfg: ContainerConfig) -> Result<ContainerId>;
    fn start_container(&self, id: &ContainerId) -> Result<()>;
    /// idempotent
    fn stop_container(&self, id: &ContainerId, grace_seconds: i64) -> Result<()>;
    /// idempotent; only when not Running
    fn remove_container(&self, id: &ContainerId) -> Result<()>;
    fn container_status(&self, id: &ContainerId) -> Result<ContainerStatus>;
    fn list_containers(&self, filter: &ContainerFilter) -> Result<Vec<ContainerStatus>>;
    fn container_stats(&self, id: &ContainerId) -> Result<ContainerStatsRec>;
    fn list_container_stats(&self, filter: &ContainerFilter) -> Result<Vec<ContainerStatsRec>>;

    // exec plane (R0: sync only)
    fn exec_sync(
        &self,
        id: &ContainerId,
        cmd: &[String],
        timeout_seconds: i64,
    ) -> Result<ExecResult>;

    // image plane (lazy law: pull_image MUST NOT move file bytes)
    fn pull_image(&self, image_ref: &str) -> Result<PulledImage>;
    fn image_status(&self, image_ref: &str) -> Result<Option<ImageRecord>>;
    fn list_images(&self) -> Result<Vec<ImageRecord>>;
    /// idempotent: not-found → Ok; refuses InUse while referenced by a live container
    fn remove_image(&self, image_ref: &str) -> Result<()>;
    fn image_fs_info(&self) -> Result<FsInfo>;

    // v1.1 streaming methods — default impls so v1-only backends compile
    fn open_exec(
        &self,
        _id: &ContainerId,
        _cmd: &[String],
        _tty: bool,
        _stdin: bool,
    ) -> Result<StreamSession> {
        Err(BackendError::Internal("v1.1 not implemented".to_string()))
    }
    /// Attach to the container's live stdio (spawned with held pipes/pty).
    fn open_attach(&self, _id: &ContainerId) -> Result<StreamSession> {
        Err(BackendError::Internal("v1.1 not implemented".to_string()))
    }
    /// Auth-aware pull; default delegates to pull_image (auth ignored = fake-honest).
    fn pull_image_with_auth(
        &self,
        image_ref: &str,
        _auth: Option<&AuthConfig>,
    ) -> Result<PulledImage> {
        self.pull_image(image_ref)
    }
    /// Honest network readiness for the CRI `Status.NetworkReady` condition
    /// (probe-truthful law, contract §D). Default false: a backend that does
    /// not wire CNI must NOT claim the pod network is ready. The fake
    /// overrides this to reflect `cni_available()`.
    fn network_ready(&self) -> bool {
        false
    }
}
