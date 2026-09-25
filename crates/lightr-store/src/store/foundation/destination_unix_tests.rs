//! Fixtures are disposable. No user data, real Store or output is modified.
use super::*;
use crate::store::foundation::topology::{SourcePath, TopologyInspection};
use std::fs::File;
use std::os::unix::fs::{symlink, MetadataExt};
use std::path::PathBuf;

fn fixture() -> (TempDir, PathBuf, PathBuf) {
    let root = TempDir::new().unwrap();
    let store = root.path().join("store");
    let output = root.path().join("output");
    fs::create_dir(&store).unwrap();
    fs::create_dir(&output).unwrap();
    fs::write(store.join("sentinel"), b"retained").unwrap();
    (root, store, output)
}

fn id(file: &File) -> (u64, u64) {
    let m = file.metadata().unwrap();
    (m.dev(), m.ino())
}

#[test]
fn destination_existing_handle_is_the_requested_directory() {
    let (root, store, output) = fixture();
    fs::write(output.join("existing"), b"user data").unwrap();
    let observed = DestinationInspection::inspect(&[&store], &output, Wait::Try).unwrap();
    assert!(!observed.is_missing());
    let handle = observed.existing_directory().unwrap();
    assert!(handle.metadata().unwrap().is_dir());
    assert_eq!(id(handle), id(&File::open(&output).unwrap()));
    observed.revalidate(Wait::Try).unwrap();
    assert_eq!(fs::read(output.join("existing")).unwrap(), b"user data");
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 2);
}

#[test]
fn destination_checks_each_protected_root_and_all_overlap_directions() {
    let (root, store, output) = fixture();
    let inner = store.join("inside");
    fs::create_dir(&inner).unwrap();
    for path in [store.as_path(), inner.as_path(), root.path()] {
        let error = DestinationInspection::inspect(&[&store], path, Wait::Try)
            .err()
            .expect("protected destination was accepted");
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    }
    assert!(DestinationInspection::inspect(&[&store, &output], &output, Wait::Try).is_err());
    let sibling = root.path().join("store-backup");
    fs::create_dir(&sibling).unwrap();
    DestinationInspection::inspect(&[&store], &sibling, Wait::Try).unwrap();
    assert_eq!(fs::read(store.join("sentinel")).unwrap(), b"retained");
}

#[test]
fn destination_missing_target_never_exposes_its_existing_ancestor() {
    let (root, store, output) = fixture();
    let alias = root.path().join("alias");
    symlink(&output, &alias).unwrap();
    let missing = alias.join("new/leaf");
    let observed = DestinationInspection::inspect(&[&store], &missing, Wait::Try).unwrap();
    assert!(
        observed.existing_directory().is_none(),
        "ancestor exposed as destination"
    );
    assert!(observed.is_missing());
    observed.revalidate(Wait::Try).unwrap();
    assert_eq!(fs::read_dir(&output).unwrap().count(), 0);
    fs::create_dir_all(&missing).unwrap();
    assert!(
        observed.revalidate(Wait::Try).is_err(),
        "changed destination was accepted"
    );
    assert!(observed.existing_directory().is_none());
    let new_observation = DestinationInspection::inspect(&[&store], &missing, Wait::Try).unwrap();
    assert_eq!(
        id(new_observation.existing_directory().unwrap()),
        id(&File::open(&missing).unwrap())
    );
}

#[test]
fn destination_alias_and_parent_use_native_directory_identity() {
    let (root, store, output) = fixture();
    fs::create_dir(output.join("branch")).unwrap();
    fs::create_dir(output.join("selected")).unwrap();
    let alias = root.path().join("alias");
    symlink(output.join("branch"), &alias).unwrap();
    let requested = alias.join("../selected");
    let observed = DestinationInspection::inspect(&[&store], &requested, Wait::Try).unwrap();
    assert_eq!(
        id(observed.existing_directory().unwrap()),
        id(&File::open(&requested).unwrap())
    );
    assert_eq!(
        id(observed.existing_directory().unwrap()),
        id(&File::open(output.join("selected")).unwrap())
    );
}

#[test]
fn destination_revalidation_detects_replacement_without_retargeting_handle() {
    let (root, store, output) = fixture();
    let observed = DestinationInspection::inspect(&[&store], &output, Wait::Try).unwrap();
    let original = id(observed.existing_directory().unwrap());
    let retained = root.path().join("retained-output");
    fs::rename(&output, &retained).unwrap();
    fs::create_dir(&output).unwrap();
    assert!(
        observed.revalidate(Wait::Try).is_err(),
        "changed destination was accepted"
    );
    assert_eq!(id(observed.existing_directory().unwrap()), original);
    assert_eq!(original, id(&File::open(&retained).unwrap()));
    assert_ne!(original, id(&File::open(&output).unwrap()));
    assert_eq!(fs::read_dir(&output).unwrap().count(), 0);
}

#[test]
fn destination_missing_protected_root_and_regular_file_are_not_created_or_adopted() {
    let (root, store, output) = fixture();
    let missing_root = root.path().join("absent-store");
    let missing_output = output.join("uncreated");
    let error = DestinationInspection::inspect(&[&missing_root], &missing_output, Wait::Try)
        .err()
        .unwrap();
    assert_eq!(error.kind(), io::ErrorKind::NotFound);
    assert!(!missing_root.exists());
    assert!(!missing_output.exists());
    let file = output.join("file");
    fs::write(&file, b"untouched").unwrap();
    assert!(DestinationInspection::inspect(&[&store], &file, Wait::Try).is_err());
    assert_eq!(fs::read(&file).unwrap(), b"untouched");
}

#[test]
fn destination_revalidation_observes_cancellation_and_deadline() {
    let (_root, store, output) = fixture();
    let observed = DestinationInspection::inspect(&[&store], &output, Wait::Try).unwrap();
    let cancellation = Cancellation::default();
    let expired = observed
        .revalidate(Wait::Until {
            deadline: Instant::now() - Duration::from_secs(1),
            cancellation: &cancellation,
        })
        .unwrap_err();
    assert_eq!(expired.kind(), io::ErrorKind::TimedOut);
    cancellation.cancel();
    let stopped = observed
        .revalidate(Wait::Until {
            deadline: Instant::now() + Duration::from_secs(5),
            cancellation: &cancellation,
        })
        .unwrap_err();
    assert_eq!(stopped.kind(), io::ErrorKind::Interrupted);
    assert_eq!(fs::read_dir(&output).unwrap().count(), 0);
}

#[test]
fn destination_only_does_not_change_source_only_or_paired_inspection() {
    let (root, store, output) = fixture();
    let source = root.path().join("source");
    fs::create_dir(&source).unwrap();
    let alone =
        TopologyInspection::inspect(&[&store], SourcePath::Directory(&source), None, Wait::Try)
            .unwrap();
    assert!(!alone.destination_is_missing());
    assert_eq!(id(alone.source_handle()), id(&File::open(&source).unwrap()));
    let paired = TopologyInspection::inspect(
        &[&store],
        SourcePath::Directory(&source),
        Some(&output.join("new")),
        Wait::Try,
    )
    .unwrap();
    assert!(paired.destination_is_missing());
    paired.revalidate(Wait::Try).unwrap();
    assert_eq!(id(paired.source_handle()), id(alone.source_handle()));
    assert!(!output.join("new").exists());
}
