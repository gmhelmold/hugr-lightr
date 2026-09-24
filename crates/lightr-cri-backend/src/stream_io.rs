//! Streaming-plane I/O machinery (WP-CRI-STREAM, unix-only) — the io-table, the
//! output fan-out, the unix fd primitives, the exit waiters, and the
//! start_container hook that registers a container's live stdio.
//!
//! PROVENANCE: TRANSCRIBED from the conformance reference
//! `lightr-cri/crates/lightr-cri-fake/src/lib.rs` (the FanOut tee ~580, the
//! ContainerIo/io_table ~85, ChildWaiter ~655, the pipe-mode AttachWaiter
//! ~1862). Split out of `stream.rs` to keep each file <400 LOC (godfile law);
//! `stream.rs` keeps the `open_exec`/`open_attach` entry points.
//!
//! This whole module is `#[cfg(unix)]` (it is only compiled into a unix build):
//! pty, OS pipes and signals are unix concepts, and the streaming plane fails
//! closed on non-unix from `stream.rs`. So nothing here needs per-item cfg.

use std::sync::{Arc, Condvar, Mutex};

use crate::vocab::{BackendError, ContainerId, ContainerState, ExitWaiter, Result};
use crate::LightrBackend;

// ── io-table: live stdio held by start_container, keyed by container id ───────
//
// Mirrors the fake's `ContainerIo` / `io_table`. NOT persisted — these fds are
// valid only in the current process (attach is unavailable after a restart;
// `open_attach` surfaces that honestly). The reaper removes the entry on exit.

/// Held stdio for a running container. tty mode keeps the pty master (duped per
/// attach). Pipe mode keeps a `FanOut` plus a record of which streams exist (so
/// attach only wires the streams the container actually has) and the stdin
/// write-end (when `config.stdin`), duped per attach so the attacher feeds the
/// live process.
pub(crate) struct ContainerIo {
    /// tty mode: the pty master fd (cloned for each attach).
    pub pty_master: Option<std::fs::File>,
    /// Pipe mode: whether the container has a stdout stream to fan out.
    pub has_stdout: bool,
    /// Pipe mode: whether the container has a stderr stream to fan out.
    pub has_stderr: bool,
    /// Pipe mode: write end of the process stdin pipe (when config.stdin).
    pub stdin_wr: Option<std::fs::File>,
    /// Pipe mode: the output fan-out shared with the tee threads. `None` in tty
    /// mode (the kernel multiplexes the pty; attach dups the master instead).
    pub fanout: Option<Arc<FanOut>>,
}

/// Output fan-out shared between the per-stream tee threads (the SOLE readers of
/// the container's output) and `open_attach` (which registers sinks). Holds the
/// write-ends of one OS pipe per live attacher, split by stream so the CRI
/// streaming server can deliver stdout and stderr separately. Sinks whose pipe
/// is broken (attacher detached) are pruned on the next write — bounded by the
/// live-attacher count, no leak. Transcribed from the fake.
#[derive(Default)]
pub(crate) struct FanOut {
    stdout_sinks: Mutex<Vec<std::fs::File>>,
    stderr_sinks: Mutex<Vec<std::fs::File>>,
}

impl FanOut {
    /// Write `data` to every live sink for `stream` ("stdout"/"stderr"), pruning
    /// any sink whose pipe is broken. Single-reader fan-out: only the tee thread
    /// calls this.
    fn broadcast(&self, stream: &str, data: &[u8]) {
        use std::io::Write;
        let sinks = if stream == "stderr" {
            &self.stderr_sinks
        } else {
            &self.stdout_sinks
        };
        let mut guard = sinks.lock().unwrap();
        guard.retain_mut(|w| w.write_all(data).and_then(|()| w.flush()).is_ok());
    }

    /// Register a fresh attacher sink for `stream` and return its read-end.
    pub(crate) fn register(&self, stream: &str) -> std::io::Result<std::fs::File> {
        let (rd, wr) = make_pipe()?;
        let sinks = if stream == "stderr" {
            &self.stderr_sinks
        } else {
            &self.stdout_sinks
        };
        sinks.lock().unwrap().push(wr);
        Ok(rd)
    }
}

// ── unix primitives (no `nix`): pipe, dup, openpty ───────────────────────────

/// Create an internal OS pipe with neither endpoint inherited across exec.
pub(crate) fn make_pipe() -> std::io::Result<(std::fs::File, std::fs::File)> {
    use std::os::unix::io::{AsRawFd, FromRawFd};
    let mut fds = [0i32; 2];
    let rc = unsafe { libc::pipe(fds.as_mut_ptr()) };
    if rc != 0 {
        return Err(std::io::Error::last_os_error());
    }
    let r = unsafe { std::fs::File::from_raw_fd(fds[0]) };
    let w = unsafe { std::fs::File::from_raw_fd(fds[1]) };
    for file in [&r, &w] {
        let flags = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFD) };
        if flags < 0
            || unsafe { libc::fcntl(file.as_raw_fd(), libc::F_SETFD, flags | libc::FD_CLOEXEC) } < 0
        {
            return Err(std::io::Error::last_os_error());
        }
    }
    Ok((r, w))
}

#[cfg(target_os = "macos")]
pub(crate) fn make_socketpair() -> std::io::Result<(std::fs::File, std::fs::File)> {
    use std::os::unix::io::{AsRawFd, FromRawFd};
    let mut fds = [-1i32; 2];
    if unsafe { libc::socketpair(libc::AF_UNIX, libc::SOCK_STREAM, 0, fds.as_mut_ptr()) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    let left = unsafe { std::fs::File::from_raw_fd(fds[0]) };
    let right = unsafe { std::fs::File::from_raw_fd(fds[1]) };
    for file in [&left, &right] {
        let flags = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFD) };
        if flags < 0
            || unsafe { libc::fcntl(file.as_raw_fd(), libc::F_SETFD, flags | libc::FD_CLOEXEC) } < 0
        {
            return Err(std::io::Error::last_os_error());
        }
    }
    Ok((left, right))
}

/// Duplicate a file descriptor into a fresh owned `File` (transcribed from the
/// fake's `dup_file`).
pub(crate) fn dup_file(f: &std::fs::File) -> std::io::Result<std::fs::File> {
    // The standard clone atomically sets close-on-exec. Command still performs
    // intentional stdio transfers; unrelated children must not retain these fds.
    f.try_clone()
}

/// Open a pty pair, returning (master, slave) as owned `File`s. Uses
/// `libc::openpty` (avoids a `nix` dependency; present on macOS + Linux).
#[cfg(not(target_os = "macos"))]
pub(crate) fn open_pty() -> std::io::Result<(std::fs::File, std::fs::File)> {
    use std::os::unix::io::{AsRawFd, FromRawFd};
    let mut master: libc::c_int = -1;
    let mut slave: libc::c_int = -1;
    let rc = unsafe {
        libc::openpty(
            &mut master,
            &mut slave,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    if rc != 0 {
        return Err(std::io::Error::last_os_error());
    }
    let master_file = unsafe { std::fs::File::from_raw_fd(master) };
    let slave_file = unsafe { std::fs::File::from_raw_fd(slave) };
    for file in [&master_file, &slave_file] {
        let flags = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFD) };
        if flags < 0
            || unsafe { libc::fcntl(file.as_raw_fd(), libc::F_SETFD, flags | libc::FD_CLOEXEC) } < 0
        {
            return Err(std::io::Error::last_os_error());
        }
    }
    Ok((master_file, slave_file))
}

// ── tee with fan-out (pipe mode) ─────────────────────────────────────────────

/// Spawn the SINGLE-reader tee for one container stream: read raw chunks, (a)
/// broadcast each chunk to live attachers IMMEDIATELY (before line framing, so
/// interactive attach has bare read→broadcast latency — no `\n` buffer stall),
/// then (b) write one CRI-formatted `F`/`P` record per line to the log. There
/// is no second reader of the container fd, so attach never races the log.
/// Transcribed from the fake's `spawn_tee_thread`.
fn spawn_tee_fanout(
    stream: &'static str,
    reader: std::fs::File,
    log: Arc<Mutex<Option<std::fs::File>>>,
    fanout: Arc<FanOut>,
) {
    use crate::util::cri_log_line;
    std::thread::spawn(move || {
        use std::io::{Read, Write};
        let mut reader = reader;
        let mut buf = [0u8; 8192];
        let mut pending: Vec<u8> = Vec::new();
        loop {
            let n = match reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => n,
            };
            let chunk = &buf[..n];
            // (a) live attachers first — raw bytes, no newline gate.
            fanout.broadcast(stream, chunk);
            // (b) CRI log — one formatted record per complete line.
            pending.extend_from_slice(chunk);
            while let Some(pos) = pending.iter().position(|&b| b == b'\n') {
                let line: Vec<u8> = pending.drain(..=pos).collect();
                let formatted = cri_log_line(stream, &line);
                if let Some(f) = log.lock().unwrap().as_mut() {
                    let _ = f.write_all(&formatted);
                }
            }
        }
        if !pending.is_empty() {
            let formatted = cri_log_line(stream, &pending);
            if let Some(f) = log.lock().unwrap().as_mut() {
                let _ = f.write_all(&formatted);
            }
        }
    });
}

// ── ExitWaiters ──────────────────────────────────────────────────────────────

/// Non-consuming observation of a direct child exit. The sole child reaper
/// publishes this state before `ExitWaiter::wait` consumes its owner.
type ChildExitResult = std::result::Result<i32, String>;
type ChildExitState = (Mutex<Option<ChildExitResult>>, Condvar);

#[derive(Clone)]
pub(crate) struct ChildExit {
    state: Arc<ChildExitState>,
}

impl ChildExit {
    pub(crate) fn wait(&self) -> Result<i32> {
        let (result, ready) = &*self.state;
        let mut result = result.lock().unwrap();
        while result.is_none() {
            result = ready.wait(result).unwrap();
        }
        match result.as_ref().unwrap() {
            Ok(code) => Ok(*code),
            Err(error) => Err(BackendError::Internal(error.clone())),
        }
    }
}

/// Waiter for an exec child: its owned thread reaps once and maps status to
/// 128+sig (signal kill) or exit code. `wait` joins that thread, then drops any
/// retained PTY slave. Transcribed from fake's `ChildWaiter`.
pub(crate) struct ChildWaiter {
    exit: ChildExit,
    reaper: std::thread::JoinHandle<()>,
    /// Keeps one slave descriptor open while caller drains a fast tty child.
    /// Darwin flushes pending master output when its final slave closes.
    pub pty_slave: Option<std::fs::File>,
}

impl ChildWaiter {
    pub(crate) fn new(
        child: std::process::Child,
        pty_slave: Option<std::fs::File>,
    ) -> Result<Self> {
        Self::start(child, pty_slave, std::thread::Builder::new(), None)
    }

    #[cfg(test)]
    pub(crate) fn new_with_setup_failure(
        child: std::process::Child,
        _pty_slave: Option<std::fs::File>,
    ) -> Result<Self> {
        Self::reap_after_setup_failure(child, "injected setup failure")
    }

    fn reap_after_setup_failure(
        mut child: std::process::Child,
        error: impl std::fmt::Display,
    ) -> Result<Self> {
        let _ = child.kill();
        let _ = child.wait();
        Err(BackendError::Internal(format!(
            "child exit watcher setup: {error}"
        )))
    }

    fn start(
        child: std::process::Child,
        pty_slave: Option<std::fs::File>,
        builder: std::thread::Builder,
        exit_notify: Option<std::fs::File>,
    ) -> Result<Self> {
        let state = Arc::new((Mutex::new(None), Condvar::new()));
        let exit = ChildExit {
            state: Arc::clone(&state),
        };
        // Keep child recoverable if OS rejects thread setup; failure reaps it
        // synchronously rather than orphaning a zombie.
        let child = Arc::new(Mutex::new(Some(child)));
        let reaper_child = Arc::clone(&child);
        let reaper = builder.spawn(move || {
            let mut child = reaper_child.lock().unwrap().take().unwrap();
            let result = child
                .wait()
                .map(|status| crate::util::exit_code_from_status(&status))
                .map_err(|error| format!("wait: {error}"));
            let (result_slot, ready) = &*state;
            *result_slot.lock().unwrap() = Some(result);
            ready.notify_all();
            if let Some(mut notify) = exit_notify {
                use std::io::Write;
                let _ = notify.write_all(&[1]);
            }
        });
        let reaper = match reaper {
            Ok(reaper) => reaper,
            Err(error) => {
                let child = child.lock().unwrap().take().unwrap();
                return Self::reap_after_setup_failure(child, error);
            }
        };
        Ok(Self {
            exit,
            reaper,
            pty_slave,
        })
    }

    #[cfg(test)]
    pub(crate) fn exit_state(&self) -> ChildExit {
        self.exit.clone()
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn new_with_exit_notify(
        child: std::process::Child,
        pty_slave: Option<std::fs::File>,
        exit_notify: std::fs::File,
    ) -> Result<Self> {
        Self::start(
            child,
            pty_slave,
            std::thread::Builder::new(),
            Some(exit_notify),
        )
    }
}

impl ExitWaiter for ChildWaiter {
    fn wait(mut self: Box<Self>) -> Result<i32> {
        let code = self.exit.wait()?;
        self.reaper
            .join()
            .map_err(|_| BackendError::Internal("child exit watcher panicked".into()))?;
        drop(self.pty_slave.take());
        Ok(code)
    }
}

/// Waiter for an attach session: the session does not own the container child,
/// so it polls the cache and completes when the container leaves Running,
/// yielding its recorded exit code. Transcribed from the fake's pipe-mode
/// `AttachWaiter` (the tty no-op waiter is folded in: a tty container's exit is
/// still observed via the same cache poll, so one waiter covers both modes).
pub(crate) struct AttachWaiter {
    pub cache: Arc<Mutex<crate::container::Cache>>,
    pub id: ContainerId,
}

impl ExitWaiter for AttachWaiter {
    fn wait(self: Box<Self>) -> Result<i32> {
        loop {
            let code = {
                let cache = self.cache.lock().unwrap();
                match cache.containers.get(&self.id.0) {
                    Some(r) if r.state == ContainerState::Running => None,
                    Some(r) => Some(r.exit_code),
                    None => Some(0), // removed underneath us
                }
            };
            if let Some(c) = code {
                return Ok(c);
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }
}

// ── start_container hook: register the live stdio ────────────────────────────

impl LightrBackend {
    /// Wire the live stdio for `open_attach`: build a `FanOut`, run the SINGLE
    /// reader of each stream through the fan-out tee (broadcast to attachers +
    /// write the CRI log), hold the stdin write-end, and register the io-table
    /// entry. Transcribed from the fake's pipe-mode `start_container`.
    pub(crate) fn register_io_and_tee(
        &self,
        id: &ContainerId,
        child: &mut std::process::Child,
        log_shared: &Arc<Mutex<Option<std::fs::File>>>,
    ) {
        use std::os::unix::io::{FromRawFd, IntoRawFd};

        let fanout = Arc::new(FanOut::default());
        let has_stdout = child.stdout.is_some();
        let has_stderr = child.stderr.is_some();

        if let Some(out) = child.stdout.take() {
            let f = unsafe { std::fs::File::from_raw_fd(out.into_raw_fd()) };
            spawn_tee_fanout("stdout", f, Arc::clone(log_shared), Arc::clone(&fanout));
        }
        if let Some(err) = child.stderr.take() {
            let f = unsafe { std::fs::File::from_raw_fd(err.into_raw_fd()) };
            spawn_tee_fanout("stderr", f, Arc::clone(log_shared), Arc::clone(&fanout));
        }
        let stdin_wr = child
            .stdin
            .take()
            .map(|s| unsafe { std::fs::File::from_raw_fd(s.into_raw_fd()) });

        let io = ContainerIo {
            pty_master: None,
            has_stdout,
            has_stderr,
            stdin_wr,
            fanout: Some(fanout),
        };
        self.io_table.lock().unwrap().insert(id.0.clone(), io);
    }
}

#[cfg(target_os = "macos")]
#[path = "stream_pty.rs"]
mod darwin_pty;

#[cfg(target_os = "macos")]
pub(crate) fn open_pty() -> std::io::Result<(std::fs::File, std::fs::File)> {
    darwin_pty::open()
}

#[cfg(all(test, target_os = "macos"))]
#[path = "stream_descriptor_tests.rs"]
mod descriptor_tests;
