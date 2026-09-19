use super::*;
use crate::store::foundation::{Cancellation, StoreLocks};
#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::fs;
use std::time::{Duration, Instant};
use tempfile::TempDir;

#[test]
fn scratch_precancel_precedes_allocation() {
    let root = TempDir::new().unwrap();
    let locks = StoreLocks::open_existing(root.path()).unwrap();
    let lease = locks.shared(Wait::Try).unwrap();
    let cancellation = Cancellation::default();
    cancellation.cancel();
    let wait = Wait::Until {
        deadline: Instant::now() + Duration::from_secs(5),
        cancellation: &cancellation,
    };
    assert_eq!(
        ScratchDirectory::reserve(&lease, wait)
            .err()
            .unwrap()
            .kind(),
        io::ErrorKind::Interrupted
    );
    assert!(!root.path().join(".si01-staging").exists());
}

#[test]
fn scratch_expired_deadline_precedes_allocation() {
    let root = TempDir::new().unwrap();
    let locks = StoreLocks::open_existing(root.path()).unwrap();
    let lease = locks.shared(Wait::Try).unwrap();
    let cancellation = Cancellation::default();
    let wait = Wait::Until {
        deadline: Instant::now() - Duration::from_secs(1),
        cancellation: &cancellation,
    };
    assert_eq!(
        ScratchDirectory::reserve(&lease, wait)
            .err()
            .unwrap()
            .kind(),
        io::ErrorKind::TimedOut
    );
    assert!(!root.path().join(".si01-staging").exists());
}

#[test]
fn scratch_platform_contract_is_explicit() {
    let root = TempDir::new().unwrap();
    let locks = StoreLocks::open_existing(root.path()).unwrap();
    let lease = locks.shared(Wait::Try).unwrap();
    let result = ScratchDirectory::reserve(&lease, Wait::Try);
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        result.unwrap().finish().unwrap();
        assert_eq!(
            fs::read_dir(root.path().join(".si01-staging"))
                .unwrap()
                .count(),
            0
        );
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        assert_eq!(result.err().unwrap().kind(), io::ErrorKind::Unsupported);
        assert!(!root.path().join(".si01-staging").exists());
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[path = "anchored_scratch_unix_tests.rs"]
mod unix;
