//! Destination-only observations; no runtime or materialization activation.
use super::topology::DestinationInspection;
use super::{Cancellation, Wait};
use std::{
    fs, io,
    time::{Duration, Instant},
};
use tempfile::TempDir;

#[test]
fn destination_precancel_precedes_native_resolution() {
    let root = TempDir::new().unwrap();
    let cancellation = Cancellation::default();
    cancellation.cancel();
    let result = DestinationInspection::inspect(
        &[],
        &root.path().join("uncreated"),
        Wait::Until {
            deadline: Instant::now() + Duration::from_secs(5),
            cancellation: &cancellation,
        },
    );
    assert_eq!(result.err().unwrap().kind(), io::ErrorKind::Interrupted);
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
}

#[test]
fn destination_expired_deadline_precedes_native_resolution() {
    let root = TempDir::new().unwrap();
    let cancellation = Cancellation::default();
    let result = DestinationInspection::inspect(
        &[],
        &root.path().join("uncreated"),
        Wait::Until {
            deadline: Instant::now() - Duration::from_secs(1),
            cancellation: &cancellation,
        },
    );
    assert_eq!(result.err().unwrap().kind(), io::ErrorKind::TimedOut);
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
}

#[test]
fn destination_platform_disposition_is_explicit_without_live_source() {
    let root = TempDir::new().unwrap();
    let store = root.path().join("store");
    let output = root.path().join("output");
    fs::create_dir(&store).unwrap();
    let result = DestinationInspection::inspect(&[&store], &output, Wait::Try);
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        let observed = result.unwrap();
        assert!(observed.is_missing());
        assert!(observed.existing_directory().is_none());
        observed.revalidate(Wait::Try).unwrap();
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    assert_eq!(result.err().unwrap().kind(), io::ErrorKind::Unsupported);
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[path = "destination_unix_tests.rs"]
mod unix;
