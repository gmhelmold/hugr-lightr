//! Linux-only ownership integration. Runs the real native supervisor and an
//! external `/bin/sh` child, so `cargo test --workspace --lib --bins` exercises
//! the owner barrier and terminal release path on CI.

use crate::run::paths::write_spec_json;
use crate::run::supervise::supervise;
use crate::run::types::{MountOnDisk2, SpecOnDisk};
use lightr_store::{volume, Store};
use std::fs;
use std::time::{Duration, Instant};

fn home() -> (tempfile::TempDir, std::sync::MutexGuard<'static, ()>) {
    let guard = crate::run::tests::ENV_LOCK
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let home = tempfile::tempdir().unwrap();
    std::env::set_var("LIGHTR_HOME", home.path());
    let gate = home.path().join("volume-gate.sh");
    fs::write(
        &gate,
        r#"#!/bin/sh
[ "$1" = __volume_gate ] || exit 127
fd="$2"
[ "$3" = -- ] || exit 127
shift 3
byte=$(dd bs=1 count=1 < "/proc/self/fd/$fd" 2>/dev/null)
[ "$byte" = "$(printf '\001')" ] || exit 127
eval "exec $fd<&-"
exec "$@"
"#,
    )
    .unwrap();
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&gate, fs::Permissions::from_mode(0o700)).unwrap();
    std::env::set_var("LIGHTR_VOLUME_GATE_SHIM", gate);
    (home, guard)
}

#[test]
fn generated_gate_fixture_enforces_marker_and_release_byte() {
    use std::os::fd::{FromRawFd, RawFd};
    let (home, _guard) = home();
    let gate = home.path().join("volume-gate.sh");
    let script = fs::read_to_string(&gate).unwrap();
    assert!(
        script.contains("\"$1\"")
            && script.contains("\"$2\"")
            && script.contains("\"$3\"")
            && script.contains("\"$@\"")
    );
    let mut fds: [RawFd; 2] = [0; 2];
    assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
    let mut write = unsafe { std::fs::File::from_raw_fd(fds[1]) };
    let wrong = std::process::Command::new(&gate)
        .args(["wrong", &fds[0].to_string(), "/bin/true"])
        .status()
        .unwrap();
    assert_eq!(wrong.code(), Some(127));
    let wrong_delimiter = std::process::Command::new(&gate)
        .args(["__volume_gate", &fds[0].to_string(), "wrong", "/bin/true"])
        .status()
        .unwrap();
    assert_eq!(wrong_delimiter.code(), Some(127));
    let mut child = std::process::Command::new(&gate)
        .args([
            "__volume_gate",
            &fds[0].to_string(),
            "--",
            "/bin/sh",
            "-c",
            "exit 23",
        ])
        .spawn()
        .unwrap();
    use std::io::Write;
    write.write_all(&[1]).unwrap();
    assert_eq!(child.wait().unwrap().code(), Some(23));
    unsafe { libc::close(fds[0]) };
}

fn start(
    home: &std::path::Path,
    cwd: &std::path::Path,
    id: &str,
    volume: &str,
    target: &str,
    script: &str,
) -> (std::path::PathBuf, std::thread::JoinHandle<i32>) {
    let store = Store::open(home.join("store")).unwrap();
    if volume::inspect(store.root(), volume).is_err() {
        volume::create(store.root(), volume, &[]).unwrap();
    }
    let dir = home.join("run").join(id);
    fs::create_dir_all(&dir).unwrap();
    write_spec_json(
        &dir,
        &SpecOnDisk {
            cwd: cwd.to_string_lossy().into_owned(),
            command: vec!["/bin/sh".to_string(), "-c".to_string(), script.to_string()],
            mounts2: vec![MountOnDisk2::NamedVolume {
                source: volume.to_string(),
                target: target.to_string(),
                readonly: false,
            }],
            engine: "native".to_string(),
            ..Default::default()
        },
    )
    .unwrap();
    let copy = dir.clone();
    let thread = std::thread::spawn(move || supervise(&copy).unwrap());
    (dir, thread)
}

fn wait_for(path: &std::path::Path) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !path.exists() {
        assert!(
            Instant::now() < deadline,
            "timed out waiting for {}",
            path.display()
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn named_volume_child_cannot_exec_before_owner_activation() {
    let (home, _guard) = home();
    let cwd = tempfile::tempdir().unwrap();
    let owner = home.path().join("store/volumes/data/.lightr/owners.json");
    let output = home.path().join("store/volumes/data/_data/output");
    let (dir, supervisor) = start(
        home.path(),
        cwd.path(),
        "barrier",
        "data",
        "mounted",
        &format!(
            "grep -q '\"phase\":\"active\"' '{}' && printf ok > mounted/output",
            owner.display()
        ),
    );
    wait_for(&dir.join("volume-owner.json"));
    assert_eq!(supervisor.join().unwrap(), 0);
    assert_eq!(fs::read_to_string(output).unwrap(), "ok");
    volume::remove(&home.path().join("store"), "data", false).unwrap();
}

#[test]
fn shared_named_mounts_hold_two_owners_then_release_exactly() {
    let (home, _guard) = home();
    let cwd = tempfile::tempdir().unwrap();
    let store = home.path().join("store");
    let (one, first) = start(home.path(), cwd.path(), "one", "shared", "one", "sleep 1");
    let (two, second) = start(home.path(), cwd.path(), "two", "shared", "two", "sleep 1");
    let owners = store.join("volumes/shared/.lightr/owners.json");
    let deadline = Instant::now() + Duration::from_secs(10);
    while fs::read_to_string(&owners)
        .unwrap_or_default()
        .matches("\"phase\":\"active\"")
        .count()
        < 2
    {
        assert!(
            Instant::now() < deadline,
            "two active owners were not published"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(volume::remove(&store, "shared", false).is_err());
    assert_eq!(first.join().unwrap(), 0);
    assert_eq!(second.join().unwrap(), 0);
    assert!(one.join("status").exists() && two.join("status").exists());
    volume::remove(&store, "shared", false).unwrap();
}

#[test]
fn concurrent_remove_and_prune_cannot_delete_live_named_mount() {
    let (home, _guard) = home();
    let cwd = tempfile::tempdir().unwrap();
    let store = home.path().join("store");
    let (dir, supervisor) = start(
        home.path(),
        cwd.path(),
        "locked",
        "locked",
        "mounted",
        "sleep 1",
    );
    wait_for(&dir.join("volume-owner.json"));
    std::thread::scope(|scope| {
        let remove = scope.spawn(|| volume::remove(&store, "locked", false));
        let prune = scope.spawn(|| volume::prune(&store));
        assert!(remove.join().unwrap().is_err());
        assert!(prune.join().unwrap().is_ok());
    });
    assert!(store.join("volumes/locked").exists());
    assert_eq!(supervisor.join().unwrap(), 0);
    volume::remove(&store, "locked", false).unwrap();
}

#[test]
fn missing_named_volume_fails_before_owner_artifact() {
    let (home, _guard) = home();
    let cwd = tempfile::tempdir().unwrap();
    let dir = home.path().join("run/missing");
    fs::create_dir_all(&dir).unwrap();
    Store::open(home.path().join("store")).unwrap();
    write_spec_json(
        &dir,
        &SpecOnDisk {
            cwd: cwd.path().to_string_lossy().into_owned(),
            command: vec!["/bin/true".to_string()],
            mounts2: vec![MountOnDisk2::NamedVolume {
                source: "absent".to_string(),
                target: "mounted".to_string(),
                readonly: false,
            }],
            engine: "native".to_string(),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(supervise(&dir).is_err());
    assert!(!home.path().join("store/volumes/absent").exists());
    assert!(!dir.join("volume-owner.json").exists());
}
