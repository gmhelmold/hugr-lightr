use super::super::{Cancellation, Wait};
use super::{SourcePath, TopologyInspection};
use std::{
    fs, io,
    time::{Duration, Instant},
};
use tempfile::TempDir;

#[test]
fn topology_precancel_precedes_platform_and_path_access() {
    let root = TempDir::new().unwrap();
    let cancellation = Cancellation::default();
    cancellation.cancel();
    let result = TopologyInspection::inspect(
        &[],
        SourcePath::Directory(&root.path().join("absent")),
        None,
        Wait::Until {
            deadline: Instant::now() + Duration::from_secs(5),
            cancellation: &cancellation,
        },
    );
    assert_eq!(result.err().unwrap().kind(), io::ErrorKind::Interrupted);
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
}

#[test]
fn topology_expired_deadline_does_not_create_a_destination() {
    let root = TempDir::new().unwrap();
    let cancellation = Cancellation::default();
    let result = TopologyInspection::inspect(
        &[root.path()],
        SourcePath::Directory(root.path()),
        Some(&root.path().join("absent")),
        Wait::Until {
            deadline: Instant::now() - Duration::from_secs(1),
            cancellation: &cancellation,
        },
    );
    assert_eq!(result.err().unwrap().kind(), io::ErrorKind::TimedOut);
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
}

#[test]
fn topology_platform_capability_is_explicit() {
    let root = TempDir::new().unwrap();
    let store = root.path().join("store");
    let source = root.path().join("source");
    fs::create_dir(&store).unwrap();
    fs::create_dir(&source).unwrap();
    let result =
        TopologyInspection::inspect(&[&store], SourcePath::Directory(&source), None, Wait::Try);
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        let inspection = result.unwrap();
        assert!(inspection.source_handle().metadata().unwrap().is_dir());
        inspection.revalidate(Wait::Try).unwrap();
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    assert_eq!(result.err().unwrap().kind(), io::ErrorKind::Unsupported);
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 2);
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[path = "topology_unix_tests.rs"]
mod unix;
