//! Tests for the named-volume registry (WP-VOL-4). Split out of `volume.rs` to
//! keep each file under the 400-LOC house limit. Included as a `#[path]` child
//! module of `volume`, so `super::*` resolves to the registry API. Parallel-safe:
//! each test owns a unique tempdir root (atomic counter + nanos) — NO global env.

use super::*;
use lightr_core::LightrError;
use std::sync::atomic::{AtomicU64, Ordering};
use tempfile::TempDir;

// Unique per-test root (atomic counter + nanos) — NO global env, parallel-safe.
static COUNTER: AtomicU64 = AtomicU64::new(0);

fn tmp_root() -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = dir.path().join(format!("store-{n}-{nanos}"));
    fs::create_dir_all(&root).unwrap();
    (dir, root)
}

#[test]
fn create_makes_data_dir_and_meta() {
    let (_d, root) = tmp_root();
    let info = create(&root, "data", &[]).unwrap();
    assert_eq!(info.name, "data");
    assert_eq!(info.driver, "local");
    assert!(data_dir(&root, "data").is_dir(), "_data/ must exist");
    assert!(meta_path(&root, "data").is_file(), "meta.json must exist");
    assert_eq!(info.mountpoint, data_dir(&root, "data"));
}

#[test]
fn create_with_labels_roundtrips_sorted() {
    let (_d, root) = tmp_root();
    let labels = vec![
        ("zone".to_string(), "b".to_string()),
        ("app".to_string(), "web".to_string()),
    ];
    create(&root, "vol", &labels).unwrap();
    let got = inspect(&root, "vol").unwrap();
    assert_eq!(
        got.labels,
        vec![
            ("app".to_string(), "web".to_string()),
            ("zone".to_string(), "b".to_string()),
        ],
        "labels must roundtrip sorted by key"
    );
}

#[test]
fn create_existing_errors() {
    let (_d, root) = tmp_root();
    create(&root, "dup", &[]).unwrap();
    let err = create(&root, "dup", &[]).unwrap_err();
    assert!(matches!(err, LightrError::InvalidRef(_)));
    assert!(err.to_string().contains("already exists"));
}

#[test]
fn create_bad_name_rejected() {
    let (_d, root) = tmp_root();
    for bad in ["", "has space", "-leading", "bad/slash", "tab\tname"] {
        let err = create(&root, bad, &[]).unwrap_err();
        assert!(
            matches!(err, LightrError::InvalidRef(_)),
            "name {bad:?} must be rejected"
        );
    }
    // No partial dir left behind for the invalid names.
    assert!(list(&root).unwrap().is_empty());
}

#[test]
fn list_sorted_and_empty() {
    let (_d, root) = tmp_root();
    assert!(
        list(&root).unwrap().is_empty(),
        "empty registry ⇒ empty list"
    );
    create(&root, "beta", &[]).unwrap();
    create(&root, "alpha", &[]).unwrap();
    let names: Vec<String> = list(&root).unwrap().into_iter().map(|v| v.name).collect();
    assert_eq!(names, vec!["alpha", "beta"], "list must be sorted by name");
}

#[test]
fn inspect_missing_errors() {
    let (_d, root) = tmp_root();
    let err = inspect(&root, "ghost").unwrap_err();
    assert!(matches!(err, LightrError::RefNotFound(_)));
}

#[test]
fn inspect_json_has_fields() {
    let (_d, root) = tmp_root();
    create(&root, "j", &[("k".to_string(), "v".to_string())]).unwrap();
    let json = inspect(&root, "j").unwrap().to_json();
    assert!(json.contains("\"name\":\"j\""), "json: {json}");
    assert!(json.contains("\"driver\":\"local\""), "json: {json}");
    assert!(json.contains("\"mountpoint\":"), "json: {json}");
    assert!(json.contains("\"k\":\"v\""), "json: {json}");
}

#[test]
fn remove_then_gone() {
    let (_d, root) = tmp_root();
    create(&root, "tmp", &[]).unwrap();
    remove(&root, "tmp", false).unwrap();
    assert!(!volume_dir(&root, "tmp").exists());
    assert!(inspect(&root, "tmp").is_err());
}

#[test]
fn remove_missing_errors() {
    let (_d, root) = tmp_root();
    let err = remove(&root, "nope", false).unwrap_err();
    assert!(matches!(err, LightrError::RefNotFound(_)));
}

#[test]
fn remove_in_use_refused() {
    let (_d, root) = tmp_root();
    create(&root, "busy", &[]).unwrap();
    let err = remove(&root, "busy", true).unwrap_err();
    assert!(matches!(err, LightrError::InvalidRef(_)));
    assert!(err.to_string().contains("in use"));
    // Still present — refusal must not delete.
    assert!(volume_dir(&root, "busy").exists());
}

#[test]
fn prune_removes_dangling() {
    let (_d, root) = tmp_root();
    create(&root, "a", &[]).unwrap();
    create(&root, "b", &[]).unwrap();
    let removed = prune(&root).unwrap();
    assert_eq!(
        removed,
        vec!["a", "b"],
        "prune returns removed names sorted"
    );
    assert!(list(&root).unwrap().is_empty(), "all dangling removed");
}

#[test]
fn prune_empty_registry_ok() {
    let (_d, root) = tmp_root();
    assert!(prune(&root).unwrap().is_empty());
}

#[test]
fn meta_json_escapes_label_values() {
    let (_d, root) = tmp_root();
    create(&root, "esc", &[("note".to_string(), "a\"b\\c".to_string())]).unwrap();
    let got = inspect(&root, "esc").unwrap();
    assert_eq!(
        got.labels,
        vec![("note".to_string(), "a\"b\\c".to_string())],
        "escaped quote/backslash must roundtrip"
    );
}

#[test]
fn run_owner_record_roundtrips_strict_owner_shape() {
    let record = RunOwnerRecord {
        volume: "data".to_string(),
        owner: VolumeOwner::Active {
            nonce: "a".repeat(64),
            run_id: "run".to_string(),
            pid: 42,
            process_start_token: "linux:42:99".to_string(),
            mount_id: "data:target".to_string(),
        },
        terminal: false,
    };
    let bytes = serde_json::to_vec(&record).unwrap();
    assert_eq!(serde_json::from_slice::<RunOwnerRecord>(&bytes).unwrap(), record);
}

#[cfg(target_os = "linux")]
#[test]
fn owner_pending_active_terminal_release_is_exact() {
    let (_d, root) = tmp_root();
    create(&root, "owned", &[]).unwrap();
    let run = root.join("run-1");
    let pending = begin_owner(&root, "owned", &run).unwrap();
    let nonce = pending.nonce().to_string();
    let active = activate_owner(
        &root,
        "owned",
        &run,
        &nonce,
        "run-1",
        std::process::id() as i32,
        "mount-1",
    )
    .unwrap();
    let lock = owner_lock(&root, "owned").unwrap();
    assert_eq!(read_owners(&root, "owned", &lock).unwrap().owners, vec![active]);
    drop(lock);
    assert!(remove(&root, "owned", false).is_err(), "active owner blocks rm");
    terminal_run_owner(&run).unwrap();
    release_owner(&root, "owned", &run).unwrap();
    remove(&root, "owned", false).unwrap();
}

#[cfg(target_os = "linux")]
#[test]
fn recovery_removes_only_terminal_matching_owner() {
    let (temp, _ignored) = tmp_root();
    let home = temp.path().join("home");
    let root = home.join("store");
    fs::create_dir_all(&root).unwrap();
    create(&root, "recover", &[]).unwrap();
    let run = home.join("run/r1");
    let pending = begin_owner(&root, "recover", &run).unwrap();
    activate_owner(
        &root,
        "recover",
        &run,
        pending.nonce(),
        "r1",
        std::process::id() as i32,
        "m1",
    )
    .unwrap();
    fs::write(run.join("status"), "exited 0").unwrap();
    terminal_run_owner(&run).unwrap();
    recover(&root, "recover", &home).unwrap();
    let lock = owner_lock(&root, "recover").unwrap();
    assert!(read_owners(&root, "recover", &lock).unwrap().owners.is_empty());
}

#[cfg(target_os = "linux")]
#[test]
fn recovery_refuses_missing_run_witness() {
    let (temp, _ignored) = tmp_root();
    let home = temp.path().join("home");
    let root = home.join("store");
    fs::create_dir_all(&root).unwrap();
    create(&root, "ambiguous", &[]).unwrap();
    let run = home.join("run/r2");
    let pending = begin_owner(&root, "ambiguous", &run).unwrap();
    activate_owner(
        &root,
        "ambiguous",
        &run,
        pending.nonce(),
        "r2",
        std::process::id() as i32,
        "m2",
    )
    .unwrap();
    fs::remove_file(run.join("volume-owner.json")).unwrap();
    assert!(recover(&root, "ambiguous", &home).is_err());
    assert!(remove(&root, "ambiguous", false).is_err());
    assert!(volume_dir(&root, "ambiguous").exists());
}

#[cfg(target_os = "linux")]
#[test]
fn sigkill_owner_recovers_after_matching_process_death_proof() {
    use std::io::Read;
    use std::os::fd::{FromRawFd, RawFd};

    let (temp, _ignored) = tmp_root();
    let home = temp.path().join("home");
    let root = home.join("store");
    fs::create_dir_all(&root).unwrap();
    create(&root, "killed", &[]).unwrap();
    let run = home.join("run/killed-run");
    let mut fds: [RawFd; 2] = [0; 2];
    assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
    let child = unsafe { libc::fork() };
    assert!(child >= 0);
    if child == 0 {
        unsafe { libc::close(fds[0]) };
        let pending = begin_owner(&root, "killed", &run).unwrap();
        activate_owner(
            &root,
            "killed",
            &run,
            pending.nonce(),
            "killed-run",
            std::process::id() as i32,
            "m-killed",
        )
        .unwrap();
        let mut ready = unsafe { std::fs::File::from_raw_fd(fds[1]) };
        use std::io::Write;
        ready.write_all(&[1]).unwrap();
        loop {
            std::thread::park();
        }
    }
    unsafe { libc::close(fds[1]) };
    let mut ready = unsafe { std::fs::File::from_raw_fd(fds[0]) };
    let mut byte = [0; 1];
    ready.read_exact(&mut byte).unwrap();
    assert_eq!(unsafe { libc::kill(child, libc::SIGKILL) }, 0);
    let mut status = 0;
    assert_eq!(unsafe { libc::waitpid(child, &mut status, 0) }, child);
    recover(&root, "killed", &home).unwrap();
    let lock = owner_lock(&root, "killed").unwrap();
    assert!(read_owners(&root, "killed", &lock).unwrap().owners.is_empty());
}

#[cfg(target_os = "linux")]
#[test]
fn shared_active_owners_require_each_exact_terminal_release() {
    let (temp, _ignored) = tmp_root();
    let home = temp.path().join("home");
    let root = home.join("store");
    fs::create_dir_all(&root).unwrap();
    create(&root, "shared", &[]).unwrap();
    for id in ["one", "two"] {
        let run = home.join("run").join(id);
        let pending = begin_owner(&root, "shared", &run).unwrap();
        activate_owner(
            &root,
            "shared",
            &run,
            pending.nonce(),
            id,
            std::process::id() as i32,
            &format!("m-{id}"),
        )
        .unwrap();
    }
    let one = home.join("run/one");
    fs::write(one.join("status"), "exited 0").unwrap();
    terminal_run_owner(&one).unwrap();
    release_owner(&root, "shared", &one).unwrap();
    assert!(remove(&root, "shared", false).is_err());
    let two = home.join("run/two");
    fs::write(two.join("status"), "exited 0").unwrap();
    terminal_run_owner(&two).unwrap();
    release_owner(&root, "shared", &two).unwrap();
    remove(&root, "shared", false).unwrap();
}

#[cfg(target_os = "linux")]
#[test]
fn failed_spawn_abandons_only_its_pending_nonce() {
    let (temp, _ignored) = tmp_root();
    let home = temp.path().join("home");
    let root = home.join("store");
    fs::create_dir_all(&root).unwrap();
    create(&root, "failed", &[]).unwrap();
    let first = begin_owner(&root, "failed", &home.join("run/one")).unwrap();
    let second = begin_owner(&root, "failed", &home.join("run/two")).unwrap();
    abandon_pending(&root, "failed", first.nonce()).unwrap();
    let lock = owner_lock(&root, "failed").unwrap();
    let owners = read_owners(&root, "failed", &lock).unwrap();
    assert_eq!(owners.owners.len(), 1);
    assert_eq!(owners.owners[0].nonce(), second.nonce());
}
