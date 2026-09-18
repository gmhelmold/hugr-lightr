//! Only the test-owned shell PID publication, never an external process source.
use std::path::Path;

pub(super) fn read_complete_pid(path: &Path) -> Option<i32> {
    let text = std::fs::read_to_string(path).ok()?;
    if !text.ends_with('\n') {
        return None;
    }
    text.trim().parse::<i32>().ok().filter(|pid| *pid > 1)
}

#[test]
fn partial_pid_publication_is_not_ready() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("child.pid");
    std::fs::write(&path, b"").unwrap();
    assert_eq!(read_complete_pid(&path), None);
    std::fs::write(&path, b"123").unwrap();
    assert_eq!(
        read_complete_pid(&path),
        None,
        "partial PID has no completed line"
    );
    std::fs::write(&path, b"123\n").unwrap();
    assert_eq!(read_complete_pid(&path), Some(123));
}

#[test]
fn absent_or_invalid_pid_publication_is_not_ready() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("child.pid");
    assert_eq!(read_complete_pid(&path), None);
    for bytes in [b"invalid\n".as_slice(), b"-1\n", b"0\n", b"1\n"] {
        std::fs::write(&path, bytes).unwrap();
        assert_eq!(read_complete_pid(&path), None);
    }
}
