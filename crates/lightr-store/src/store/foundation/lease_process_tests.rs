//! Real child-process probes. Handshakes determine ordering; timeouts only bound
//! test failure. Environment variables affect this test helper, never production.
use super::{CacheLocks, StoreLocks, Wait};
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};
use tempfile::TempDir;

const CHILD: &str = "store::foundation::lease_process_tests::lease_child_probe";
const ROOT_ENV: &str = "LIGHTR_SI01_LEASE_CHILD_ROOT";

// This method is an IPC helper when launched with an isolated child environment.
// Its ordinary invocation is not counted as an independent concurrency witness.
#[test]
fn lease_child_probe() {
    let Some(root) = std::env::var_os(ROOT_ENV) else {
        return;
    };
    let mode = std::env::var("LIGHTR_SI01_LEASE_CHILD_MODE").unwrap();
    let marker = std::env::var_os("LIGHTR_SI01_LEASE_CHILD_READY").unwrap();
    if mode == "cache" {
        let cache = CacheLocks::open_existing(Path::new(&root)).unwrap();
        let _held = cache.exclusive(Wait::Try).unwrap();
        fs::write(marker, b"held").unwrap();
        let mut line = String::new();
        io::stdin().read_line(&mut line).unwrap();
        assert_eq!(line.trim(), "release");
    } else {
        let domain = StoreLocks::open_existing(Path::new(&root)).unwrap();
        if mode == "expect-blocked" {
            assert_eq!(
                domain.exclusive(Wait::Try).unwrap_err().kind(),
                io::ErrorKind::WouldBlock
            );
            fs::write(marker, b"blocked").unwrap();
        } else {
            let _held = domain.shared(Wait::Try).unwrap();
            fs::write(marker, b"held").unwrap();
            let mut line = String::new();
            io::stdin().read_line(&mut line).unwrap();
            assert_eq!(line.trim(), "release");
        }
    }
}

struct ChildProbe(Child);
impl Drop for ChildProbe {
    fn drop(&mut self) {
        // A failing parent must not strand a child holding a native lock.
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
impl ChildProbe {
    fn launch(root: &Path, marker: &Path, mode: &str) -> Self {
        Self(
            Command::new(std::env::current_exe().unwrap())
                .args(["--exact", CHILD, "--nocapture"])
                .env(ROOT_ENV, root)
                .env("LIGHTR_SI01_LEASE_CHILD_MODE", mode)
                .env("LIGHTR_SI01_LEASE_CHILD_READY", marker)
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::inherit())
                .spawn()
                .unwrap(),
        )
    }
    fn ready(&mut self, marker: &Path) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while !marker.exists() {
            if let Some(status) = self.0.try_wait().unwrap() {
                panic!("child exited before acquiring its lock: {status}");
            }
            assert!(
                Instant::now() < deadline,
                "child handshake watchdog expired"
            );
            std::thread::park_timeout(Duration::from_millis(2));
        }
    }
    fn finish(&mut self, release: bool) {
        if release {
            self.0
                .stdin
                .take()
                .unwrap()
                .write_all(b"release\n")
                .unwrap();
        }
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Some(status) = self.0.try_wait().unwrap() {
                assert!(status.success(), "child failed: {status}");
                return;
            }
            assert!(
                Instant::now() < deadline,
                "child completion watchdog expired"
            );
            std::thread::park_timeout(Duration::from_millis(2));
        }
    }
}

#[test]
fn lease_cross_process_shared_and_exclusive_contend_both_directions() {
    let root = TempDir::new().unwrap();
    let signals = TempDir::new().unwrap();
    let domain = StoreLocks::open_existing(root.path()).unwrap();
    let marker = signals.path().join("first");
    let mut child = ChildProbe::launch(root.path(), &marker, "shared");
    child.ready(&marker);
    let shared = domain.shared(Wait::Try).unwrap();
    assert_eq!(
        domain.exclusive(Wait::Try).unwrap_err().kind(),
        io::ErrorKind::WouldBlock
    );
    child.finish(true);
    drop(shared);
    let exclusive = domain.exclusive(Wait::Try).unwrap();
    let marker = signals.path().join("second");
    let mut child = ChildProbe::launch(root.path(), &marker, "expect-blocked");
    child.finish(false);
    assert_eq!(fs::read(marker).unwrap(), b"blocked");
    drop(exclusive);
    assert!(domain.shared(Wait::Try).is_ok());
}

#[test]
fn lease_killed_holder_releases_only_its_native_lock() {
    let root = TempDir::new().unwrap();
    let signals = TempDir::new().unwrap();
    let domain = StoreLocks::open_existing(root.path()).unwrap();
    let marker = signals.path().join("held");
    let mut child = ChildProbe::launch(root.path(), &marker, "shared");
    child.ready(&marker);
    assert_eq!(
        domain.exclusive(Wait::Try).unwrap_err().kind(),
        io::ErrorKind::WouldBlock
    );
    drop(child); // kill + wait, never an unlink of the lock file.
    assert!(domain.exclusive(Wait::Try).is_ok());
    assert!(root.path().join(".gc.lock").is_file());
}

#[test]
fn lease_cache_is_shared_across_processes_and_foreign_stores() {
    let store = TempDir::new().unwrap();
    let cache_root = TempDir::new().unwrap();
    let signals = TempDir::new().unwrap();
    let domain = StoreLocks::open_existing(store.path()).unwrap();
    let cache = CacheLocks::open_existing(cache_root.path()).unwrap();
    let marker = signals.path().join("cache-held");
    let mut child = ChildProbe::launch(cache_root.path(), &marker, "cache");
    child.ready(&marker);
    let foreign = domain.exclusive(Wait::Try).unwrap();
    assert_eq!(
        cache.exclusive(Wait::Try).unwrap_err().kind(),
        io::ErrorKind::WouldBlock
    );
    child.finish(true);
    assert!(cache.exclusive(Wait::Try).is_ok());
    drop(foreign);
}
