//! Native descriptor ownership regressions, using only this test's PTYs.
use super::{dup_file, open_pty};
use std::fs::File;
use std::os::fd::AsRawFd;
use std::os::unix::fs::MetadataExt;
use std::process::{Command, Stdio};

const CHILD: &str = "stream_io::descriptor_tests::descriptor_absence_child";
const INPUT: &str = "LIGHTR_OWNED_PTY_DESCRIPTOR_IDENTITIES";

fn assert_cloexec(file: &File) {
    // SAFETY: inspect one live borrowed descriptor, without closing it.
    let flags = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFD) };
    assert!(
        flags >= 0,
        "descriptor flags: {}",
        std::io::Error::last_os_error()
    );
    assert_ne!(flags & libc::FD_CLOEXEC, 0, "PTY descriptor survives exec");
}

fn check_exec(files: &[&File]) {
    let identities = files
        .iter()
        .map(|file| {
            let m = file.metadata().unwrap();
            format!("{}:{}:{}:{}", file.as_raw_fd(), m.dev(), m.ino(), m.rdev())
        })
        .collect::<Vec<_>>()
        .join(",");
    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", CHILD, "--nocapture"])
        .env(INPUT, identities)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "unrelated child retained a PTY descriptor:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn pty_descriptors_are_close_on_exec_at_return() {
    let (master, slave) = open_pty().unwrap();
    assert_cloexec(&master);
    assert_cloexec(&slave);
}

#[test]
fn pty_descriptor_duplicates_are_close_on_exec() {
    let (master, slave) = open_pty().unwrap();
    for source in [&master, &slave] {
        let duplicate = dup_file(source).unwrap();
        assert_cloexec(&duplicate);
        assert_eq!(
            source.metadata().unwrap().rdev(),
            duplicate.metadata().unwrap().rdev()
        );
        check_exec(&[&duplicate]);
    }
}

#[test]
fn pty_descriptors_are_absent_after_unrelated_exec() {
    let (master, slave) = open_pty().unwrap();
    check_exec(&[&master, &slave]);
}

#[test]
fn descriptor_absence_child() {
    let Ok(input) = std::env::var(INPUT) else {
        return; // Re-executed helper, not an additional behavioral witness.
    };
    for item in input.split(',') {
        let fields = item
            .split(':')
            .map(|s| s.parse::<u64>().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(fields.len(), 4);
        let fd: libc::c_int = fields[0].try_into().unwrap();
        let mut metadata = std::mem::MaybeUninit::<libc::stat>::uninit();
        // SAFETY: fstat initializes metadata on success; it does not acquire
        // ownership of this explicitly identified inherited test descriptor.
        if unsafe { libc::fstat(fd, metadata.as_mut_ptr()) } == 0 {
            let m = unsafe { metadata.assume_init() };
            let actual = (m.st_dev as u64, m.st_ino, m.st_rdev as u64);
            assert_ne!(
                actual,
                (fields[1], fields[2], fields[3]),
                "unrelated exec inherited the original PTY endpoint"
            );
        } else {
            assert_eq!(
                std::io::Error::last_os_error().raw_os_error(),
                Some(libc::EBADF)
            );
        }
    }
}
