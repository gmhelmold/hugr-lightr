//! Tests for the streaming plane (WP-CRI-STREAM) — open_exec / open_attach.
//!
//! Parallel-safe: each test owns a unique tempdir home (atomic counter + nanos,
//! no process-global mutation) and spawns real, short-lived host processes.
//! unix-only (the plane is unix-only; the windows gate compiles but never runs).

use std::io::Read;
use std::path::PathBuf;

use crate::vocab::{
    BackendError, ContainerConfig, ContainerId, ContainerState, ContainerStatus, ExitWaiter,
};
use crate::{CriBackend, LightrBackend};

#[cfg(unix)]
use crate::stream_io::ChildWaiter;

fn temp_home() -> PathBuf {
    static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let p = std::env::temp_dir().join(format!("lightr-cri-stream-{nanos}-{n}"));
    std::fs::create_dir_all(&p).unwrap();
    p
}

fn cfg(command: Vec<&str>) -> ContainerConfig {
    ContainerConfig {
        name: "c".into(),
        attempt: 0,
        image_ref: "img".into(),
        command: command.into_iter().map(String::from).collect(),
        args: Vec::new(),
        working_dir: String::new(),
        envs: Vec::new(),
        mounts: Vec::new(),
        labels: Default::default(),
        annotations: Default::default(),
        log_path: String::new(),
        tty: false,
        stdin: false,
        security: None,
    }
}

/// Create + start a container with `command` (empty ⇒ keep-alive). Returns a
/// Running container id (or the keep-alive one). Polls until Running.
/// Run a Ready **host_network** sandbox named `name` so the create-gate
/// (WP-CRI-SANDBOX) admits containers, and — crucially — the container takes the
/// HOST-process path, not the ns-engine path. These tests exercise the exec/stream
/// IO plane over real short-lived host processes; they do NOT test isolation. A
/// host_network sandbox has no pinned netns (sandbox.rs), so post-#99
/// `start_container` legitimately runs a host process here instead of fail-closing
/// on the non-hydratable `img` image (which it correctly would for a netns'd pod).
fn ready_sandbox(b: &LightrBackend, name: &str) -> crate::vocab::SandboxId {
    b.run_sandbox(crate::vocab::SandboxConfig {
        name: name.into(),
        uid: "u".into(),
        namespace: "ns".into(),
        attempt: 0,
        labels: Default::default(),
        annotations: Default::default(),
        log_directory: String::new(),
        hostname: String::new(),
        host_network: true,
        dns: None,
        port_mappings: Vec::new(),
    })
    .expect("run_sandbox")
}

fn running_container(b: &LightrBackend, command: Vec<&str>) -> ContainerId {
    let sb = ready_sandbox(b, "sb-test");
    let id = b.create_container(&sb, cfg(command)).unwrap();
    b.start_container(&id).unwrap();
    id
}

fn wait_state(b: &LightrBackend, id: &ContainerId, want: ContainerState) {
    for _ in 0..200 {
        if let Ok(ContainerStatus { state, .. }) = b.container_status(id) {
            if state == want {
                return;
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    panic!("container {} did not reach {want:?}", id.0);
}

// ── open_exec: non-tty stdout + waiter exit code ─────────────────────────────

#[test]
fn open_exec_pipe_yields_stdout_and_zero_exit() {
    let b = LightrBackend::new(temp_home());
    let id = running_container(&b, vec![]); // keep-alive container

    let mut s = b
        .open_exec(&id, &["echo".into(), "hello-exec".into()], false, false)
        .unwrap();
    assert!(s.pty_master.is_none());
    let mut out = String::new();
    s.stdout.take().unwrap().read_to_string(&mut out).unwrap();
    assert_eq!(out, "hello-exec\n");
    // stderr stream exists (piped) but is empty.
    let mut err = String::new();
    s.stderr.take().unwrap().read_to_string(&mut err).unwrap();
    assert!(err.is_empty(), "stderr: {err:?}");
    // The real waiter reaps the child and yields the exit code.
    assert_eq!(s.waiter.wait().unwrap(), 0);
}

#[test]
fn open_exec_waiter_yields_nonzero_exit() {
    let b = LightrBackend::new(temp_home());
    let id = running_container(&b, vec![]);

    let s = b
        .open_exec(
            &id,
            &["sh".into(), "-c".into(), "exit 7".into()],
            false,
            false,
        )
        .unwrap();
    assert_eq!(s.waiter.wait().unwrap(), 7);
}

#[test]
fn open_exec_stdin_pipe_present_when_requested() {
    let b = LightrBackend::new(temp_home());
    let id = running_container(&b, vec![]);

    // `cat` echoes stdin to stdout; feed it, close stdin, read it back.
    let mut s = b.open_exec(&id, &["cat".into()], false, true).unwrap();
    {
        use std::io::Write;
        let mut stdin = s.stdin.take().expect("stdin pipe requested");
        stdin.write_all(b"piped-in\n").unwrap();
        // drop stdin → EOF so cat exits
    }
    let mut out = String::new();
    s.stdout.take().unwrap().read_to_string(&mut out).unwrap();
    assert_eq!(out, "piped-in\n");
    assert_eq!(s.waiter.wait().unwrap(), 0);
}

// ── open_exec: tty ───────────────────────────────────────────────────────────

#[test]
fn open_exec_tty_uses_pty_master_no_stderr() {
    let b = LightrBackend::new(temp_home());
    let id = running_container(&b, vec![]);

    let mut s = b
        .open_exec(&id, &["echo".into(), "hello-tty".into()], true, false)
        .unwrap();
    assert!(s.pty_master.is_some(), "tty must hand back a pty master");
    assert!(s.stderr.is_none(), "tty merges stderr onto the pty stream");
    assert!(
        s.stdin.is_none(),
        "tty: write to the master, no separate stdin"
    );

    // Bound both bytes and elapsed time; preserve errors instead of converting
    // them to empty successful output. The real echo command remains unchanged.
    let master = s.stdout.take().unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    let reader = std::thread::spawn(move || {
        let _ = tx.send(read_pty_line(master));
    });
    let out = rx
        .recv_timeout(std::time::Duration::from_secs(5))
        .expect("pty read timed out")
        .expect("pty stream failed before a complete line");
    reader.join().unwrap();
    let text = String::from_utf8_lossy(&out);
    assert!(text.contains("hello-tty"), "pty output: {text:?}");

    let code = s.waiter.wait().unwrap();
    assert_eq!(code, 0);
}

#[test]
fn open_exec_tty_keeps_output_until_first_master_read() {
    let b = LightrBackend::new(temp_home());
    let id = running_container(&b, vec![]);
    let marker = temp_home().join("child-closed-stdio");
    let marker_c = {
        use std::os::unix::ffi::OsStrExt;
        std::ffi::CString::new(marker.as_os_str().as_bytes()).unwrap()
    };
    assert_eq!(unsafe { libc::mkfifo(marker_c.as_ptr(), 0o600) }, 0);
    let command = format!(
        "printf 'fast-tty-output\\n'; exec 0>&- 1>&- 2>&-; printf x > '{}'",
        marker.display()
    );

    let mut s = b
        .open_exec(&id, &["sh".into(), "-c".into(), command], true, false)
        .unwrap();

    // FIFO write runs only after direct child's slave descriptors close. Do not
    // read master before it arrives: this is final-slave-close lifetime edge.
    let mut signal = std::fs::File::open(&marker).unwrap();
    let mut ready = Vec::new();
    signal.read_to_end(&mut ready).unwrap();
    assert_eq!(ready, b"x");
    let out = read_pty_line(s.stdout.take().unwrap()).expect("queued tty output");
    assert!(
        String::from_utf8_lossy(&out).contains("fast-tty-output"),
        "pty output: {out:?}"
    );
    assert_eq!(s.waiter.wait().unwrap(), 0);
}

#[test]
fn child_exit_state_observes_mapped_exit_before_waiter_consumption() {
    let child = std::process::Command::new("sh")
        .args(["-c", "exit 7"])
        .spawn()
        .unwrap();
    let waiter = ChildWaiter::new(child, Some(std::fs::File::open("/dev/null").unwrap())).unwrap();
    let exit = waiter.exit_state();

    // This observes sole reaper's saved status; it neither consumes waiter nor
    // releases retained slave before caller explicitly consumes waiter.
    assert_eq!(exit.wait().unwrap(), 7);
    assert!(waiter.pty_slave.is_some());
    assert_eq!(Box::new(waiter).wait().unwrap(), 7);
}

#[test]
fn child_waiter_setup_failure_reaps_child_without_orphan_thread() {
    let child = std::process::Command::new("true").spawn().unwrap();
    let pid = child.id() as libc::pid_t;
    let result = ChildWaiter::new_with_setup_failure(child, None);
    let error = match result {
        Ok(_) => panic!("injected setup failure must reject watcher setup"),
        Err(error) => error,
    };
    assert!(matches!(error, BackendError::Internal(message) if message.contains("watcher setup")));
    assert_eq!(unsafe { libc::kill(pid, 0) }, -1);
    assert_eq!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(libc::ESRCH)
    );
}

// ── open_exec: precondition / not-found ──────────────────────────────────────

#[test]
fn open_exec_requires_running_and_existing() {
    let b = LightrBackend::new(temp_home());
    // missing container → NotFound
    assert!(matches!(
        b.open_exec(&ContainerId("nope".into()), &["true".into()], false, false),
        Err(BackendError::NotFound(_))
    ));
    // created-but-not-started container → FailedPrecondition
    let sb = ready_sandbox(&b, "sb");
    let id = b.create_container(&sb, cfg(vec!["true"])).unwrap();
    assert!(matches!(
        b.open_exec(&id, &["true".into()], false, false),
        Err(BackendError::FailedPrecondition(_))
    ));
    // empty command on a running container → InvalidArgument
    let rid = running_container(&b, vec![]);
    assert!(matches!(
        b.open_exec(&rid, &[], false, false),
        Err(BackendError::InvalidArgument(_))
    ));
}

// ── open_attach: pipe mode receives live output, waiter on exit ──────────────

#[test]
fn open_attach_pipe_receives_live_output() {
    let b = LightrBackend::new(temp_home());
    // A container that keeps emitting lines while Running, so an attach that
    // registers after start still catches subsequent ticks (no start race).
    let id = running_container(
        &b,
        vec![
            "sh",
            "-c",
            "i=0; while [ $i -lt 200 ]; do echo tick; i=$((i+1)); sleep 0.02; done",
        ],
    );
    wait_state(&b, &id, ContainerState::Running);

    let mut s = b.open_attach(&id).unwrap();
    assert!(s.pty_master.is_none());
    let mut stdout = s.stdout.take().expect("pipe-mode stdout sink");

    // Read at least one tick from the fan-out sink.
    let mut buf = [0u8; 64];
    let n = stdout.read(&mut buf).unwrap();
    assert!(n > 0, "attach received no bytes");
    let got = String::from_utf8_lossy(&buf[..n]);
    assert!(got.contains("tick"), "attach output: {got:?}");

    // Stop the container; the attach waiter completes on exit.
    b.stop_container(&id, 0).unwrap();
    wait_state(&b, &id, ContainerState::Exited);
    // Waiter returns the recorded exit code (SIGKILL ⇒ 137); just assert it
    // returns rather than blocking forever.
    let code = s.waiter.wait().unwrap();
    assert!(code != 0, "expected a non-zero (killed) exit, got {code}");
}

#[test]
fn open_attach_requires_running_and_existing() {
    let b = LightrBackend::new(temp_home());
    assert!(matches!(
        b.open_attach(&ContainerId("nope".into())),
        Err(BackendError::NotFound(_))
    ));
    let sb = ready_sandbox(&b, "sb");
    let id = b.create_container(&sb, cfg(vec!["true"])).unwrap();
    assert!(matches!(
        b.open_attach(&id),
        Err(BackendError::FailedPrecondition(_))
    ));
}

// ── parallel-safe: two backends + two exec sessions concurrently ─────────────

#[test]
fn parallel_exec_sessions_are_independent() {
    let b1 = LightrBackend::new(temp_home());
    let b2 = LightrBackend::new(temp_home());
    let id1 = running_container(&b1, vec![]);
    let id2 = running_container(&b2, vec![]);

    let h1 = {
        let s = b1
            .open_exec(
                &id1,
                &["sh".into(), "-c".into(), "exit 3".into()],
                false,
                false,
            )
            .unwrap();
        std::thread::spawn(move || s.waiter.wait().unwrap())
    };
    let h2 = {
        let s = b2
            .open_exec(
                &id2,
                &["sh".into(), "-c".into(), "exit 5".into()],
                false,
                false,
            )
            .unwrap();
        std::thread::spawn(move || s.waiter.wait().unwrap())
    };
    assert_eq!(h1.join().unwrap(), 3);
    assert_eq!(h2.join().unwrap(), 5);
}

fn read_pty_line(reader: impl Read) -> std::io::Result<Vec<u8>> {
    use std::io::BufRead;
    let mut line = Vec::new();
    // read_until handles short reads and Interrupted, but preserves other errors.
    std::io::BufReader::new(reader.take(128)).read_until(b'\n', &mut line)?;
    if line.last() != Some(&b'\n') {
        return Err(std::io::ErrorKind::UnexpectedEof.into());
    }
    Ok(line)
}

#[test]
fn pty_line_reader_handles_interruption_and_fragmentation() {
    struct Chunks {
        interrupted: bool,
        bytes: std::io::Cursor<Vec<u8>>,
    }
    impl Read for Chunks {
        fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
            if !self.interrupted {
                self.interrupted = true;
                return Err(std::io::ErrorKind::Interrupted.into());
            }
            let n = out.len().min(2);
            self.bytes.read(&mut out[..n])
        }
    }
    let reader = Chunks {
        interrupted: false,
        bytes: std::io::Cursor::new(b"hello-tty\r\n".to_vec()),
    };
    assert_eq!(read_pty_line(reader).unwrap(), b"hello-tty\r\n");
}

#[test]
fn pty_line_reader_preserves_errors_and_bounds_incomplete_output() {
    struct Broken;
    impl Read for Broken {
        fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
            Err(std::io::Error::from_raw_os_error(5))
        }
    }
    assert_eq!(read_pty_line(Broken).unwrap_err().raw_os_error(), Some(5));
    assert_eq!(
        read_pty_line(std::io::repeat(b'x')).unwrap_err().kind(),
        std::io::ErrorKind::UnexpectedEof
    );
}
