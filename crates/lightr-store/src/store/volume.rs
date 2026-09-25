//! Named-volume registry — daemonless, on-disk (WP-VOL-4, docker-volume parity).
//!
//! Layout (root is INJECTED by the caller — house convention, never read from
//! the global env here, so the unit tests pass a private tempdir and run safely
//! under the multi-threaded CI runner):
//!
//! ```text
//! <root>/volumes/<name>/_data/      ← the volume's data directory
//! <root>/volumes/<name>/meta.json   ← metadata (name, created_at, driver, labels)
//! ```
//!
//! These dirs back [`lightr_engine::MountKind::NamedVolume`] (the VOL-1 type).
//! Detached Linux runs publish durable active owners around child exec; [`remove`]
//! and [`prune`] recover only positively-dead owners, then refuse nonempty state.
//!
//! `meta.json` is hand-encoded JSON (lightr-store is intentionally serde-free —
//! cf. the refs/ac/imgmeta planes), with a single fixed, well-defined schema:
//! `{"name":"…","created_at":<u64>,"driver":"local","labels":{…}}`.

use lightr_core::{LightrError, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};

/// Fixed driver for the local on-disk registry (docker's default driver name).
pub const DRIVER_LOCAL: &str = "local";

/// Durable named-volume owner envelope. No mutable refcount exists: active
/// entries are the complete refcount authority.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OwnersFile {
    pub version: u8,
    pub owners: Vec<VolumeOwner>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "phase", deny_unknown_fields)]
pub enum VolumeOwner {
    #[serde(rename = "pending")]
    Pending {
        nonce: String,
        coordinator_pid: i32,
        coordinator_start_token: String,
    },
    #[serde(rename = "active")]
    Active {
        nonce: String,
        run_id: String,
        pid: i32,
        process_start_token: String,
        mount_id: String,
    },
}

impl VolumeOwner {
    pub fn nonce(&self) -> &str {
        match self {
            Self::Pending { nonce, .. } | Self::Active { nonce, .. } => nonce,
        }
    }

    pub fn active(&self) -> bool {
        matches!(self, Self::Active { .. })
    }
}

/// Exclusive ownership lock for one volume. Unix only: callers retain this
/// guard across every read/modify/write lifecycle transition.
pub struct OwnerLock(File);

impl Drop for OwnerLock {
    fn drop(&mut self) {
        #[cfg(unix)]
        {
            // Explicit unlock documents lifetime boundary; close would also release.
            let _ = unsafe { libc::flock(std::os::fd::AsRawFd::as_raw_fd(&self.0), libc::LOCK_UN) };
        }
        #[cfg(not(unix))]
        let _ = &self.0;
    }
}

/// Per-run durable witness for an owner transition. `terminal` becomes true only
/// after run status has reached its terminal durable state.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunOwnerRecord {
    pub volume: String,
    #[serde(flatten)]
    pub owner: VolumeOwner,
    #[serde(default)]
    pub terminal: bool,
}

/// Metadata for one named volume — the decoded `meta.json` plus the resolved
/// host path of the `_data/` directory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VolumeInfo {
    /// Volume name (validated docker charset).
    pub name: String,
    /// Creation time, unix seconds.
    pub created_at: u64,
    /// Driver — always `"local"` for the daemonless on-disk registry.
    pub driver: String,
    /// User labels, sorted by key for deterministic output.
    pub labels: Vec<(String, String)>,
    /// Absolute host path of the volume's `_data/` directory (the mountpoint).
    pub mountpoint: PathBuf,
}

impl VolumeInfo {
    /// Render this record as the canonical single-object JSON (the persisted
    /// `meta.json` fields plus the derived `mountpoint`). Used by the CLI
    /// `inspect`/`ls --json` handlers so the JSON shape has one source of truth.
    pub fn to_json(&self) -> String {
        let mut s = String::with_capacity(160);
        s.push('{');
        s.push_str("\"name\":");
        s.push_str(&json_str(&self.name));
        s.push_str(",\"created_at\":");
        s.push_str(&self.created_at.to_string());
        s.push_str(",\"driver\":");
        s.push_str(&json_str(&self.driver));
        s.push_str(",\"mountpoint\":");
        s.push_str(&json_str(&self.mountpoint.to_string_lossy()));
        s.push_str(",\"labels\":");
        s.push_str(&labels_json(&self.labels));
        s.push('}');
        s
    }
}

// ── name validation (docker charset; transcribed — lightr-store cannot depend
// on lightr-run where the mount-side `name_validate` lives) ───────────────────

/// Validate a volume name against docker's rule: `[a-zA-Z0-9][a-zA-Z0-9_.-]*`
/// — non-empty, first char alphanumeric, the rest alphanumeric or `_ . -`.
/// Fail-closed: anything else is rejected. (Same charset as the run-side
/// container-name validator and the `-v` named-volume parser.)
fn name_validate(name: &str) -> Result<()> {
    let mut chars = name.chars();
    match chars.next() {
        None => return Err(LightrError::InvalidRef("empty volume name".to_string())),
        Some(c) if c.is_ascii_alphanumeric() => {}
        Some(_) => {
            return Err(LightrError::InvalidRef(format!(
                "invalid volume name '{name}': must start with [a-zA-Z0-9]"
            )));
        }
    }
    for c in chars {
        let ok = c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-');
        if !ok {
            return Err(LightrError::InvalidRef(format!(
                "invalid volume name '{name}': only [a-zA-Z0-9_.-] allowed"
            )));
        }
    }
    Ok(())
}

// ── path helpers ──────────────────────────────────────────────────────────────

/// The volumes root: `<root>/volumes`.
fn volumes_root(root: &Path) -> PathBuf {
    root.join("volumes")
}

/// One volume's directory: `<root>/volumes/<name>`.
fn volume_dir(root: &Path, name: &str) -> PathBuf {
    volumes_root(root).join(name)
}

/// One volume's `_data/` directory (the mountpoint).
fn data_dir(root: &Path, name: &str) -> PathBuf {
    volume_dir(root, name).join("_data")
}

/// One volume's `meta.json` path.
fn meta_path(root: &Path, name: &str) -> PathBuf {
    volume_dir(root, name).join("meta.json")
}

fn owner_dir(root: &Path, name: &str) -> PathBuf {
    volume_dir(root, name).join(".lightr")
}

fn owners_path(root: &Path, name: &str) -> PathBuf {
    owner_dir(root, name).join("owners.json")
}

/// Acquire exclusive ownership lock. Platforms without Unix `flock` are
/// deliberately unsupported until an equivalent atomic primitive exists.
#[cfg(unix)]
pub fn owner_lock(root: &Path, name: &str) -> Result<OwnerLock> {
    name_validate(name)?;
    let dir = owner_dir(root, name);
    fs::create_dir_all(&dir)?;
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(dir.join("lock"))?;
    let rc = unsafe { libc::flock(std::os::fd::AsRawFd::as_raw_fd(&file), libc::LOCK_EX) };
    if rc != 0 {
        return Err(LightrError::Io(std::io::Error::last_os_error()));
    }
    Ok(OwnerLock(file))
}

#[cfg(not(unix))]
pub fn owner_lock(_root: &Path, _name: &str) -> Result<OwnerLock> {
    Err(LightrError::InvalidRef(
        "named-volume ownership unsupported: no atomic flock".to_string(),
    ))
}

/// Read strict v1 ownership state while caller holds [`OwnerLock`]. Missing state
/// is an empty v1 envelope; malformed state is never repaired or guessed.
pub fn read_owners(root: &Path, name: &str, _lock: &OwnerLock) -> Result<OwnersFile> {
    let path = owners_path(root, name);
    if !path.exists() {
        return Ok(OwnersFile {
            version: 1,
            owners: Vec::new(),
        });
    }
    let owners: OwnersFile = serde_json::from_slice(&fs::read(path)?).map_err(|_| {
        LightrError::InvalidManifest(format!("volume {name}: malformed .lightr/owners.json"))
    })?;
    if owners.version != 1
        || owners
            .owners
            .iter()
            .any(|owner| !valid_nonce(owner.nonce()))
    {
        return Err(LightrError::InvalidManifest(format!(
            "volume {name}: malformed .lightr/owners.json"
        )));
    }
    Ok(owners)
}

/// Publish complete owner state while caller holds [`OwnerLock`]. `rename` is
/// linearization point; success is returned only after parent-directory fsync.
pub fn write_owners(root: &Path, name: &str, owners: &OwnersFile, _lock: &OwnerLock) -> Result<()> {
    if owners.version != 1
        || owners
            .owners
            .iter()
            .any(|owner| !valid_nonce(owner.nonce()))
    {
        return Err(LightrError::InvalidManifest(
            "invalid volume owners v1".to_string(),
        ));
    }
    let dir = owner_dir(root, name);
    fs::create_dir_all(&dir)?;
    let tmp = dir.join("owners.json.tmp");
    let path = dir.join("owners.json");
    let bytes =
        serde_json::to_vec(owners).map_err(|e| LightrError::Io(std::io::Error::other(e)))?;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&tmp)?;
    use std::io::Write;
    file.write_all(&bytes)?;
    file.sync_all()?;
    drop(file);
    fs::rename(tmp, path)?;
    File::open(dir)?.sync_all()?;
    Ok(())
}

fn valid_nonce(nonce: &str) -> bool {
    nonce.len() == 64 && nonce.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Stable process identity. Linux `/proc/<pid>/stat` field 22 is parsed only
/// after final `)` so a comm containing spaces or parentheses cannot shift it.
#[cfg(target_os = "linux")]
pub fn process_start_token(pid: i32) -> Result<String> {
    if pid <= 0 {
        return Err(LightrError::InvalidRef("invalid process pid".to_string()));
    }
    let stat = fs::read_to_string(format!("/proc/{pid}/stat"))?;
    let after = stat
        .rsplit_once(')')
        .map(|(_, rest)| rest)
        .ok_or_else(|| LightrError::InvalidManifest(format!("malformed /proc/{pid}/stat")))?;
    let start = after
        .split_whitespace()
        .nth(19)
        .ok_or_else(|| LightrError::InvalidManifest(format!("malformed /proc/{pid}/stat")))?;
    if !start.bytes().all(|b| b.is_ascii_digit()) {
        return Err(LightrError::InvalidManifest(format!(
            "malformed /proc/{pid}/stat"
        )));
    }
    Ok(format!("linux:{pid}:{start}"))
}

#[cfg(not(target_os = "linux"))]
pub fn process_start_token(_pid: i32) -> Result<String> {
    Err(LightrError::InvalidRef(
        "named-volume ownership unsupported: no stable process start token".to_string(),
    ))
}

/// Create and durably publish one pending owner. Caller later replaces this
/// exact nonce with active state; failed spawn removes only this nonce.
pub fn begin_owner(root: &Path, name: &str, run_dir: &Path) -> Result<VolumeOwner> {
    let lock = owner_lock(root, name)?;
    let mut owners = read_owners(root, name, &lock)?;
    let owner = VolumeOwner::Pending {
        nonce: fresh_nonce(),
        coordinator_pid: std::process::id() as i32,
        coordinator_start_token: process_start_token(std::process::id() as i32)?,
    };
    owners.owners.push(owner.clone());
    write_owners(root, name, &owners, &lock)?;
    if let Err(error) = write_run_owner(run_dir, name, &owner, false) {
        // Witness publication failed after pending persistence. Remove only this
        // nonce while retaining the same flock; never return a ghost pending that
        // recovery cannot attribute to a run witness.
        owners
            .owners
            .retain(|candidate| candidate.nonce() != owner.nonce());
        if let Err(rollback) = write_owners(root, name, &owners, &lock) {
            return Err(LightrError::InvalidRef(format!(
                "{error}; named-volume pending rollback failed: {rollback}"
            )));
        }
        return Err(error);
    }
    Ok(owner)
}

/// Replace exact pending nonce with active child ownership and durably witness
/// same transition in run directory. No PID-only replacement is possible.
pub fn activate_owner(
    root: &Path,
    name: &str,
    run_dir: &Path,
    pending_nonce: &str,
    run_id: &str,
    pid: i32,
    mount_id: &str,
) -> Result<VolumeOwner> {
    let lock = owner_lock(root, name)?;
    let mut owners = read_owners(root, name, &lock)?;
    let index = owners
        .owners
        .iter()
        .position(
            |owner| matches!(owner, VolumeOwner::Pending { nonce, .. } if nonce == pending_nonce),
        )
        .ok_or_else(|| LightrError::InvalidRef(format!("volume {name}: pending owner lost")))?;
    let owner = VolumeOwner::Active {
        nonce: pending_nonce.to_string(),
        run_id: run_id.to_string(),
        pid,
        process_start_token: process_start_token(pid)?,
        mount_id: mount_id.to_string(),
    };
    let pending = owners.owners[index].clone();
    owners.owners[index] = owner.clone();
    write_owners(root, name, &owners, &lock)?;
    if let Err(error) = write_run_owner(run_dir, name, &owner, false) {
        // The old pending witness is still durable. Restore it before returning so
        // caller-side child teardown can abandon this exact nonce without stranding
        // an active owner whose witness was never published.
        owners.owners[index] = pending;
        write_owners(root, name, &owners, &lock)?;
        return Err(error);
    }
    Ok(owner)
}

/// Remove exact pending nonce after spawn failure. Any other state is refusal.
pub fn abandon_pending(root: &Path, name: &str, nonce: &str) -> Result<()> {
    let lock = owner_lock(root, name)?;
    let mut owners = read_owners(root, name, &lock)?;
    let before = owners.owners.len();
    owners
        .owners
        .retain(|owner| !matches!(owner, VolumeOwner::Pending { nonce: n, .. } if n == nonce));
    if owners.owners.len() == before {
        return Err(LightrError::InvalidRef(format!(
            "volume {name}: pending owner lost"
        )));
    }
    write_owners(root, name, &owners, &lock)
}

/// Mark matching run witness terminal after caller has fsynced terminal status.
pub fn terminal_run_owner(run_dir: &Path) -> Result<()> {
    let mut record = read_run_owner(run_dir)?;
    record.terminal = true;
    write_run_owner(run_dir, &record.volume.clone(), &record.owner, true)
}

/// Release exact active identity only after terminal run witness is durable.
pub fn release_owner(root: &Path, name: &str, run_dir: &Path) -> Result<()> {
    let record = read_run_owner(run_dir)?;
    if record.volume != name || !record.terminal {
        return Err(LightrError::InvalidRef(format!(
            "volume {name}: terminal owner record required"
        )));
    }
    let VolumeOwner::Active {
        nonce,
        run_id,
        process_start_token,
        ..
    } = &record.owner
    else {
        return Err(LightrError::InvalidRef(format!(
            "volume {name}: active owner required"
        )));
    };
    let lock = owner_lock(root, name)?;
    let mut owners = read_owners(root, name, &lock)?;
    let before = owners.owners.len();
    owners.owners.retain(|owner| {
        !matches!(owner, VolumeOwner::Active {
        nonce: n, run_id: r, process_start_token: token, ..
    } if n == nonce && r == run_id && token == process_start_token)
    });
    if owners.owners.len() == before {
        return Err(LightrError::InvalidRef(format!(
            "volume {name}: active owner lost"
        )));
    }
    write_owners(root, name, &owners, &lock)
}

/// Recover only owners with positive death proof. Any missing, malformed, or
/// mismatched run witness is ambiguity: no owner is deleted and `rm`/`prune`
/// callers receive refusal.
pub fn recover(root: &Path, name: &str, home: &Path) -> Result<()> {
    destructive_owner_supported()?;
    let lock = owner_lock(root, name)?;
    recover_locked(root, name, home, &lock)
}

/// Same recovery transition while caller retains the volume flock. Destructive
/// callers use this to make recovery, empty snapshot, and deletion one interval.
fn recover_locked(root: &Path, name: &str, home: &Path, lock: &OwnerLock) -> Result<()> {
    let mut owners = read_owners(root, name, lock)?;
    let mut keep = Vec::with_capacity(owners.owners.len());
    for owner in &owners.owners {
        match owner {
            VolumeOwner::Active {
                nonce: _,
                run_id,
                pid,
                process_start_token,
                ..
            } => {
                let run_dir = home.join("run").join(run_id);
                let record = read_run_owner(&run_dir).map_err(|_| ambiguous(name))?;
                if record.volume != name || record.owner != *owner {
                    return Err(ambiguous(name));
                }
                let terminal = record.terminal
                    && fs::read_to_string(run_dir.join("status"))
                        .map(|status| status.trim_start().starts_with("exited "))
                        .unwrap_or(false);
                if terminal || process_dead(*pid, process_start_token)? {
                    continue;
                }
                keep.push(owner.clone());
            }
            VolumeOwner::Pending {
                nonce,
                coordinator_pid,
                coordinator_start_token,
            } => {
                let active_same_nonce = owners.owners.iter().any(|candidate| {
                    matches!(candidate, VolumeOwner::Active { nonce: active, .. } if active == nonce)
                });
                let record = find_pending_witness(home, name, nonce)?;
                if record.owner != *owner {
                    return Err(ambiguous(name));
                }
                if active_same_nonce || !process_dead(*coordinator_pid, coordinator_start_token)? {
                    keep.push(owner.clone());
                }
            }
        }
    }
    if keep != owners.owners {
        owners.owners = keep;
        write_owners(root, name, &owners, lock)?;
    }
    Ok(())
}

fn ambiguous(name: &str) -> LightrError {
    LightrError::InvalidRef(format!("volume {name}: owner recovery ambiguous"))
}

fn find_pending_witness(home: &Path, volume: &str, nonce: &str) -> Result<RunOwnerRecord> {
    let runs = fs::read_dir(home.join("run")).map_err(|_| ambiguous(volume))?;
    let mut matched = None;
    for entry in runs {
        let entry = entry.map_err(|_| ambiguous(volume))?;
        let record = match read_run_owner(&entry.path()) {
            Ok(record) => record,
            Err(_) => continue,
        };
        if record.volume == volume
            && record.owner.nonce() == nonce
            && matched.replace(record).is_some()
        {
            return Err(ambiguous(volume));
        }
    }
    matched.ok_or_else(|| ambiguous(volume))
}

#[cfg(target_os = "linux")]
fn process_dead(pid: i32, token: &str) -> Result<bool> {
    match process_start_token(pid) {
        Ok(observed) => Ok(observed != token),
        Err(LightrError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => Ok(true),
        Err(_) => Err(LightrError::InvalidRef(
            "process observation ambiguous".to_string(),
        )),
    }
}

#[cfg(not(target_os = "linux"))]
fn process_dead(_pid: i32, _token: &str) -> Result<bool> {
    Err(LightrError::InvalidRef(
        "named-volume recovery unsupported: no stable process start token".to_string(),
    ))
}

fn run_owner_path(run_dir: &Path) -> PathBuf {
    run_dir.join("volume-owner.json")
}

fn write_run_owner(
    run_dir: &Path,
    volume: &str,
    owner: &VolumeOwner,
    terminal: bool,
) -> Result<()> {
    fs::create_dir_all(run_dir)?;
    let bytes = serde_json::to_vec(&RunOwnerRecord {
        volume: volume.to_string(),
        owner: owner.clone(),
        terminal,
    })
    .map_err(|e| LightrError::Io(std::io::Error::other(e)))?;
    let tmp = run_dir.join("volume-owner.json.tmp");
    let mut file = std::fs::File::create(&tmp)?;
    use std::io::Write;
    file.write_all(&bytes)?;
    file.sync_all()?;
    drop(file);
    fs::rename(tmp, run_owner_path(run_dir))?;
    File::open(run_dir)?.sync_all()?;
    Ok(())
}

fn read_run_owner(run_dir: &Path) -> Result<RunOwnerRecord> {
    serde_json::from_slice(&fs::read(run_owner_path(run_dir))?)
        .map_err(|_| LightrError::InvalidManifest("malformed volume-owner.json".to_string()))
}

fn fresh_nonce() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    lightr_core::Digest::of_bytes(
        format!(
            "{}:{now}:{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        )
        .as_bytes(),
    )
    .to_hex()
}

// ── verbs (called from the CLI handler) ───────────────────────────────────────

/// Create a named volume under `root`. Errors if the name is invalid (exit-2
/// class) or if a volume of that name already exists (docker:
/// "volume already exists"). `labels` are stored verbatim, sorted by key.
pub fn create(root: &Path, name: &str, labels: &[(String, String)]) -> Result<VolumeInfo> {
    name_validate(name)?;

    let dir = volume_dir(root, name);
    if dir.exists() {
        return Err(LightrError::InvalidRef(format!(
            "volume already exists: {name}"
        )));
    }

    let mut labels = labels.to_vec();
    labels.sort_by(|a, b| a.0.cmp(&b.0));

    let info = VolumeInfo {
        name: name.to_string(),
        created_at: now_unix(),
        driver: DRIVER_LOCAL.to_string(),
        labels,
        mountpoint: data_dir(root, name),
    };

    // _data/ first (creates the parent volume dir too), then meta.json.
    fs::create_dir_all(data_dir(root, name))?;
    fs::write(meta_path(root, name), meta_json(&info))?;

    Ok(info)
}

/// List all volumes under `root`, sorted by name. A volume dir without a
/// readable/parseable `meta.json` is skipped (fail-soft on a partial dir —
/// listing is a view, not the truth). Absent registry ⇒ empty list.
pub fn list(root: &Path) -> Result<Vec<VolumeInfo>> {
    let vroot = volumes_root(root);
    if !vroot.exists() {
        return Ok(vec![]);
    }

    let mut out: Vec<VolumeInfo> = Vec::new();
    let entries = match fs::read_dir(&vroot) {
        Ok(d) => d,
        Err(_) => return Ok(vec![]),
    };
    for entry in entries.filter_map(|e| e.ok()) {
        if !entry.path().is_dir() {
            continue;
        }
        let name = match entry.file_name().into_string() {
            Ok(n) => n,
            Err(_) => continue, // non-UTF-8 dir name — skip
        };
        if let Ok(info) = read_info(root, &name) {
            out.push(info);
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

/// Inspect one volume. Missing ⇒ `RefNotFound` (docker: "no such volume").
pub fn inspect(root: &Path, name: &str) -> Result<VolumeInfo> {
    name_validate(name)?;
    if !volume_dir(root, name).exists() {
        return Err(LightrError::RefNotFound(format!("no such volume: {name}")));
    }
    read_info(root, name)
}

/// Remove one volume only while its durable active-owner snapshot is empty.
/// `in_use` remains an additional caller-side refusal for legacy callers.
pub fn remove(root: &Path, name: &str, in_use: bool) -> Result<()> {
    destructive_owner_supported()?;
    name_validate(name)?;
    let dir = volume_dir(root, name);
    if !dir.exists() {
        return Err(LightrError::RefNotFound(format!("no such volume: {name}")));
    }
    if in_use {
        return Err(LightrError::InvalidRef(format!("volume is in use: {name}")));
    }
    let home = root.parent().ok_or_else(|| {
        LightrError::InvalidRef(format!("volume {name}: owner recovery ambiguous"))
    })?;
    let lock = owner_lock(root, name)?;
    recover_locked(root, name, home, &lock)?;
    let owners = read_owners(root, name, &lock)?;
    if !owners.owners.is_empty() {
        return Err(LightrError::InvalidRef(format!("volume is in use: {name}")));
    }
    fs::remove_dir_all(&dir)?;
    Ok(())
}

/// Check platform primitives before a CLI creates/prints a detached run id.
/// Linux requires a readable stable process-start token; other targets reject.
pub fn owner_runtime_supported() -> Result<()> {
    process_start_token(std::process::id() as i32).map(|_| ())
}

/// Prune volumes with empty durable ownership snapshots. Busy or ambiguous
/// volumes remain in place; `prune` is deliberately not a broad delete.
pub fn prune(root: &Path) -> Result<Vec<String>> {
    destructive_owner_supported()?;
    let mut removed: Vec<String> = Vec::new();
    for info in list(root)? {
        match remove(root, &info.name, false) {
            Ok(()) => removed.push(info.name),
            Err(LightrError::InvalidRef(_)) => {}
            Err(e) => return Err(e),
        }
    }
    removed.sort();
    Ok(removed)
}

#[cfg(target_os = "linux")]
fn destructive_owner_supported() -> Result<()> {
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn destructive_owner_supported() -> Result<()> {
    Err(LightrError::Unsupported(
        "named-volume remove/prune/recovery requires Linux process identity".to_string(),
    ))
}

// ── meta.json encode / decode (hand-rolled, fixed schema) ─────────────────────

/// The exact bytes written to `meta.json` (the persisted subset — `mountpoint`
/// is derived from `root`, not stored).
fn meta_json(info: &VolumeInfo) -> Vec<u8> {
    let mut s = String::with_capacity(128);
    s.push('{');
    s.push_str("\"name\":");
    s.push_str(&json_str(&info.name));
    s.push_str(",\"created_at\":");
    s.push_str(&info.created_at.to_string());
    s.push_str(",\"driver\":");
    s.push_str(&json_str(&info.driver));
    s.push_str(",\"labels\":");
    s.push_str(&labels_json(&info.labels));
    s.push('}');
    s.into_bytes()
}

/// Encode the labels map as a JSON object.
fn labels_json(labels: &[(String, String)]) -> String {
    let mut s = String::from("{");
    for (i, (k, v)) in labels.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push_str(&json_str(k));
        s.push(':');
        s.push_str(&json_str(v));
    }
    s.push('}');
    s
}

/// Read + decode one volume's `meta.json`, attaching the derived mountpoint.
/// Missing/corrupt meta ⇒ `InvalidManifest`.
fn read_info(root: &Path, name: &str) -> Result<VolumeInfo> {
    let bytes = fs::read(meta_path(root, name))?;
    let text = String::from_utf8(bytes)
        .map_err(|_| LightrError::InvalidManifest(format!("volume {name}: non-UTF-8 meta.json")))?;
    let mut info = parse_meta(&text).ok_or_else(|| {
        LightrError::InvalidManifest(format!("volume {name}: malformed meta.json"))
    })?;
    // mountpoint is derived from `root`, not stored — attach it now.
    info.mountpoint = data_dir(root, name);
    Ok(info)
}

/// Minimal parser for the fixed `meta.json` schema this module writes. Returns
/// `None` on any deviation (fail-closed — we only ever read what we wrote). The
/// `mountpoint` is derived by the caller, so it is left empty here.
fn parse_meta(text: &str) -> Option<VolumeInfo> {
    Some(VolumeInfo {
        name: json_field_str(text, "\"name\":")?,
        created_at: json_field_u64(text, "\"created_at\":")?,
        driver: json_field_str(text, "\"driver\":")?,
        labels: json_field_labels(text)?,
        mountpoint: PathBuf::new(),
    })
}

/// Extract a string field value following `key` (e.g. `"name":`). Handles the
/// `\"` and `\\` escapes this module emits.
fn json_field_str(text: &str, key: &str) -> Option<String> {
    let start = text.find(key)? + key.len();
    let rest = text[start..].trim_start();
    let rest = rest.strip_prefix('"')?;
    let mut out = String::new();
    let mut chars = rest.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => return Some(out),
            '\\' => match chars.next()? {
                '"' => out.push('"'),
                '\\' => out.push('\\'),
                other => out.push(other),
            },
            other => out.push(other),
        }
    }
    None
}

/// Extract an unsigned-integer field value following `key`.
fn json_field_u64(text: &str, key: &str) -> Option<u64> {
    let start = text.find(key)? + key.len();
    let rest = text[start..].trim_start();
    let end = rest
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(rest.len());
    rest[..end].parse::<u64>().ok()
}

/// Parse the `"labels":{ … }` object into sorted key/value pairs.
fn json_field_labels(text: &str) -> Option<Vec<(String, String)>> {
    let start = text.find("\"labels\":")? + "\"labels\":".len();
    let rest = text[start..].trim_start();
    let rest = rest.strip_prefix('{')?;
    let end = rest.find('}')?;
    let body = &rest[..end];
    let mut out: Vec<(String, String)> = Vec::new();
    if body.trim().is_empty() {
        return Some(out);
    }
    // Schema guarantee: label keys/values are simple quoted strings whose only
    // escapes are `\"`/`\\`, joined by top-level `,` with a single `:` per pair
    // (labels come from the CLI as k=v). Split on `,` then the first `:`.
    for pair in body.split(',') {
        let (k, v) = pair.split_once(':')?;
        let k = unquote(k.trim())?;
        let v = unquote(v.trim())?;
        out.push((k, v));
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Some(out)
}

/// Strip surrounding quotes + unescape a JSON string token.
fn unquote(tok: &str) -> Option<String> {
    let inner = tok.strip_prefix('"')?.strip_suffix('"')?;
    let mut out = String::new();
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => match chars.next()? {
                '"' => out.push('"'),
                '\\' => out.push('\\'),
                other => out.push(other),
            },
            other => out.push(other),
        }
    }
    Some(out)
}

/// JSON-escape a string and wrap it in quotes. Names are a restricted charset;
/// labels are arbitrary user strings, so we escape `"` and `\` defensively.
fn json_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

/// Wall-clock now, unix seconds (0 if the clock is before the epoch).
fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

// ─────────────────────────────────── tests ──────────────────────────────────

#[cfg(test)]
#[path = "volume_tests.rs"]
mod tests;
