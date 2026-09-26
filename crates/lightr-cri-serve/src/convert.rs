//! Seam conversions: CANONICAL vocab (`cri_canon`, vendored lightr-cri) ⇄ LOCAL
//! vocab (`lightr_cri_backend`, this workspace).
//!
//! The two vocab crates are PARALLEL TRANSCRIPTIONS of the frozen seam —
//! structurally identical, nominally distinct (same crate name, different repo).
//! So every conversion below is a mechanical field-by-field move. Helpers are
//! named by direction:
//!   * `c2l_*` = canonical → local  (incoming method ARGS)
//!   * `l2c_*` = local → canonical  (outgoing RESULTS / errors)
//!
//! v1.2 security context is now part of both vocabularies and is converted
//! field-for-field, so kubelet security profiles reach the real backend.

use cri_canon as canon;
use lightr_cri_backend as local;

// ── Errors / Result ──────────────────────────────────────────────────────────

pub fn l2c_err(e: local::BackendError) -> canon::BackendError {
    use local::BackendError as L;
    match e {
        L::NotFound(m) => canon::BackendError::NotFound(m),
        L::AlreadyExists(m) => canon::BackendError::AlreadyExists(m),
        L::InvalidArgument(m) => canon::BackendError::InvalidArgument(m),
        L::FailedPrecondition(m) => canon::BackendError::FailedPrecondition(m),
        L::InUse(m) => canon::BackendError::InUse(m),
        L::Internal(m) => canon::BackendError::Internal(m),
        L::Io(e) => canon::BackendError::Io(e), // std::io::Error moves as-is
    }
}

// ── Identifiers ──────────────────────────────────────────────────────────────

pub fn c2l_sandbox_id(x: &canon::SandboxId) -> local::SandboxId {
    local::SandboxId(x.0.clone())
}
pub fn l2c_sandbox_id(x: local::SandboxId) -> canon::SandboxId {
    canon::SandboxId(x.0)
}
pub fn c2l_container_id(x: &canon::ContainerId) -> local::ContainerId {
    local::ContainerId(x.0.clone())
}
pub fn l2c_container_id(x: local::ContainerId) -> canon::ContainerId {
    canon::ContainerId(x.0)
}

// ── Enums ────────────────────────────────────────────────────────────────────

pub fn c2l_protocol(p: canon::Protocol) -> local::Protocol {
    match p {
        canon::Protocol::Tcp => local::Protocol::Tcp,
        canon::Protocol::Udp => local::Protocol::Udp,
        canon::Protocol::Sctp => local::Protocol::Sctp,
    }
}
pub fn l2c_protocol(p: local::Protocol) -> canon::Protocol {
    match p {
        local::Protocol::Tcp => canon::Protocol::Tcp,
        local::Protocol::Udp => canon::Protocol::Udp,
        local::Protocol::Sctp => canon::Protocol::Sctp,
    }
}

pub fn c2l_sandbox_state(s: canon::SandboxState) -> local::SandboxState {
    match s {
        canon::SandboxState::Ready => local::SandboxState::Ready,
        canon::SandboxState::NotReady => local::SandboxState::NotReady,
    }
}
pub fn l2c_sandbox_state(s: local::SandboxState) -> canon::SandboxState {
    match s {
        local::SandboxState::Ready => canon::SandboxState::Ready,
        local::SandboxState::NotReady => canon::SandboxState::NotReady,
    }
}

pub fn c2l_container_state(s: canon::ContainerState) -> local::ContainerState {
    match s {
        canon::ContainerState::Created => local::ContainerState::Created,
        canon::ContainerState::Running => local::ContainerState::Running,
        canon::ContainerState::Exited => local::ContainerState::Exited,
        canon::ContainerState::Unknown => local::ContainerState::Unknown,
    }
}
pub fn l2c_container_state(s: local::ContainerState) -> canon::ContainerState {
    match s {
        local::ContainerState::Created => canon::ContainerState::Created,
        local::ContainerState::Running => canon::ContainerState::Running,
        local::ContainerState::Exited => canon::ContainerState::Exited,
        local::ContainerState::Unknown => canon::ContainerState::Unknown,
    }
}

// ── Leaf structs ─────────────────────────────────────────────────────────────

pub fn c2l_dns(d: canon::DnsConfig) -> local::DnsConfig {
    local::DnsConfig {
        servers: d.servers,
        searches: d.searches,
        options: d.options,
    }
}
pub fn l2c_dns(d: local::DnsConfig) -> canon::DnsConfig {
    canon::DnsConfig {
        servers: d.servers,
        searches: d.searches,
        options: d.options,
    }
}

pub fn c2l_port(p: canon::PortMapping) -> local::PortMapping {
    local::PortMapping {
        protocol: c2l_protocol(p.protocol),
        container_port: p.container_port,
        host_port: p.host_port,
        host_ip: p.host_ip,
    }
}
pub fn l2c_port(p: local::PortMapping) -> canon::PortMapping {
    canon::PortMapping {
        protocol: l2c_protocol(p.protocol),
        container_port: p.container_port,
        host_port: p.host_port,
        host_ip: p.host_ip,
    }
}

pub fn c2l_host_alias(alias: canon::HostAlias) -> local::HostAlias {
    local::HostAlias {
        ip: alias.ip,
        hostnames: alias.hostnames,
    }
}

pub fn l2c_host_alias(alias: local::HostAlias) -> canon::HostAlias {
    canon::HostAlias {
        ip: alias.ip,
        hostnames: alias.hostnames,
    }
}

pub fn c2l_mount(m: canon::Mount) -> local::Mount {
    local::Mount {
        container_path: m.container_path,
        host_path: m.host_path,
        readonly: m.readonly,
    }
}
pub fn l2c_mount(m: local::Mount) -> canon::Mount {
    canon::Mount {
        container_path: m.container_path,
        host_path: m.host_path,
        readonly: m.readonly,
    }
}

fn c2l_profile(p: canon::SecurityProfile) -> local::SecurityProfile {
    local::SecurityProfile {
        profile_type: match p.profile_type {
            canon::ProfileType::RuntimeDefault => local::ProfileType::RuntimeDefault,
            canon::ProfileType::Unconfined => local::ProfileType::Unconfined,
            canon::ProfileType::Localhost => local::ProfileType::Localhost,
        },
        localhost_ref: p.localhost_ref,
    }
}

fn l2c_profile(p: local::SecurityProfile) -> canon::SecurityProfile {
    canon::SecurityProfile {
        profile_type: match p.profile_type {
            local::ProfileType::RuntimeDefault => canon::ProfileType::RuntimeDefault,
            local::ProfileType::Unconfined => canon::ProfileType::Unconfined,
            local::ProfileType::Localhost => canon::ProfileType::Localhost,
        },
        localhost_ref: p.localhost_ref,
    }
}

fn c2l_security(s: canon::SecurityContext) -> local::SecurityContext {
    local::SecurityContext {
        apparmor: s.apparmor.map(c2l_profile),
        seccomp: s.seccomp.map(c2l_profile),
        capabilities: s.capabilities.map(|c| local::Capabilities {
            add: c.add,
            drop: c.drop,
        }),
        run_as_user: s.run_as_user,
        run_as_group: s.run_as_group,
    }
}

fn l2c_security(s: local::SecurityContext) -> canon::SecurityContext {
    canon::SecurityContext {
        apparmor: s.apparmor.map(l2c_profile),
        seccomp: s.seccomp.map(l2c_profile),
        capabilities: s.capabilities.map(|c| canon::Capabilities {
            add: c.add,
            drop: c.drop,
        }),
        run_as_user: s.run_as_user,
        run_as_group: s.run_as_group,
    }
}

pub fn c2l_auth(a: &canon::AuthConfig) -> local::AuthConfig {
    local::AuthConfig {
        username: a.username.clone(),
        password: a.password.clone(),
        auth: a.auth.clone(),
        server_address: a.server_address.clone(),
    }
}

// ── Configs ──────────────────────────────────────────────────────────────────

pub fn c2l_sandbox_cfg(c: canon::SandboxConfig) -> local::SandboxConfig {
    local::SandboxConfig {
        name: c.name,
        uid: c.uid,
        namespace: c.namespace,
        attempt: c.attempt,
        labels: c.labels,
        annotations: c.annotations,
        log_directory: c.log_directory,
        hostname: c.hostname,
        host_network: c.host_network,
        dns: c.dns.map(c2l_dns),
        port_mappings: c.port_mappings.into_iter().map(c2l_port).collect(),
        host_aliases: c.host_aliases.into_iter().map(c2l_host_alias).collect(),
    }
}
pub fn l2c_sandbox_cfg(c: local::SandboxConfig) -> canon::SandboxConfig {
    canon::SandboxConfig {
        name: c.name,
        uid: c.uid,
        namespace: c.namespace,
        attempt: c.attempt,
        labels: c.labels,
        annotations: c.annotations,
        log_directory: c.log_directory,
        hostname: c.hostname,
        host_network: c.host_network,
        dns: c.dns.map(l2c_dns),
        port_mappings: c.port_mappings.into_iter().map(l2c_port).collect(),
        host_aliases: c.host_aliases.into_iter().map(l2c_host_alias).collect(),
    }
}

pub fn c2l_container_cfg(c: canon::ContainerConfig) -> local::ContainerConfig {
    local::ContainerConfig {
        name: c.name,
        attempt: c.attempt,
        image_ref: c.image_ref,
        command: c.command,
        args: c.args,
        working_dir: c.working_dir,
        envs: c.envs,
        mounts: c.mounts.into_iter().map(c2l_mount).collect(),
        labels: c.labels,
        annotations: c.annotations,
        log_path: c.log_path,
        tty: c.tty,
        stdin: c.stdin,
        security: c.security.map(c2l_security),
    }
}
pub fn l2c_container_cfg(c: local::ContainerConfig) -> canon::ContainerConfig {
    canon::ContainerConfig {
        name: c.name,
        attempt: c.attempt,
        image_ref: c.image_ref,
        command: c.command,
        args: c.args,
        working_dir: c.working_dir,
        envs: c.envs,
        mounts: c.mounts.into_iter().map(l2c_mount).collect(),
        labels: c.labels,
        annotations: c.annotations,
        log_path: c.log_path,
        tty: c.tty,
        stdin: c.stdin,
        security: c.security.map(l2c_security),
    }
}

// ── Status / records (results only → l2c) ────────────────────────────────────

pub fn l2c_sandbox_status(s: local::SandboxStatus) -> canon::SandboxStatus {
    canon::SandboxStatus {
        id: l2c_sandbox_id(s.id),
        config: l2c_sandbox_cfg(s.config),
        state: l2c_sandbox_state(s.state),
        created_at_nanos: s.created_at_nanos,
        ip: s.ip,
        netns_path: s.netns_path,
    }
}

pub fn l2c_container_status(s: local::ContainerStatus) -> canon::ContainerStatus {
    canon::ContainerStatus {
        id: l2c_container_id(s.id),
        sandbox: l2c_sandbox_id(s.sandbox),
        config: l2c_container_cfg(s.config),
        state: l2c_container_state(s.state),
        created_at_nanos: s.created_at_nanos,
        started_at_nanos: s.started_at_nanos,
        finished_at_nanos: s.finished_at_nanos,
        exit_code: s.exit_code,
        reason: s.reason,
        message: s.message,
    }
}

pub fn l2c_exec_result(r: local::ExecResult) -> canon::ExecResult {
    canon::ExecResult {
        exit_code: r.exit_code,
        stdout: r.stdout,
        stderr: r.stderr,
    }
}

pub fn l2c_stats(r: local::ContainerStatsRec) -> canon::ContainerStatsRec {
    canon::ContainerStatsRec {
        id: l2c_container_id(r.id),
        timestamp_nanos: r.timestamp_nanos,
        cpu_usage_core_nanos: r.cpu_usage_core_nanos,
        memory_working_set_bytes: r.memory_working_set_bytes,
    }
}

pub fn l2c_pulled(p: local::PulledImage) -> canon::PulledImage {
    canon::PulledImage {
        ref_name: p.ref_name,
        root_hex: p.root_hex,
        total_size: p.total_size,
    }
}

pub fn l2c_image_record(r: local::ImageRecord) -> canon::ImageRecord {
    canon::ImageRecord {
        id: r.id,
        ref_name: r.ref_name,
        size: r.size,
    }
}

pub fn l2c_fs_info(f: local::FsInfo) -> canon::FsInfo {
    canon::FsInfo {
        timestamp_nanos: f.timestamp_nanos,
        mountpoint: f.mountpoint,
        used_bytes: f.used_bytes,
        inodes_used: f.inodes_used,
    }
}

// ── Filters (args only → c2l) ────────────────────────────────────────────────

pub fn c2l_sandbox_filter(f: &canon::SandboxFilter) -> local::SandboxFilter {
    local::SandboxFilter {
        id: f.id.as_ref().map(c2l_sandbox_id),
        state: f.state.map(c2l_sandbox_state),
        label_selector: f.label_selector.clone(),
    }
}

pub fn c2l_container_filter(f: &canon::ContainerFilter) -> local::ContainerFilter {
    local::ContainerFilter {
        id: f.id.as_ref().map(c2l_container_id),
        sandbox: f.sandbox.as_ref().map(c2l_sandbox_id),
        state: f.state.map(c2l_container_state),
        label_selector: f.label_selector.clone(),
    }
}

// ── Streaming session (result only → l2c) ────────────────────────────────────

/// Bridges a LOCAL `ExitWaiter` so it satisfies the CANONICAL `ExitWaiter`
/// trait. The two waiter traits are nominally distinct (parallel transcriptions)
/// but identical in shape: a single `wait(self: Box<Self>) -> Result<i32>`.
struct WaiterBridge(Box<dyn local::ExitWaiter>);

impl canon::ExitWaiter for WaiterBridge {
    fn wait(self: Box<Self>) -> canon::Result<i32> {
        self.0.wait().map_err(l2c_err)
    }
}

/// Convert a local `StreamSession` into the canonical one. The fd fields are
/// plain `std::fs::File` (the SAME std type in both crates) so they move across
/// directly; only the boxed waiter needs the trait bridge above.
pub fn l2c_stream(s: local::StreamSession) -> canon::StreamSession {
    canon::StreamSession {
        stdin: s.stdin,
        stdout: s.stdout,
        stderr: s.stderr,
        pty_master: s.pty_master,
        waiter: Box::new(WaiterBridge(s.waiter)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn v13_configs_round_trip_through_transcription() {
        let sandbox = canon::SandboxConfig {
            name: "pod".into(),
            uid: "uid".into(),
            namespace: "ns".into(),
            attempt: 0,
            labels: BTreeMap::new(),
            annotations: BTreeMap::new(),
            log_directory: String::new(),
            hostname: String::new(),
            host_network: false,
            dns: None,
            port_mappings: vec![],
            host_aliases: vec![canon::HostAlias {
                ip: "10.0.0.10".into(),
                hostnames: vec!["cache.internal".into()],
            }],
        };
        assert_eq!(l2c_sandbox_cfg(c2l_sandbox_cfg(sandbox.clone())), sandbox);

        let container = canon::ContainerConfig {
            name: "ctr".into(),
            attempt: 0,
            image_ref: "example:v13".into(),
            command: vec![],
            args: vec![],
            working_dir: String::new(),
            envs: vec![],
            mounts: vec![],
            labels: BTreeMap::new(),
            annotations: BTreeMap::new(),
            log_path: String::new(),
            tty: false,
            stdin: false,
            security: Some(canon::SecurityContext {
                apparmor: None,
                seccomp: None,
                capabilities: None,
                run_as_user: Some(1000),
                run_as_group: Some(1000),
            }),
        };
        assert_eq!(l2c_container_cfg(c2l_container_cfg(container.clone())), container);
    }
}
