use super::*;
use crate::store::foundation::topology::{DestinationInspection, ProtectedRoot};
use crate::store::foundation::Cancellation;
use std::fs;
use std::os::fd::AsRawFd;
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tempfile::TempDir;

struct Fixture {
    root: TempDir,
    store: PathBuf,
    parent: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let root = TempDir::new().unwrap();
        let store = root.path().join("store");
        let parent = root.path().join("public");
        fs::create_dir(&store).unwrap();
        fs::create_dir(&parent).unwrap();
        fs::write(store.join("sentinel"), b"store").unwrap();
        Self {
            root,
            store,
            parent,
        }
    }

    fn inspect(&self, destination: &std::path::Path) -> DestinationInspection {
        DestinationInspection::inspect(&[&self.store], destination, Wait::Try).unwrap()
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn anchor_existing_empty_uses_exact_directory_and_rollback_never_removes_it() {
    let f = Fixture::new();
    let output = f.parent.join("output");
    fs::create_dir(&output).unwrap();
    let observed = f.inspect(&output);
    let anchor = observed.anchor_empty(Wait::Try).unwrap();
    assert!(!anchor.was_created());
    assert_eq!(anchor.created_components(), 0);
    anchor.revalidate(Wait::Try).unwrap();
    anchor.rollback_empty().unwrap();
    assert!(output.is_dir());
    assert_eq!(fs::read_dir(&output).unwrap().count(), 0);
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn anchor_existing_nonempty_is_rejected_without_touching_user_data() {
    let f = Fixture::new();
    let output = f.parent.join("output");
    fs::create_dir(&output).unwrap();
    fs::write(output.join("keep"), b"user").unwrap();
    let error = f.inspect(&output).anchor_empty(Wait::Try).err().unwrap();
    assert_eq!(error.primary.unwrap().raw_os_error(), Some(libc::ENOTEMPTY));
    assert!(error.cleanup.is_empty());
    assert!(error.cleanup_complete);
    assert_eq!(fs::read(output.join("keep")).unwrap(), b"user");
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn anchor_missing_nested_destination_creates_exact_owned_suffix_and_rolls_back() {
    let f = Fixture::new();
    let output = f.parent.join("one/two/output");
    let observed = f.inspect(&output);
    let anchor = observed.anchor_empty(Wait::Try).unwrap();
    assert!(anchor.was_created());
    assert_eq!(anchor.created_components(), 3);
    anchor.revalidate(Wait::Try).unwrap();

    for path in [
        f.parent.join("one"),
        f.parent.join("one/two"),
        output.clone(),
    ] {
        let mode = fs::metadata(path).unwrap().permissions().mode() & 0o777;
        assert_eq!(
            mode & 0o077,
            0,
            "created destination component was not private"
        );
    }

    anchor.rollback_empty().unwrap();
    assert!(!f.parent.join("one").exists());
    assert_eq!(fs::read_dir(&f.parent).unwrap().count(), 0);
    assert_eq!(fs::read(f.store.join("sentinel")).unwrap(), b"store");
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn anchor_missing_alias_parent_uses_native_resolution_not_lexical_parent() {
    let f = Fixture::new();
    let actual = f.root.path().join("actual");
    let child = actual.join("child");
    fs::create_dir_all(&child).unwrap();
    let lexical = f.root.path().join("target");
    fs::create_dir(&lexical).unwrap();
    let alias = f.root.path().join("alias");
    symlink(&child, &alias).unwrap();

    let requested = alias.join("../target/output");
    let actual_output = actual.join("target/output");
    fs::create_dir(actual.join("target")).unwrap();
    let observed = f.inspect(&requested);
    let anchor = observed.anchor_empty(Wait::Try).unwrap();

    assert!(actual_output.is_dir());
    assert!(!lexical.join("output").exists());
    anchor.rollback_empty().unwrap();
    assert!(!actual_output.exists());
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn anchor_never_adopts_a_name_that_appears_after_preflight() {
    let f = Fixture::new();
    let output = f.parent.join("output");
    let observed = f.inspect(&output);

    let error = anchor_checked(&observed, Wait::Try, |step, _| {
        if step == AnchorStep::AfterPreflight {
            fs::create_dir(&output)?;
            fs::write(output.join("foreign"), b"keep")?;
        }
        Ok(())
    })
    .err()
    .unwrap();

    assert_eq!(error.primary.unwrap().kind(), io::ErrorKind::AlreadyExists);
    assert!(error.cleanup.is_empty());
    assert!(error.cleanup_complete);
    assert_eq!(fs::read(output.join("foreign")).unwrap(), b"keep");
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn anchor_cancellation_after_first_creation_rolls_back_owned_prefix() {
    let f = Fixture::new();
    let output = f.parent.join("one/two");
    let observed = f.inspect(&output);
    let cancellation = Cancellation::default();
    let wait = Wait::Until {
        deadline: Instant::now() + Duration::from_secs(30),
        cancellation: &cancellation,
    };

    let error = anchor_checked(&observed, wait, |step, _| {
        if step == AnchorStep::AfterCreate(0) {
            cancellation.cancel();
        }
        Ok(())
    })
    .err()
    .unwrap();

    assert_eq!(error.primary.unwrap().kind(), io::ErrorKind::Interrupted);
    assert!(error.cleanup.is_empty());
    assert!(error.cleanup_complete);
    assert!(!f.parent.join("one").exists());
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn anchor_final_validation_rejects_replacement_and_preserves_decoy() {
    let f = Fixture::new();
    let output = f.parent.join("output");
    let retained = f.parent.join("retained");
    let observed = f.inspect(&output);

    let error = anchor_checked(&observed, Wait::Try, |step, _| {
        if step == AnchorStep::BeforeFinalValidation {
            fs::rename(&output, &retained)?;
            fs::create_dir(&output)?;
        }
        Ok(())
    })
    .err()
    .unwrap();

    assert_eq!(error.primary.unwrap().kind(), io::ErrorKind::InvalidData);
    assert!(!error.cleanup_complete);
    assert_eq!(error.cleanup.len(), 1);
    assert!(retained.is_dir());
    assert!(output.is_dir(), "replacement output was removed");
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn anchor_existing_replacement_after_empty_scan_is_rejected_without_cleanup() {
    let f = Fixture::new();
    let output = f.parent.join("output");
    let retained = f.parent.join("retained");
    fs::create_dir(&output).unwrap();
    let observed = f.inspect(&output);

    let error = anchor_checked(&observed, Wait::Try, |step, _| {
        if step == AnchorStep::BeforeFinalValidation {
            fs::rename(&output, &retained)?;
            fs::create_dir(&output)?;
        }
        Ok(())
    })
    .err()
    .unwrap();

    assert_eq!(error.primary.unwrap().kind(), io::ErrorKind::InvalidData);
    assert!(error.cleanup.is_empty());
    assert!(error.cleanup_complete);
    assert!(retained.is_dir());
    assert!(output.is_dir());
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn anchor_rollback_preserves_unknown_output_and_reports_partial_state() {
    let f = Fixture::new();
    let output = f.parent.join("output");
    let observed = f.inspect(&output);
    let anchor = observed.anchor_empty(Wait::Try).unwrap();

    fs::write(output.join("unknown"), b"preserve").unwrap();
    let error = anchor.rollback_empty().unwrap_err();

    assert!(error.primary.is_none());
    assert_eq!(error.cleanup.len(), 1);
    assert_eq!(error.cleanup[0].raw_os_error(), Some(libc::ENOTEMPTY));
    assert!(!error.cleanup_complete);
    assert_eq!(fs::read(output.join("unknown")).unwrap(), b"preserve");
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn anchor_created_handle_is_close_on_exec_and_revalidates() {
    let f = Fixture::new();
    let output = f.parent.join("output");
    let observed = f.inspect(&output);
    let anchor = observed.anchor_empty(Wait::Try).unwrap();

    let fd = anchor.anchored_handle().as_raw_fd();
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFD) };
    assert!(flags >= 0);
    assert_ne!(flags & libc::FD_CLOEXEC, 0);
    anchor.revalidate(Wait::Try).unwrap();
    anchor.rollback_empty().unwrap();
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn anchor_revalidates_planned_protected_roots_before_mutation() {
    let f = Fixture::new();
    let managed_parent = f.root.path().join("managed-parent");
    let public_parent = f.root.path().join("public-parent");
    fs::create_dir(&managed_parent).unwrap();
    fs::create_dir(&public_parent).unwrap();
    let planned = managed_parent.join("planned");
    let output = public_parent.join("output");

    let observed = DestinationInspection::inspect_configured(
        &[
            ProtectedRoot::ExistingDirectory(&f.store),
            ProtectedRoot::PlannedDirectory(&planned),
        ],
        &output,
        Wait::Try,
    )
    .unwrap();

    fs::create_dir(&planned).unwrap();
    let error = observed.anchor_empty(Wait::Try).err().unwrap();
    assert!(error.primary.is_some());
    assert!(!output.exists());
    assert!(planned.is_dir());
}
