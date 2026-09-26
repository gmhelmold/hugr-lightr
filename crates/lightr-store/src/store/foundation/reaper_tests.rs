use super::reaper::reap_owned_scratch;
use super::ScratchDirectory;
use crate::store::foundation::{CacheLocks, StoreLocks, Wait};
use std::fs;
use std::io;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use tempfile::TempDir;

const CHILD: &str = "store::foundation::anchored_scratch::reaper_tests::reaper_child_probe";
const ROOT_ENV: &str = "LIGHTR_SI01_REAPER_CHILD_ROOT";
const READY_ENV: &str = "LIGHTR_SI01_REAPER_CHILD_READY";

#[test]
fn reaper_child_probe() {
    let (Some(root), Some(ready)) = (std::env::var_os(ROOT_ENV), std::env::var_os(READY_ENV))
    else {
        return;
    };
    let domain = StoreLocks::open_existing(Path::new(&root)).unwrap();
    let lease = domain.shared(Wait::Try).unwrap();
    let _scratch = ScratchDirectory::reserve(&lease, Wait::Try).unwrap();
    fs::write(ready, b"reserved").unwrap();
    let mut line = String::new();
    io::stdin().read_line(&mut line).unwrap();
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn reaper_skips_unknown_and_legacy_scratch() {
    let root = TempDir::new().unwrap();
    let domain = StoreLocks::open_existing(root.path()).unwrap();
    let staging = root.path().join(".si01-staging");
    fs::create_dir_all(staging.join(".tmp-legacy-tree-0")).unwrap();
    fs::write(staging.join(".tmp-legacy-tree-0/retained"), b"keep").unwrap();
    fs::create_dir(staging.join("unknown")).unwrap();
    let guard = domain.exclusive(Wait::Try).unwrap();

    assert_eq!(reap_owned_scratch(&domain, &guard).unwrap(), 0);
    assert_eq!(
        fs::read(staging.join(".tmp-legacy-tree-0/retained")).unwrap(),
        b"keep"
    );
    assert!(staging.join("unknown").is_dir());
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn reaper_requires_matching_exclusive_guard_and_preserves_live_helper() {
    let root = TempDir::new().unwrap();
    let other = TempDir::new().unwrap();
    let domain = StoreLocks::open_existing(root.path()).unwrap();
    let foreign = StoreLocks::open_existing(other.path()).unwrap();
    let lease = domain.shared(Wait::Try).unwrap();
    let scratch = ScratchDirectory::reserve(&lease, Wait::Try).unwrap();
    assert_eq!(
        domain.exclusive(Wait::Try).unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    drop(scratch);
    drop(lease);

    let foreign_guard = foreign.exclusive(Wait::Try).unwrap();
    assert_eq!(
        reap_owned_scratch(&domain, &foreign_guard)
            .unwrap_err()
            .kind(),
        std::io::ErrorKind::InvalidInput
    );
    assert!(root.path().join(".si01-staging").is_dir());
}

#[test]
fn cache_workers_from_two_stores_contend_with_standalone_cache() {
    let one = TempDir::new().unwrap();
    let two = TempDir::new().unwrap();
    let cache = TempDir::new().unwrap();
    let store_one = StoreLocks::open_existing(one.path()).unwrap();
    let store_two = StoreLocks::open_existing(two.path()).unwrap();
    let cache_one = CacheLocks::open_existing(cache.path()).unwrap();
    let cache_two = CacheLocks::open_existing(&cache.path().join(".")).unwrap();
    let lease_one = store_one.shared(Wait::Try).unwrap();
    let lease_two = store_two.shared(Wait::Try).unwrap();
    let mut worker_one = lease_one.worker();
    let mut worker_two = lease_two.worker();

    let held = worker_one.cache(&cache_one, Wait::Try).unwrap();
    assert_eq!(
        worker_two.cache(&cache_two, Wait::Try).unwrap_err().kind(),
        io::ErrorKind::WouldBlock
    );
    drop(held);
    let held = worker_two.cache(&cache_two, Wait::Try).unwrap();
    assert_eq!(
        cache_one.exclusive(Wait::Try).unwrap_err().kind(),
        io::ErrorKind::WouldBlock
    );
    drop(held);
    assert!(cache_one.exclusive(Wait::Try).is_ok());
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn reaper_removes_owned_stale_scratch_after_owner_dies() {
    let root = TempDir::new().unwrap();
    let signals = TempDir::new().unwrap();
    let domain = StoreLocks::open_existing(root.path()).unwrap();
    let ready = signals.path().join("reserved");
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", CHILD, "--nocapture"])
        .env(ROOT_ENV, root.path())
        .env(READY_ENV, &ready)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !ready.exists() {
        if let Some(status) = child.try_wait().unwrap() {
            panic!("child exited before reserving scratch: {status}");
        }
        assert!(
            Instant::now() < deadline,
            "child handshake watchdog expired"
        );
        std::thread::park_timeout(Duration::from_millis(2));
    }
    let staging = root.path().join(".si01-staging");
    assert_eq!(fs::read_dir(&staging).unwrap().count(), 1);
    child.kill().unwrap();
    assert!(!child.wait().unwrap().success());
    let guard = domain.exclusive(Wait::Try).unwrap();

    assert_eq!(reap_owned_scratch(&domain, &guard).unwrap(), 1);
    assert_eq!(fs::read_dir(staging).unwrap().count(), 0);
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn reaper_preserves_replacement_after_ownership_validation() {
    let root = TempDir::new().unwrap();
    let signals = TempDir::new().unwrap();
    let domain = StoreLocks::open_existing(root.path()).unwrap();
    let ready = signals.path().join("reserved");
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", CHILD, "--nocapture"])
        .env(ROOT_ENV, root.path())
        .env(READY_ENV, &ready)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !ready.exists() {
        if let Some(status) = child.try_wait().unwrap() {
            panic!("child exited before reserving scratch: {status}");
        }
        assert!(
            Instant::now() < deadline,
            "child handshake watchdog expired"
        );
        std::thread::park_timeout(Duration::from_millis(2));
    }
    let staging_path = root.path().join(".si01-staging");
    let stale = fs::read_dir(&staging_path)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    child.kill().unwrap();
    assert!(!child.wait().unwrap().success());
    let guard = domain.exclusive(Wait::Try).unwrap();
    let staging = guard.staging().unwrap();
    let replacement = root.path().join("retained-stale");

    let error = super::native::reap_owned_scratch(
        staging.anchored_handle(),
        stale.file_name().unwrap(),
        || {
            fs::rename(&stale, &replacement).unwrap();
            fs::create_dir(&stale).unwrap();
            fs::write(stale.join("sentinel"), b"retain").unwrap();
        },
    )
    .unwrap_err();

    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert_eq!(fs::read(stale.join("sentinel")).unwrap(), b"retain");
    assert!(replacement.is_dir());
}
