#![cfg(target_os = "linux")]

use std::fs;
use std::process::{Command, Output};
use std::thread;
use std::time::{Duration, Instant};

use tempfile::TempDir;

fn lightr(home: &TempDir, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_lightr"))
        .args(args)
        .env("LIGHTR_HOME", home.path())
        .output()
        .expect("run lightr")
}

fn ok(home: &TempDir, args: &[&str]) -> String {
    let output = lightr(home, args);
    assert!(
        output.status.success(),
        "{:?}: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

fn start(home: &TempDir, work: &std::path::Path, volume: &str, command: &str) -> String {
    ok(
        home,
        &[
            "run",
            "--detach",
            "--dir",
            work.to_str().unwrap(),
            "--volume",
            volume,
            "--",
            "/bin/sh",
            "-c",
            command,
        ],
    )
    .trim()
    .to_string()
}

fn await_path(path: &std::path::Path) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !path.exists() {
        assert!(
            Instant::now() < deadline,
            "timed out waiting for {}",
            path.display()
        );
        thread::sleep(Duration::from_millis(20));
    }
}

fn await_exit(home: &TempDir, id: &str) {
    let status = home.path().join("run").join(id).join("status");
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if fs::read_to_string(&status)
            .map(|s| s.starts_with("exited "))
            .unwrap_or(false)
        {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for {}",
            status.display()
        );
        thread::sleep(Duration::from_millis(20));
    }
}

fn kill_supervisor_and_child(pid: i32) {
    let ppid: i32 = fs::read_to_string(format!("/proc/{pid}/status"))
        .unwrap()
        .lines()
        .find_map(|line| line.strip_prefix("PPid:\t"))
        .unwrap()
        .parse()
        .unwrap();
    unsafe { libc::kill(ppid, libc::SIGKILL) };
    unsafe { libc::kill(pid, libc::SIGKILL) };
    thread::sleep(Duration::from_millis(100));
}

#[test]
fn detached_named_volume_releases_after_terminal_status() {
    let home = TempDir::new().unwrap();
    let work = TempDir::new().unwrap();
    ok(&home, &["volume", "create", "data"]);
    let id = start(
        &home,
        work.path(),
        "data:mounted",
        "grep -q '\"phase\":\"active\"' \"$LIGHTR_HOME/store/volumes/data/.lightr/owners.json\" && printf payload > mounted/result",
    );
    await_exit(&home, &id);
    assert_eq!(
        fs::read_to_string(home.path().join("store/volumes/data/_data/result")).unwrap(),
        "payload"
    );
    ok(&home, &["volume", "rm", "data"]);
}

#[test]
fn shared_mounts_block_rm_until_each_run_releases() {
    let home = TempDir::new().unwrap();
    let work = TempDir::new().unwrap();
    ok(&home, &["volume", "create", "shared"]);
    let first = start(&home, work.path(), "shared:one", "sleep 1");
    let second = start(&home, work.path(), "shared:two", "sleep 1");
    let owners = home.path().join("store/volumes/shared/.lightr/owners.json");
    await_path(&owners);
    let deadline = Instant::now() + Duration::from_secs(10);
    while fs::read_to_string(&owners)
        .unwrap()
        .matches("\"phase\":\"active\"")
        .count()
        < 2
    {
        assert!(
            Instant::now() < deadline,
            "timed out waiting for two shared owners"
        );
        thread::sleep(Duration::from_millis(20));
    }
    let refused = lightr(&home, &["volume", "rm", "shared"]);
    assert!(!refused.status.success(), "rm accepted live shared mount");
    await_exit(&home, &first);
    await_exit(&home, &second);
    ok(&home, &["volume", "rm", "shared"]);
}

#[test]
fn killed_child_recovers_only_after_positive_death_proof() {
    let home = TempDir::new().unwrap();
    let work = TempDir::new().unwrap();
    ok(&home, &["volume", "create", "crash"]);
    let id = start(&home, work.path(), "crash:mounted", "sleep 30");
    let run = home.path().join("run").join(&id);
    let pid: i32 = fs::read_to_string(run.join("pid"))
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    await_path(&run.join("volume-owner.json"));
    kill_supervisor_and_child(pid);
    ok(&home, &["volume", "rm", "crash"]);
}

#[test]
fn mismatched_pid_token_refuses_recovery() {
    let home = TempDir::new().unwrap();
    let work = TempDir::new().unwrap();
    ok(&home, &["volume", "create", "token"]);
    let id = start(&home, work.path(), "token:mounted", "sleep 2");
    let witness = home.path().join("run").join(&id).join("volume-owner.json");
    await_path(&witness);
    let bad = fs::read_to_string(&witness)
        .unwrap()
        .replacen("linux:", "linux:mismatch:", 1);
    fs::write(&witness, bad).unwrap();
    let pid: i32 = fs::read_to_string(home.path().join("run").join(&id).join("pid"))
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    kill_supervisor_and_child(pid);
    let refused = lightr(&home, &["volume", "rm", "token"]);
    assert!(
        !refused.status.success(),
        "rm accepted mismatched pid token"
    );
    assert!(home.path().join("store/volumes/token").exists());
}

#[test]
fn concurrent_rm_and_prune_refuse_live_owner_under_flock() {
    let home = TempDir::new().unwrap();
    let work = TempDir::new().unwrap();
    ok(&home, &["volume", "create", "locked"]);
    let _id = start(&home, work.path(), "locked:mounted", "sleep 2");
    await_path(&home.path().join("store/volumes/locked/.lightr/owners.json"));
    let exe = env!("CARGO_BIN_EXE_lightr");
    let left = Command::new(exe)
        .args(["volume", "rm", "locked"])
        .env("LIGHTR_HOME", home.path())
        .spawn()
        .unwrap();
    let right = Command::new(exe)
        .args(["volume", "prune", "--force"])
        .env("LIGHTR_HOME", home.path())
        .spawn()
        .unwrap();
    assert!(
        !left.wait_with_output().unwrap().status.success(),
        "rm accepted live owner"
    );
    assert!(
        right.wait_with_output().unwrap().status.success(),
        "prune must skip live owner"
    );
    assert!(
        home.path().join("store/volumes/locked").exists(),
        "concurrent delete removed live owner"
    );
}
