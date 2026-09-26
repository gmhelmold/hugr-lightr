use super::reaper::reap_owned_scratch;
use super::ScratchDirectory;
use crate::store::foundation::{StoreLocks, Wait};
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
