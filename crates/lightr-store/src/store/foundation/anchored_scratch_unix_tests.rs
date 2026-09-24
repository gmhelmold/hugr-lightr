use super::*;
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::{Path, PathBuf};

fn reserved_path(root: &Path) -> PathBuf {
    let paths = fs::read_dir(root.join(".si01-staging"))
        .unwrap()
        .map(|item| item.unwrap().path())
        .collect::<Vec<_>>();
    assert_eq!(paths.len(), 1);
    paths[0].clone()
}

#[test]
fn scratch_round_trip_and_checked_cleanup() {
    let root = TempDir::new().unwrap();
    fs::write(root.path().join("sentinel"), b"retained").unwrap();
    let locks = StoreLocks::open_existing(root.path()).unwrap();
    let lease = locks.shared(Wait::Try).unwrap();
    let scratch = ScratchDirectory::reserve(&lease, Wait::Try).unwrap();
    let directory = scratch.directory("exact name", Wait::Try).unwrap();
    let mut file = directory.file("bytes", Wait::Try).unwrap();
    let data = vec![37u8; 131_073];
    file.write_all(&data).unwrap();
    file.flush().unwrap();
    file.rewind().unwrap();
    let mut actual = Vec::new();
    file.read_to_end(&mut actual).unwrap();
    assert_eq!(actual, data);
    let path = reserved_path(root.path());
    assert_eq!(fs::read(path.join("exact name/bytes")).unwrap(), data);
    file.finish().unwrap();
    directory.finish().unwrap();
    scratch.finish().unwrap();
    assert!(!path.exists());
    assert_eq!(fs::read(root.path().join("sentinel")).unwrap(), b"retained");
}

#[test]
fn scratch_existing_names_are_never_adopted_or_truncated() {
    let root = TempDir::new().unwrap();
    let locks = StoreLocks::open_existing(root.path()).unwrap();
    let lease = locks.shared(Wait::Try).unwrap();
    let scratch = ScratchDirectory::reserve(&lease, Wait::Try).unwrap();
    let mut existing = scratch.file("occupied", Wait::Try).unwrap();
    existing.write_all(b"original bytes").unwrap();
    assert_eq!(
        scratch
            .file("occupied", Wait::Try)
            .err()
            .expect("existing file was adopted")
            .kind(),
        io::ErrorKind::AlreadyExists
    );
    assert_eq!(
        scratch
            .directory("occupied", Wait::Try)
            .err()
            .unwrap()
            .kind(),
        io::ErrorKind::AlreadyExists
    );
    let path = reserved_path(root.path());
    assert_eq!(fs::read(path.join("occupied")).unwrap(), b"original bytes");
    symlink("absent", path.join("dangling")).unwrap();
    assert_eq!(
        scratch.file("dangling", Wait::Try).err().unwrap().kind(),
        io::ErrorKind::AlreadyExists
    );
    assert_eq!(
        scratch
            .directory("dangling", Wait::Try)
            .err()
            .unwrap()
            .kind(),
        io::ErrorKind::AlreadyExists
    );
    assert_eq!(
        fs::read_link(path.join("dangling")).unwrap(),
        Path::new("absent")
    );
    assert!(!path.join("absent").exists());
    fs::remove_file(path.join("dangling")).unwrap();
    existing.finish().unwrap();
    scratch.finish().unwrap();
}

#[test]
fn scratch_invalid_components_do_not_access_other_paths() {
    let root = TempDir::new().unwrap();
    let locks = StoreLocks::open_existing(root.path()).unwrap();
    let lease = locks.shared(Wait::Try).unwrap();
    let scratch = ScratchDirectory::reserve(&lease, Wait::Try).unwrap();
    for name in ["", ".", "..", "one/two", "/absolute", "nul\0byte"] {
        assert_eq!(
            scratch.file(name, Wait::Try).err().unwrap().kind(),
            io::ErrorKind::InvalidInput
        );
        assert_eq!(
            scratch.directory(name, Wait::Try).err().unwrap().kind(),
            io::ErrorKind::InvalidInput
        );
    }
    assert_eq!(fs::read_dir(reserved_path(root.path())).unwrap().count(), 0);
    scratch.finish().unwrap();
}

#[test]
fn scratch_anchor_ignores_replaced_path_spelling() {
    let root = TempDir::new().unwrap();
    let locks = StoreLocks::open_existing(root.path()).unwrap();
    let lease = locks.shared(Wait::Try).unwrap();
    let scratch = ScratchDirectory::reserve(&lease, Wait::Try).unwrap();
    let old = reserved_path(root.path());
    let moved = root.path().join("retained-reservation");
    let decoy = root.path().join("unrelated");
    fs::create_dir(&decoy).unwrap();
    fs::write(decoy.join("sentinel"), b"unchanged").unwrap();
    fs::rename(&old, &moved).unwrap();
    symlink(&decoy, &old).unwrap();
    let directory = scratch.directory("child", Wait::Try).unwrap();
    let mut file = directory.file("new", Wait::Try).unwrap();
    file.write_all(b"anchored").unwrap();
    assert_eq!(fs::read(moved.join("child/new")).unwrap(), b"anchored");
    assert!(!decoy.join("child").exists());
    file.finish().unwrap();
    directory.finish().unwrap();
    assert_eq!(
        scratch.finish().unwrap_err().kind(),
        io::ErrorKind::InvalidData
    );
    assert!(fs::symlink_metadata(&old).unwrap().file_type().is_symlink());
    assert_eq!(fs::read(decoy.join("sentinel")).unwrap(), b"unchanged");
    assert!(moved.is_dir());
}

#[test]
fn scratch_cleanup_preserves_replacement_entries() {
    let root = TempDir::new().unwrap();
    let locks = StoreLocks::open_existing(root.path()).unwrap();
    let lease = locks.shared(Wait::Try).unwrap();
    let scratch = ScratchDirectory::reserve(&lease, Wait::Try).unwrap();
    let mut file = scratch.file("entry", Wait::Try).unwrap();
    file.write_all(b"owned").unwrap();
    let path = reserved_path(root.path());
    fs::rename(path.join("entry"), path.join("retained-owned")).unwrap();
    fs::write(path.join("entry"), b"replacement").unwrap();
    assert_eq!(
        file.finish().expect_err("replacement was removed").kind(),
        io::ErrorKind::InvalidData
    );
    assert_eq!(fs::read(path.join("entry")).unwrap(), b"replacement");
    assert_eq!(fs::read(path.join("retained-owned")).unwrap(), b"owned");
    assert_eq!(
        scratch.finish().unwrap_err().raw_os_error(),
        Some(libc::ENOTEMPTY)
    );
    assert_eq!(fs::read(path.join("entry")).unwrap(), b"replacement");
}

#[test]
fn scratch_unknown_children_prevent_recursive_cleanup() {
    let root = TempDir::new().unwrap();
    let locks = StoreLocks::open_existing(root.path()).unwrap();
    let lease = locks.shared(Wait::Try).unwrap();
    let scratch = ScratchDirectory::reserve(&lease, Wait::Try).unwrap();
    let path = reserved_path(root.path());
    fs::write(path.join("unexpected"), b"never remove me").unwrap();
    assert_eq!(
        scratch.finish().unwrap_err().raw_os_error(),
        Some(libc::ENOTEMPTY)
    );
    assert_eq!(
        fs::read(path.join("unexpected")).unwrap(),
        b"never remove me"
    );
}

#[test]
fn scratch_drop_removes_only_owned_names() {
    let root = TempDir::new().unwrap();
    let locks = StoreLocks::open_existing(root.path()).unwrap();
    let lease = locks.shared(Wait::Try).unwrap();
    {
        let scratch = ScratchDirectory::reserve(&lease, Wait::Try).unwrap();
        let directory = scratch.directory("nested", Wait::Try).unwrap();
        let mut file = directory.file("bytes", Wait::Try).unwrap();
        file.write_all(b"temporary").unwrap();
    }
    assert_eq!(
        fs::read_dir(root.path().join(".si01-staging"))
            .unwrap()
            .count(),
        0
    );
}

#[test]
fn scratch_child_cancellation_does_not_allocate() {
    let root = TempDir::new().unwrap();
    let locks = StoreLocks::open_existing(root.path()).unwrap();
    let lease = locks.shared(Wait::Try).unwrap();
    let scratch = ScratchDirectory::reserve(&lease, Wait::Try).unwrap();
    let cancellation = Cancellation::default();
    cancellation.cancel();
    let wait = Wait::Until {
        deadline: Instant::now() + Duration::from_secs(5),
        cancellation: &cancellation,
    };
    assert_eq!(
        scratch.directory("cancelled", wait).err().unwrap().kind(),
        io::ErrorKind::Interrupted
    );
    assert_eq!(
        scratch.file("cancelled", wait).err().unwrap().kind(),
        io::ErrorKind::Interrupted
    );
    assert_eq!(fs::read_dir(reserved_path(root.path())).unwrap().count(), 0);
    scratch.finish().unwrap();
}

#[test]
fn scratch_allocations_progress_independently() {
    let root = TempDir::new().unwrap();
    let locks = StoreLocks::open_existing(root.path()).unwrap();
    let lease = locks.shared(Wait::Try).unwrap();
    std::thread::scope(|scope| {
        let threads = (0..4)
            .map(|_| {
                scope.spawn(|| {
                    let scratch = ScratchDirectory::reserve(&lease, Wait::Try).unwrap();
                    let mut file = scratch.file("same-name", Wait::Try).unwrap();
                    file.write_all(b"independent").unwrap();
                    file.finish().unwrap();
                    scratch.finish().unwrap();
                })
            })
            .collect::<Vec<_>>();
        for thread in threads {
            thread.join().unwrap();
        }
    });
    assert_eq!(
        fs::read_dir(root.path().join(".si01-staging"))
            .unwrap()
            .count(),
        0
    );
}

#[test]
fn scratch_modes_and_lease_exclusion_remain_private() {
    let root = TempDir::new().unwrap();
    let locks = StoreLocks::open_existing(root.path()).unwrap();
    let lease = locks.shared(Wait::Try).unwrap();
    let scratch = ScratchDirectory::reserve(&lease, Wait::Try).unwrap();
    let directory = scratch.directory("child", Wait::Try).unwrap();
    let file = directory.file("probe", Wait::Try).unwrap();
    let path = reserved_path(root.path());
    assert_eq!(fs::metadata(&path).unwrap().permissions().mode() & 0o077, 0);
    assert_eq!(
        fs::metadata(path.join("child"))
            .unwrap()
            .permissions()
            .mode()
            & 0o077,
        0
    );
    assert_eq!(
        fs::metadata(path.join("child/probe"))
            .unwrap()
            .permissions()
            .mode()
            & 0o177,
        0
    );
    assert_eq!(
        locks.exclusive(Wait::Try).unwrap_err().kind(),
        io::ErrorKind::WouldBlock
    );
    file.finish().unwrap();
    directory.finish().unwrap();
    scratch.finish().unwrap();
    drop(lease);
    locks.exclusive(Wait::Try).unwrap();
}
