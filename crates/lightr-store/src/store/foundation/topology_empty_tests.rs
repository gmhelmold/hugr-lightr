//! Owned fixtures; no user Store, process-global cwd or environment mutation.
use super::{scan_empty, DirectoryStream, Entry};
use crate::store::foundation::topology::DestinationInspection;
use crate::store::foundation::{Cancellation, Wait};
use std::fs;
use std::io;
use std::os::unix::fs::{symlink, MetadataExt};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tempfile::TempDir;

fn fixture() -> (TempDir, PathBuf, PathBuf) {
    let root = TempDir::new().unwrap();
    let store = root.path().join("store");
    let output = root.path().join("output");
    fs::create_dir(&store).unwrap();
    fs::create_dir(&output).unwrap();
    fs::write(store.join("sentinel"), b"retained").unwrap();
    (root, store, output)
}

#[test]
fn empty_destination_existing_and_missing_are_checked_without_creation() {
    let (_root, store, output) = fixture();
    let existing = DestinationInspection::inspect(&[&store], &output, Wait::Try).unwrap();
    existing.require_empty(Wait::Try).unwrap();
    existing.require_empty(Wait::Try).unwrap();
    assert!(existing
        .existing_directory()
        .unwrap()
        .metadata()
        .unwrap()
        .is_dir());
    let missing = output.join("not-created/leaf");
    let proposed = DestinationInspection::inspect(&[&store], &missing, Wait::Try).unwrap();
    proposed.require_empty(Wait::Try).unwrap();
    assert!(proposed.existing_directory().is_none());
    assert!(!missing.exists());
    assert_eq!(fs::read_dir(&output).unwrap().count(), 0);
    assert_eq!(fs::read(store.join("sentinel")).unwrap(), b"retained");
}

#[test]
fn empty_destination_counts_hidden_files_directories_and_dangling_links() {
    for kind in 0..4 {
        let (_root, store, output) = fixture();
        let entry = output.join(if kind == 0 { ".hidden" } else { "entry" });
        match kind {
            0 | 1 => fs::write(&entry, b"user bytes").unwrap(),
            2 => fs::create_dir(&entry).unwrap(),
            _ => symlink("absent-target", &entry).unwrap(),
        }
        let observed = DestinationInspection::inspect(&[&store], &output, Wait::Try).unwrap();
        assert_eq!(
            observed
                .require_empty(Wait::Try)
                .unwrap_err()
                .raw_os_error(),
            Some(libc::ENOTEMPTY)
        );
        assert!(fs::symlink_metadata(&entry).is_ok());
        assert_eq!(fs::read_dir(&output).unwrap().count(), 1);
        assert_eq!(fs::read(store.join("sentinel")).unwrap(), b"retained");
    }
}

#[test]
fn empty_destination_scan_has_an_independent_directory_offset() {
    let (_root, store, output) = fixture();
    fs::write(output.join("retained"), b"payload").unwrap();
    let observed = DestinationInspection::inspect(&[&store], &output, Wait::Try).unwrap();
    let held = observed.existing_directory().unwrap();
    // Intentionally advance the retained File's offset through a shared dup.
    // The production empty-check must open its own stream, not inherit this EOF.
    let mut shared = DirectoryStream::open(held.try_clone().unwrap()).unwrap();
    let mut ended = false;
    for _ in 0..5 {
        if matches!(shared.next().unwrap(), Entry::End) {
            ended = true;
            break;
        }
    }
    assert!(ended);
    shared.close().unwrap();
    for _ in 0..2 {
        assert_eq!(
            observed
                .require_empty(Wait::Try)
                .unwrap_err()
                .raw_os_error(),
            Some(libc::ENOTEMPTY)
        );
        assert!(
            held.metadata().unwrap().is_dir(),
            "borrowed handle was closed"
        );
    }
    assert_eq!(fs::read(output.join("retained")).unwrap(), b"payload");
}

#[test]
fn empty_destination_rechecks_contents_without_freezing_them() {
    let (_root, store, output) = fixture();
    let observed = DestinationInspection::inspect(&[&store], &output, Wait::Try).unwrap();
    observed.require_empty(Wait::Try).unwrap();
    fs::write(output.join("new"), b"not overwritten").unwrap();
    assert_eq!(
        observed
            .require_empty(Wait::Try)
            .unwrap_err()
            .raw_os_error(),
        Some(libc::ENOTEMPTY)
    );
    assert_eq!(fs::read(output.join("new")).unwrap(), b"not overwritten");
    fs::remove_file(output.join("new")).unwrap();
    observed.require_empty(Wait::Try).unwrap();
}

#[test]
fn empty_destination_rejects_replacement_and_missing_target_adoption() {
    let (root, store, output) = fixture();
    let observed = DestinationInspection::inspect(&[&store], &output, Wait::Try).unwrap();
    let identity = observed
        .existing_directory()
        .unwrap()
        .metadata()
        .unwrap()
        .ino();
    fs::rename(&output, root.path().join("retained-output")).unwrap();
    fs::create_dir(&output).unwrap();
    assert!(observed.require_empty(Wait::Try).is_err());
    assert_eq!(
        observed
            .existing_directory()
            .unwrap()
            .metadata()
            .unwrap()
            .ino(),
        identity
    );
    let missing = output.join("new");
    let proposed = DestinationInspection::inspect(&[&store], &missing, Wait::Try).unwrap();
    fs::create_dir(&missing).unwrap();
    assert!(proposed.require_empty(Wait::Try).is_err());
    assert!(proposed.existing_directory().is_none());
    assert!(missing.is_dir());
}

#[test]
fn empty_destination_alias_parent_keeps_native_not_lexical_target() {
    let (root, store, output) = fixture();
    fs::create_dir(output.join("branch")).unwrap();
    fs::create_dir(output.join("selected")).unwrap();
    fs::create_dir(root.path().join("selected")).unwrap();
    fs::write(root.path().join("selected/user"), b"lexical decoy").unwrap();
    symlink(output.join("branch"), root.path().join("alias")).unwrap();
    let path = root.path().join("alias/../selected");
    let observed = DestinationInspection::inspect(&[&store], &path, Wait::Try).unwrap();
    observed.require_empty(Wait::Try).unwrap();
    fs::write(output.join("selected/native"), b"actual target").unwrap();
    assert_eq!(
        observed
            .require_empty(Wait::Try)
            .unwrap_err()
            .raw_os_error(),
        Some(libc::ENOTEMPTY)
    );
    assert_eq!(
        fs::read(root.path().join("selected/user")).unwrap(),
        b"lexical decoy"
    );
}

#[test]
fn empty_destination_cancellation_and_deadline_do_not_modify_output() {
    let (_root, store, output) = fixture();
    let observed = DestinationInspection::inspect(&[&store], &output, Wait::Try).unwrap();
    let cancellation = Cancellation::default();
    let expired = Wait::Until {
        deadline: Instant::now() - Duration::from_secs(1),
        cancellation: &cancellation,
    };
    assert_eq!(
        observed.require_empty(expired).unwrap_err().kind(),
        io::ErrorKind::TimedOut
    );
    cancellation.cancel();
    let cancelled = Wait::Until {
        deadline: Instant::now() + Duration::from_secs(60),
        cancellation: &cancellation,
    };
    assert_eq!(
        observed.require_empty(cancelled).unwrap_err().kind(),
        io::ErrorKind::Interrupted
    );
    assert_eq!(fs::read_dir(&output).unwrap().count(), 0);
}

#[test]
fn empty_destination_scan_checks_cancellation_after_eof() {
    let cancellation = Cancellation::default();
    let wait = Wait::Until {
        deadline: Instant::now() + Duration::from_secs(60),
        cancellation: &cancellation,
    };
    let result = scan_empty(wait, || {
        cancellation.cancel();
        Ok(Entry::End)
    });
    assert_eq!(result.unwrap_err().kind(), io::ErrorKind::Interrupted);
}

#[test]
fn empty_destination_scan_preserves_read_errors_and_bounds_dot_streams() {
    for code in [libc::EIO, libc::EBADF] {
        let result = scan_empty(Wait::Try, || Err(io::Error::from_raw_os_error(code)));
        assert_eq!(result.unwrap_err().raw_os_error(), Some(code));
    }
    let mut calls = 0;
    let result = scan_empty(Wait::Try, || {
        calls += 1;
        Ok(Entry::Dot)
    });
    assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidData);
    assert_eq!(calls, 3);
}

#[test]
fn empty_destination_with_planned_root_keeps_absent_paths_absent() {
    use crate::store::foundation::topology::ProtectedRoot;
    let (_root, store, output) = fixture();
    let planned = store.join("future-store");
    let roots = [ProtectedRoot::PlannedDirectory(&planned)];
    let present = DestinationInspection::inspect_configured(&roots, &output, Wait::Try).unwrap();
    present.require_empty(Wait::Try).unwrap();
    let missing = output.join("future-output/leaf");
    let absent = DestinationInspection::inspect_configured(&roots, &missing, Wait::Try).unwrap();
    absent.require_empty(Wait::Try).unwrap();
    assert!(!planned.exists());
    assert!(!output.join("future-output").exists());
    assert_eq!(fs::read(store.join("sentinel")).unwrap(), b"retained");
}

#[test]
fn empty_destination_rejects_a_planned_root_created_since_inspection() {
    use crate::store::foundation::topology::ProtectedRoot;
    let (_root, store, output) = fixture();
    let planned = store.join("future-store");
    let observed = DestinationInspection::inspect_configured(
        &[ProtectedRoot::PlannedDirectory(&planned)],
        &output,
        Wait::Try,
    )
    .unwrap();
    observed.require_empty(Wait::Try).unwrap();
    fs::create_dir(&planned).unwrap();
    assert_eq!(
        observed.require_empty(Wait::Try).unwrap_err().kind(),
        io::ErrorKind::InvalidData
    );
    assert!(planned.is_dir());
    assert_eq!(fs::read_dir(&output).unwrap().count(), 0);
    assert_eq!(fs::read(store.join("sentinel")).unwrap(), b"retained");
}

#[cfg(target_os = "linux")]
#[test]
fn empty_destination_non_utf8_name_is_an_entry_not_an_omission() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    let (_root, store, output) = fixture();
    let path = output.join(OsStr::from_bytes(b"\xff"));
    fs::write(&path, b"preserved").unwrap();
    let observed = DestinationInspection::inspect(&[&store], &output, Wait::Try).unwrap();
    assert_eq!(
        observed
            .require_empty(Wait::Try)
            .unwrap_err()
            .raw_os_error(),
        Some(libc::ENOTEMPTY)
    );
    assert_eq!(fs::read(&path).unwrap(), b"preserved");
}
