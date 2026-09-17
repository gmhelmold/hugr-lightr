//! Ownership-specific release tests. The original counterexample stays intact.
use super::*;
use tempfile::TempDir;

#[test]
fn release_of_one_shared_owner_preserves_another_independent_owner() {
    let root = TempDir::new().unwrap();
    let directory = Directory::open(root.path()).unwrap();
    let first = NativeLock::acquire(&directory, ".gc.lock", true, Wait::Try).unwrap();
    let second = NativeLock::acquire(&directory, ".gc.lock", true, Wait::Try).unwrap();
    let duplicate = first._file.try_clone().unwrap();
    drop(first);
    assert_eq!(
        NativeLock::acquire(&directory, ".gc.lock", false, Wait::Try)
            .unwrap_err()
            .kind(),
        io::ErrorKind::WouldBlock
    );
    drop(second);
    let exclusive = NativeLock::acquire(&directory, ".gc.lock", false, Wait::Try).unwrap();
    drop(duplicate);
    assert_eq!(
        NativeLock::acquire(&directory, ".gc.lock", true, Wait::Try)
            .unwrap_err()
            .kind(),
        io::ErrorKind::WouldBlock
    );
    drop(exclusive);
}

#[test]
fn exclusive_owner_release_preserves_stable_file_and_ignores_duplicate_lifetime() {
    let root = TempDir::new().unwrap();
    let path = root.path().join(".gc.lock");
    fs::write(&path, b"preserve-lock-file").unwrap();
    let directory = Directory::open(root.path()).unwrap();
    let owner = NativeLock::acquire(&directory, ".gc.lock", false, Wait::Try).unwrap();
    let original_id = identity(&owner._file).unwrap();
    let duplicate = owner._file.try_clone().unwrap();
    drop(owner);
    let next = NativeLock::acquire(&directory, ".gc.lock", true, Wait::Try).unwrap();
    assert_eq!(identity(&next._file).unwrap(), original_id);
    assert_eq!(fs::read(&path).unwrap(), b"preserve-lock-file");
    drop(next);
    drop(duplicate);
}

#[test]
fn cleanup_with_a_different_owner_process_does_not_unlock_the_live_owner() {
    let root = TempDir::new().unwrap();
    let directory = Directory::open(root.path()).unwrap();
    let owner = NativeLock::acquire(&directory, ".gc.lock", true, Wait::Try).unwrap();
    // A deterministic ownership seam, not a claim of a real fork experiment.
    let inherited = NativeLock {
        _file: owner._file.try_clone().unwrap(),
        owner_process: std::process::id().wrapping_add(1),
    };
    drop(inherited);
    assert_eq!(
        NativeLock::acquire(&directory, ".gc.lock", false, Wait::Try)
            .unwrap_err()
            .kind(),
        io::ErrorKind::WouldBlock
    );
    drop(owner);
    assert!(NativeLock::acquire(&directory, ".gc.lock", false, Wait::Try).is_ok());
}

#[test]
fn unwind_releases_owned_lock_despite_a_duplicate() {
    let root = TempDir::new().unwrap();
    let directory = Directory::open(root.path()).unwrap();
    let owner = NativeLock::acquire(&directory, ".gc.lock", true, Wait::Try).unwrap();
    let duplicate = owner._file.try_clone().unwrap();
    let result = std::panic::catch_unwind(move || {
        let _owner = owner;
        panic!("controlled unwind of the acquiring owner");
    });
    assert!(result.is_err());
    let next = NativeLock::acquire(&directory, ".gc.lock", false, Wait::Try).unwrap();
    drop(next);
    drop(duplicate);
}

#[test]
fn error_after_acquisition_drops_only_its_acquired_lock() {
    let root = TempDir::new().unwrap();
    let directory = Directory::open(root.path()).unwrap();
    let mut duplicate = None;
    let result: io::Result<()> = (|| {
        let owner = NativeLock::acquire(&directory, ".gc.lock", true, Wait::Try)?;
        duplicate = Some(owner._file.try_clone()?);
        Err(io::Error::other(
            "controlled validation failure after acquisition",
        ))
    })();
    assert!(result.is_err());
    let next = NativeLock::acquire(&directory, ".gc.lock", false, Wait::Try).unwrap();
    drop(next);
    drop(duplicate);
}
