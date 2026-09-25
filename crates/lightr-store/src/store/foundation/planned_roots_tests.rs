//! Explicit planned-root contracts. Fixtures own all paths; no public routing.
use super::topology::{DestinationInspection, ProtectedRoot, SourcePath, TopologyInspection};
use super::{Cancellation, Wait};
use std::{
    fs, io,
    time::{Duration, Instant},
};
use tempfile::TempDir;

fn rejected<T>(result: io::Result<T>, expected: io::ErrorKind, message: &str) {
    let error = result.err().unwrap_or_else(|| panic!("{message}"));
    assert_eq!(error.kind(), expected, "{message}: {error}");
}

#[test]
fn planned_roots_precancel_precedes_path_access() {
    let root = TempDir::new().unwrap();
    let planned = root.path().join("store");
    let target = root.path().join("output");
    let cancellation = Cancellation::default();
    cancellation.cancel();
    let wait = Wait::Until {
        deadline: Instant::now() + Duration::from_secs(5),
        cancellation: &cancellation,
    };
    let roots = [ProtectedRoot::PlannedDirectory(&planned)];
    rejected(
        TopologyInspection::inspect_configured(&roots, SourcePath::Directory(&target), None, wait),
        io::ErrorKind::Interrupted,
        "cancelled topology was accepted",
    );
    rejected(
        DestinationInspection::inspect_configured(&roots, &target, wait),
        io::ErrorKind::Interrupted,
        "cancelled destination was accepted",
    );
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
}

#[test]
fn planned_roots_deadline_precedes_path_access() {
    let root = TempDir::new().unwrap();
    let cancellation = Cancellation::default();
    let wait = Wait::Until {
        deadline: Instant::now() - Duration::from_secs(1),
        cancellation: &cancellation,
    };
    rejected(
        TopologyInspection::inspect_configured(&[], SourcePath::Directory(root.path()), None, wait),
        io::ErrorKind::TimedOut,
        "expired topology was accepted",
    );
    rejected(
        DestinationInspection::inspect_configured(&[], root.path(), wait),
        io::ErrorKind::TimedOut,
        "expired destination was accepted",
    );
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
}

#[test]
fn planned_roots_platform_support_is_explicit() {
    let root = TempDir::new().unwrap();
    let managed = root.path().join("managed");
    let public = root.path().join("public");
    fs::create_dir(&managed).unwrap();
    fs::create_dir(&public).unwrap();
    let planned = managed.join("new-store");
    let roots = [ProtectedRoot::PlannedDirectory(&planned)];
    let source = TopologyInspection::inspect_configured(
        &roots,
        SourcePath::Directory(&public),
        None,
        Wait::Try,
    );
    let destination = DestinationInspection::inspect_configured(&roots, &public, Wait::Try);
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        source.unwrap().revalidate(Wait::Try).unwrap();
        let destination = destination.unwrap();
        assert!(!destination.is_missing());
        destination.revalidate(Wait::Try).unwrap();
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        rejected(
            source,
            io::ErrorKind::Unsupported,
            "invented native topology support",
        );
        rejected(
            destination,
            io::ErrorKind::Unsupported,
            "invented native destination support",
        );
    }
    assert!(!planned.exists());
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 2);
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[path = "planned_roots_unix_tests.rs"]
mod unix;
