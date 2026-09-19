//! macOS-only native regressions. Children and scratch are owned by each test.
use super::configure_child;
use crate::stream_io::open_pty;
use std::fs::{self, File};
use std::io::{self, Read};
use std::os::fd::AsRawFd;
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::PathBuf;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const CHILD_ENV: &str = "LIGHTR_OWNED_PTY_CLOSE_WITNESS";
const CHILD_TEST: &str = "stream::tty_setup::tests::child_closes_standard_streams";
const PAYLOAD: &[u8] = b"pty-output-after-stdio-close\n";
const WATCHDOG: Duration = Duration::from_secs(10);

struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "lightr-pty-reference-{}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn marker(&self) -> PathBuf {
        self.0.join("stdio-closed")
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_file(self.marker());
        let _ = fs::remove_dir(&self.0);
    }
}
struct OwnedChild(Child);
impl OwnedChild {
    fn finish(&mut self) -> ExitStatus {
        let end = Instant::now() + WATCHDOG;
        loop {
            if let Some(status) = self.0.try_wait().unwrap() {
                return status;
            }
            assert!(Instant::now() < end, "owned child did not exit");
            std::thread::sleep(Duration::from_millis(1));
        }
    }
}
impl Drop for OwnedChild {
    fn drop(&mut self) {
        if matches!(self.0.try_wait(), Ok(None)) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}

fn wire(mut command: Command) -> (OwnedChild, File) {
    let (master, slave) = open_pty().unwrap();
    for file in [&master, &slave] {
        // SAFETY: live owned descriptors; fixture copies must not survive exec.
        let result = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_SETFD, libc::FD_CLOEXEC) };
        assert_eq!(result, 0);
    }
    command.stdin(slave.try_clone().unwrap());
    command.stdout(slave.try_clone().unwrap());
    command.stderr(slave.try_clone().unwrap());
    // SAFETY: same production child setup, after stdio is the fresh PTY slave.
    unsafe { command.pre_exec(|| configure_child()) };
    let child = OwnedChild(command.spawn().expect("owned PTY child setup failed"));
    drop(command);
    drop(slave);
    (child, master)
}

fn probe(scratch: &Scratch) -> (OwnedChild, File) {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command.args(["--exact", CHILD_TEST, "--nocapture"]);
    command.env(CHILD_ENV, scratch.marker());
    let (child, master) = wire(command);
    let end = Instant::now() + WATCHDOG;
    loop {
        match fs::read(scratch.marker()) {
            Ok(bytes) if bytes == b"closed" => break,
            Ok(_) => (),
            Err(error) if error.kind() == io::ErrorKind::NotFound => (),
            Err(error) => panic!("marker read: {error}"),
        }
        assert!(Instant::now() < end, "stdio-close marker watchdog");
        std::thread::sleep(Duration::from_millis(1));
    }
    (child, master)
}

fn drain(mut master: &File) -> io::Result<Vec<u8>> {
    let end = Instant::now() + WATCHDOG;
    let mut output = Vec::new();
    loop {
        let left = end.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err(io::ErrorKind::TimedOut.into());
        }
        let mut poll = libc::pollfd {
            fd: master.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        // SAFETY: one initialized pollfd for a live owned descriptor.
        let ready = unsafe { libc::poll(&mut poll, 1, left.as_millis().min(1000) as i32) };
        if ready < 0 {
            let error = io::Error::last_os_error();
            if error.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            return Err(error);
        }
        if ready == 0 {
            continue;
        }
        let mut buffer = [0u8; 1024];
        let count = match master.read(&mut buffer) {
            Ok(count) => count,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        };
        if count == 0 {
            return Ok(output);
        }
        if output.len() + count > 8192 {
            return Err(io::Error::other("PTY fixture exceeded output bound"));
        }
        output.extend_from_slice(&buffer[..count]);
    }
}

#[test]
fn child_closes_standard_streams() {
    let Some(marker) = std::env::var_os(CHILD_ENV) else {
        return; // Child entry point, not an additional runtime witness.
    };
    // SAFETY: executed only in our re-executed test child. No parent fd changes.
    unsafe {
        assert_eq!(
            libc::write(1, PAYLOAD.as_ptr().cast(), PAYLOAD.len()),
            PAYLOAD.len() as isize
        );
        assert_eq!(libc::close(0), 0);
        assert_eq!(libc::close(1), 0);
        assert_eq!(libc::close(2), 0);
    }
    fs::write(marker, b"closed").unwrap();
    std::process::exit(7);
}

#[test]
fn delayed_reader_keeps_output_after_all_child_stdio_close() {
    let scratch = Scratch::new();
    let (mut child, master) = probe(&scratch);
    let output = drain(&master).unwrap();
    assert!(
        output.ends_with(b"pty-output-after-stdio-close\r\n"),
        "queued PTY output was lost: {output:?}"
    );
    assert_eq!(child.finish().code(), Some(7));
}

#[test]
fn closing_the_master_releases_the_exiting_child() {
    let scratch = Scratch::new();
    let (mut child, master) = probe(&scratch);
    drop(master);
    let status = child.finish();
    // Disconnect may deliver SIGHUP before the child calls exit(7).
    assert!(status.code() == Some(7) || status.signal() == Some(libc::SIGHUP));
}

#[test]
fn empty_output_reaps_without_a_reader() {
    let mut command = Command::new("/bin/sh");
    command.args(["-c", "exit 9"]);
    let (mut child, _master) = wire(command);
    assert_eq!(child.finish().code(), Some(9));
}

#[test]
fn setup_error_does_not_spawn_a_non_tty_workload() {
    let mut command = Command::new("/bin/sh");
    command.args(["-c", "exit 0"]);
    command.stdin(Stdio::null());
    // SAFETY: only the child changes session; non-PTY stdin must be rejected.
    unsafe { command.pre_exec(|| configure_child()) };
    let error = match command.spawn() {
        Ok(child) => {
            let mut child = OwnedChild(child);
            let _ = child.finish();
            panic!("non-terminal setup was accepted")
        }
        Err(error) => error,
    };
    assert_eq!(error.raw_os_error(), Some(libc::ENOTTY));
}

#[test]
fn actual_exec_keeps_raw_master_merged_output_and_exit_code() {
    let mut command = Command::new("/bin/sh");
    command.args(["-c", "printf 'out\\n'; printf 'err\\n' >&2; exit 11"]);
    let mut session = super::super::open_exec_tty(command).unwrap();
    assert!(session.stdin.is_none());
    assert!(session.stderr.is_none());
    let master = session.pty_master.as_ref().unwrap();
    // SAFETY: isatty only inspects our live master descriptor.
    assert_eq!(unsafe { libc::isatty(master.as_raw_fd()) }, 1);
    let output = drain(session.stdout.as_ref().unwrap());
    drop(session.stdout.take());
    drop(session.pty_master.take());
    assert_eq!(session.waiter.wait().unwrap(), 11);
    assert_eq!(output.unwrap(), b"out\r\nerr\r\n");
}
