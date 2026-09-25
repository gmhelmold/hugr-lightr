//! Retained VZ lazy-compose lifecycle. State and receipts expose identity, never authority.

use lightr_core::{LightrError, Result};
use lightr_engine::{engine_for, EngineKind, ExecSpec, ResumedInstance};
use lightr_run::{SuspendedIdentity, SuspensionOwner};
use lightr_store::Store;
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::model::ServiceSpec;

pub(crate) trait LazyOwner: Send {
    fn identity(&self) -> SuspendedIdentity;
    fn resume(&self) -> Result<ResumedInstance>;
    fn guest_ip(&self) -> Result<String>;
}

impl LazyOwner for SuspensionOwner {
    fn identity(&self) -> SuspendedIdentity {
        self.identity()
    }
    fn resume(&self) -> Result<ResumedInstance> {
        self.resume()
    }
    fn guest_ip(&self) -> Result<String> {
        self.guest_ip()
    }
}

pub(crate) trait LazyFactory: Send + Sync {
    fn suspend(&self, stack_dir: &Path, svc: &ServiceSpec) -> Result<Box<dyn LazyOwner>>;
}

pub(crate) struct RealLazyFactory;

impl LazyFactory for RealLazyFactory {
    fn suspend(&self, stack_dir: &Path, svc: &ServiceSpec) -> Result<Box<dyn LazyOwner>> {
        if svc.image_ref.is_empty() || svc.image_ref == "scratch" {
            return Err(LightrError::InvalidManifest(format!(
                "lazy VZ service {:?} requires a rootfs image",
                svc.name
            )));
        }
        let service_dir = stack_dir.join("services").join(&svc.name);
        let rootfs = service_dir.join("rootfs");
        std::fs::create_dir_all(&rootfs).map_err(LightrError::Io)?;
        let store = Store::open(super::up::lightr_home_pub().join("store"))?;
        lightr_index::hydrate(&rootfs, &store, &svc.image_ref)?;
        let limits = lightr_core::ResourceLimits {
            memory_bytes: svc.mem_limit_bytes,
            cpu_millis: svc.cpu_limit_millis,
            pids_max: None,
        };
        let spec = ExecSpec {
            cwd: &rootfs,
            command: &svc.command,
            rootfs: Some(&rootfs),
            limits,
            net: true,
            net_isolate: false,
            net_fd: None,
            net_mac: None,
            mounts: &[],
            env: &[],
            workdir: None,
            user: None,
            hostname: svc.hostname.as_deref(),
            add_host: &[],
            dns: &[],
            mesh_ip: None,
            read_only: false,
            shm_size: None,
            cap_drop: &[],
            cap_add: &[],
            init: false,
            join_netns: None,
            cgroup_name: None,
            exec_ready_fd: None,
            apparmor: None,
            seccomp: None,
            bind_mounts: &[],
            resolv_conf: None,
            tmpfs: &[],
            ulimits: &[],
            oom_score_adj: None,
        };
        let engine = engine_for(EngineKind::Vz)?;
        match SuspensionOwner::suspend(engine, &spec, stack_dir, &svc.name)? {
            Ok(owner) => Ok(Box::new(owner)),
            Err(lightr_engine::SuspendResume::Unsupported { reason, .. }) => Err(
                LightrError::Unsupported(format!("lazy VZ service {:?}: {reason}", svc.name)),
            ),
            Err(_) => unreachable!("SuspendResume has only suspended or unsupported variants"),
        }
    }
}

pub(crate) struct LazyService {
    owner: std::sync::Mutex<Box<dyn LazyOwner>>,
    stack_dir: PathBuf,
    service: String,
    resumed: std::sync::Mutex<Option<ResumedInstance>>,
}

impl LazyService {
    pub(crate) fn new(owner: Box<dyn LazyOwner>, stack_dir: &Path, service: &str) -> Result<Self> {
        let identity = owner.identity();
        let this = Self {
            owner: std::sync::Mutex::new(owner),
            stack_dir: stack_dir.to_path_buf(),
            service: service.to_string(),
            resumed: std::sync::Mutex::new(None),
        };
        this.write_state("suspended", &identity, None, None, None)?;
        Ok(this)
    }

    pub(crate) fn accept(&self, mut inbound: std::net::TcpStream, target_port: u16) -> Result<()> {
        let accepted_at = now_ms();
        inbound
            .set_read_timeout(Some(Duration::from_secs(30)))
            .map_err(LightrError::Io)?;
        let mut first = [0u8; 1];
        let read = inbound.read(&mut first).map_err(LightrError::Io)?;
        if read == 0 {
            return Ok(());
        }
        let first_byte_at = now_ms();
        let identity = self
            .owner
            .lock()
            .expect("lazy owner mutex poisoned")
            .identity();
        self.write_state(
            "resuming",
            &identity,
            None,
            Some(accepted_at),
            Some(first_byte_at),
        )?;
        let mut retained = self.resumed.lock().expect("lazy resumed mutex poisoned");
        let resumed = match &*retained {
            Some(instance) => instance.clone(),
            None => {
                let instance = self
                    .owner
                    .lock()
                    .expect("lazy owner mutex poisoned")
                    .resume()?;
                *retained = Some(instance.clone());
                instance
            }
        };
        drop(retained);
        let resumed_at = now_ms();
        self.write_state(
            "running",
            &identity,
            Some(resumed.pid),
            Some(accepted_at),
            Some(first_byte_at),
        )?;
        self.write_receipt(
            &identity,
            resumed.pid,
            accepted_at,
            first_byte_at,
            resumed_at,
        )?;
        let guest_ip = self
            .owner
            .lock()
            .expect("lazy owner mutex poisoned")
            .guest_ip()?;
        let guest_addr = format!("{guest_ip}:{target_port}")
            .parse()
            .map_err(|error| {
                LightrError::InvalidRef(format!("invalid VZ guest address: {error}"))
            })?;
        let deadline = std::time::Instant::now() + Duration::from_secs(30);
        let mut outbound = loop {
            match std::net::TcpStream::connect_timeout(&guest_addr, Duration::from_millis(250)) {
                Ok(stream) => break stream,
                Err(error) if std::time::Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(50));
                    let _ = error;
                }
                Err(error) => return Err(LightrError::Io(error)),
            }
        };
        outbound
            .write_all(&first[..read])
            .map_err(LightrError::Io)?;
        super::supervise_net::proxy_bidirectional(inbound, outbound);
        Ok(())
    }

    fn service_dir(&self) -> PathBuf {
        self.stack_dir.join("services").join(&self.service)
    }

    fn write_state(
        &self,
        status: &str,
        identity: &SuspendedIdentity,
        pid: Option<u32>,
        accepted_at: Option<u128>,
        first_byte_at: Option<u128>,
    ) -> Result<()> {
        write_jcs_atomic(
            &self.service_dir().join("state.json"),
            &json!({
                "artifact_sha256": identity.artifact_sha256, "instance_id": identity.instance_id,
                "pid": pid, "status": status, "accepted_at_unix_ms": accepted_at,
                "first_byte_at_unix_ms": first_byte_at,
            }),
        )
    }

    fn write_receipt(
        &self,
        identity: &SuspendedIdentity,
        pid: u32,
        accepted_at: u128,
        first_byte_at: u128,
        resumed_at: u128,
    ) -> Result<()> {
        write_jcs_atomic(
            &self.service_dir().join("lazy_v1.receipt.json"),
            &json!({
                "accept_at_unix_ms": accepted_at, "artifact_sha256": identity.artifact_sha256,
                "first_byte_at_unix_ms": first_byte_at, "instance_id": identity.instance_id,
                "pid": pid, "resume_at_unix_ms": resumed_at, "schema": "lazy_v1", "service": self.service,
            }),
        )
    }
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

fn write_jcs_atomic(path: &Path, value: &Value) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| LightrError::InvalidRef("lazy state has no parent".into()))?;
    std::fs::create_dir_all(parent).map_err(LightrError::Io)?;
    let mut bytes = Vec::new();
    jcs(value, &mut bytes)?;
    let temp = parent.join(format!(
        ".{}.tmp-{}",
        path.file_name().and_then(|n| n.to_str()).unwrap_or("state"),
        std::process::id()
    ));
    let mut file = std::fs::File::create(&temp).map_err(LightrError::Io)?;
    file.write_all(&bytes).map_err(LightrError::Io)?;
    file.sync_all().map_err(LightrError::Io)?;
    std::fs::rename(&temp, path).map_err(LightrError::Io)?;
    std::fs::File::open(parent)
        .map_err(LightrError::Io)?
        .sync_all()
        .map_err(LightrError::Io)
}

fn jcs(value: &Value, out: &mut Vec<u8>) -> Result<()> {
    match value {
        Value::Null => out.extend_from_slice(b"null"),
        Value::Bool(v) => out.extend_from_slice(if *v { b"true" } else { b"false" }),
        Value::String(v) => out.extend_from_slice(
            serde_json::to_string(v)
                .map_err(|e| LightrError::InvalidManifest(e.to_string()))?
                .as_bytes(),
        ),
        Value::Number(v) => out.extend_from_slice(v.to_string().as_bytes()),
        Value::Array(values) => {
            out.push(b'[');
            for (i, v) in values.iter().enumerate() {
                if i != 0 {
                    out.push(b',');
                }
                jcs(v, out)?;
            }
            out.push(b']');
        }
        Value::Object(values) => {
            out.push(b'{');
            let mut entries: Vec<_> = values.iter().collect();
            entries.sort_by(|(a, _), (b, _)| a.encode_utf16().cmp(b.encode_utf16()));
            for (i, (k, v)) in entries.into_iter().enumerate() {
                if i != 0 {
                    out.push(b',');
                }
                jcs(&Value::String(k.clone()), out)?;
                out.push(b':');
                jcs(v, out)?;
            }
            out.push(b'}');
        }
    };
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    struct FakeOwner(Arc<AtomicUsize>);
    impl LazyOwner for FakeOwner {
        fn identity(&self) -> SuspendedIdentity {
            SuspendedIdentity {
                instance_id: "retained-id".into(),
                artifact_sha256: "digest".into(),
            }
        }
        fn resume(&self) -> Result<ResumedInstance> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(ResumedInstance {
                instance_id: "retained-id".into(),
                artifact_sha256: "digest".into(),
                pid: 4242,
            })
        }
        fn guest_ip(&self) -> Result<String> {
            Ok("127.0.0.2".into())
        }
    }

    #[test]
    fn fake_owner_writes_suspended_jcs_state_without_authority() {
        let dir = tempfile::tempdir().unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        let lazy =
            LazyService::new(Box::new(FakeOwner(Arc::clone(&calls))), dir.path(), "web").unwrap();
        let state = std::fs::read_to_string(dir.path().join("services/web/state.json")).unwrap();
        assert_eq!(
            state,
            r#"{"accepted_at_unix_ms":null,"artifact_sha256":"digest","first_byte_at_unix_ms":null,"instance_id":"retained-id","pid":null,"status":"suspended"}"#
        );
        assert!(!state.contains("release_token"));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        drop(lazy);
    }
}
