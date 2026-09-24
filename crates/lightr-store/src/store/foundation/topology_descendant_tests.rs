//! Native owned-fixture checks. No user directory, cwd, environment or process changes.
use super::*;
use crate::store::foundation::{Cancellation, StoreLocks};
use std::io::Read;
use std::os::unix::fs::{symlink, MetadataExt};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tempfile::TempDir;

struct Fixture {
    _root: TempDir,
    store: PathBuf,
    source: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let root = TempDir::new().unwrap();
        let store = root.path().join("store");
        let source = root.path().join("source");
        std::fs::create_dir(&store).unwrap();
        std::fs::create_dir_all(source.join("nested")).unwrap();
        std::fs::write(source.join("nested/data"), b"captured input").unwrap();
        Self {
            _root: root,
            store,
            source,
        }
    }
    fn inspect(&self) -> TopologyInspection {
        TopologyInspection::inspect(
            &[&self.store],
            SourcePath::Directory(&self.source),
            None,
            Wait::Try,
        )
        .unwrap()
    }
}
fn file(path: &str) -> SourcePath<'_> {
    SourcePath::File(Path::new(path))
}
fn contents(mut f: File) -> Vec<u8> {
    let mut b = Vec::new();
    f.read_to_end(&mut b).unwrap();
    b
}

#[test]
fn descendant_nested_file_and_directory_preserve_opened_identity() {
    let f = Fixture::new();
    let observed = f.inspect();
    let opened = observed
        .open_source_descendant(file("nested/data"), Wait::Try)
        .unwrap();
    let a = opened.metadata().unwrap();
    let b = std::fs::metadata(f.source.join("nested/data")).unwrap();
    assert_eq!((a.dev(), a.ino()), (b.dev(), b.ino()));
    assert_eq!(contents(opened), b"captured input");
    let dir = observed
        .open_source_descendant(SourcePath::Directory(Path::new("nested")), Wait::Try)
        .unwrap();
    assert!(dir.metadata().unwrap().is_dir());
    let flags = unsafe { libc::fcntl(dir.as_raw_fd(), libc::F_GETFD) };
    assert_ne!(flags & libc::FD_CLOEXEC, 0);
    assert!(observed.source_handle().metadata().unwrap().is_dir());
}

#[test]
fn descendant_each_file_open_has_an_independent_offset() {
    let f = Fixture::new();
    let observed = f.inspect();
    let a = observed
        .open_source_descendant(file("nested/data"), Wait::Try)
        .unwrap();
    assert_eq!(contents(a), b"captured input");
    let b = observed
        .open_source_descendant(file("nested/data"), Wait::Try)
        .unwrap();
    assert_eq!(contents(b), b"captured input");
}

#[test]
fn descendant_rejects_links_at_every_component() {
    let f = Fixture::new();
    let observed = f.inspect();
    symlink("nested", f.source.join("alias")).unwrap();
    symlink("nested/data", f.source.join("leaf-link")).unwrap();
    symlink("missing", f.source.join("dangling")).unwrap();
    for path in ["alias/data", "leaf-link", "dangling"] {
        assert!(
            observed
                .open_source_descendant(file(path), Wait::Try)
                .is_err(),
            "descendant symbolic link was followed"
        );
    }
    assert!(
        observed
            .open_source_descendant(SourcePath::Directory(Path::new("alias")), Wait::Try)
            .is_err(),
        "directory link was followed"
    );
    assert_eq!(
        std::fs::read(f.source.join("nested/data")).unwrap(),
        b"captured input"
    );
}

#[test]
fn descendant_rejects_non_normal_relative_components() {
    let f = Fixture::new();
    let observed = f.inspect();
    for path in [
        "",
        ".",
        "..",
        "/nested/data",
        "nested/../nested/data",
        "nested/./data",
        "nested//data",
        "nested/data/",
        "nested/\0data",
    ] {
        assert_eq!(
            observed
                .open_source_descendant(file(path), Wait::Try)
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidInput,
            "path: {path:?}"
        );
    }
}

#[test]
fn descendant_work_budgets_reject_before_traversal() {
    let f = Fixture::new();
    let observed = f.inspect();
    for path in ["x".repeat(4097), vec!["x"; 129].join("/")] {
        assert_eq!(
            observed
                .open_source_descendant(file(&path), Wait::Try)
                .unwrap_err()
                .kind(),
            io::ErrorKind::Unsupported
        );
    }
}

#[test]
fn descendant_native_missing_type_and_hardlink_errors_are_preserved() {
    let f = Fixture::new();
    let observed = f.inspect();
    assert_eq!(
        observed
            .open_source_descendant(file("nested/missing"), Wait::Try)
            .unwrap_err()
            .raw_os_error(),
        Some(libc::ENOENT)
    );
    assert!(observed
        .open_source_descendant(SourcePath::Directory(Path::new("nested/data")), Wait::Try)
        .is_err());
    assert!(observed
        .open_source_descendant(file("nested"), Wait::Try)
        .is_err());
    std::fs::hard_link(f.source.join("nested/data"), f.source.join("second-name")).unwrap();
    assert_eq!(
        observed
            .open_source_descendant(file("nested/data"), Wait::Try)
            .unwrap_err()
            .kind(),
        io::ErrorKind::Unsupported,
        "multiply linked descendant accepted"
    );
    let single = TopologyInspection::inspect(
        &[&f.store],
        SourcePath::File(&f.source.join("second-name")),
        None,
        Wait::Try,
    );
    assert!(single.is_err());
}

#[test]
fn descendant_file_source_cannot_be_used_as_a_directory() {
    let f = Fixture::new();
    let observed = TopologyInspection::inspect(
        &[&f.store],
        SourcePath::File(&f.source.join("nested/data")),
        None,
        Wait::Try,
    )
    .unwrap();
    assert_eq!(
        observed
            .open_source_descendant(file("child"), Wait::Try)
            .unwrap_err()
            .kind(),
        io::ErrorKind::InvalidInput
    );
}

#[test]
fn descendant_revalidates_the_original_source_before_opening() {
    let f = Fixture::new();
    let observed = f.inspect();
    std::fs::rename(&f.source, f._root.path().join("old-source")).unwrap();
    std::fs::create_dir_all(f.source.join("nested")).unwrap();
    std::fs::write(f.source.join("nested/data"), b"replacement").unwrap();
    assert!(
        observed
            .open_source_descendant(file("nested/data"), Wait::Try)
            .is_err(),
        "stale original source accepted"
    );
    assert_eq!(
        std::fs::read(f.source.join("nested/data")).unwrap(),
        b"replacement"
    );
}

#[test]
fn descendant_revalidates_each_component_after_opening() {
    let f = Fixture::new();
    let observed = f.inspect();
    let result = open_checked(&observed, file("nested/data"), Wait::Try, |index| {
        if index == 1 {
            std::fs::rename(f.source.join("nested"), f.source.join("previous"))?;
            std::fs::create_dir(f.source.join("nested"))?;
            std::fs::write(f.source.join("nested/data"), b"replacement")?;
        }
        Ok(())
    });
    assert!(result.is_err(), "substituted descendant directory accepted");
    assert_eq!(
        std::fs::read(f.source.join("previous/data")).unwrap(),
        b"captured input"
    );
    assert_eq!(
        std::fs::read(f.source.join("nested/data")).unwrap(),
        b"replacement"
    );
}

#[test]
fn descendant_opened_file_remains_the_original_after_later_rename() {
    let f = Fixture::new();
    let observed = f.inspect();
    let opened = observed
        .open_source_descendant(file("nested/data"), Wait::Try)
        .unwrap();
    std::fs::rename(f.source.join("nested/data"), f.source.join("old-data")).unwrap();
    std::fs::write(f.source.join("nested/data"), b"new pathname content").unwrap();
    assert_eq!(contents(opened), b"captured input");
    assert_eq!(
        std::fs::read(f.source.join("nested/data")).unwrap(),
        b"new pathname content"
    );
}

#[test]
fn descendant_cancellation_deadline_and_scoped_errors_are_terminal() {
    let f = Fixture::new();
    let observed = f.inspect();
    let cancellation = Cancellation::default();
    let wait = Wait::Until {
        deadline: Instant::now() + Duration::from_secs(60),
        cancellation: &cancellation,
    };
    let result = open_checked(&observed, file("nested/data"), wait, |_| {
        cancellation.cancel();
        Ok(())
    });
    assert_eq!(result.unwrap_err().kind(), io::ErrorKind::Interrupted);
    assert_eq!(
        observed
            .open_source_descendant(file("nested/data"), wait)
            .unwrap_err()
            .kind(),
        io::ErrorKind::Interrupted
    );
    let active = Cancellation::default();
    let expired = Wait::Until {
        deadline: Instant::now() - Duration::from_secs(1),
        cancellation: &active,
    };
    assert_eq!(
        observed
            .open_source_descendant(file("nested/data"), expired)
            .unwrap_err()
            .kind(),
        io::ErrorKind::TimedOut
    );
    let result = open_checked(&observed, file("nested/data"), Wait::Try, |_| {
        Err(io::Error::from_raw_os_error(libc::EIO))
    });
    assert_eq!(result.unwrap_err().raw_os_error(), Some(libc::EIO));
}

#[test]
fn descendant_preserves_native_name_bytes_and_does_not_acquire_a_lease() {
    let f = Fixture::new();
    let observed = f.inspect();
    let name = "nested/back\\slash:é";
    std::fs::write(f.source.join(name), b"exact name").unwrap();
    let locks = StoreLocks::open_existing(&f.store).unwrap();
    let _exclusive = locks.exclusive(Wait::Try).unwrap();
    assert_eq!(
        contents(
            observed
                .open_source_descendant(file(name), Wait::Try)
                .unwrap()
        ),
        b"exact name"
    );
    assert!(!f.source.join(".gc.lock").exists());
}

#[cfg(target_os = "linux")]
#[test]
fn descendant_linux_non_utf8_name_is_preserved() {
    use std::ffi::OsStr;
    let f = Fixture::new();
    let observed = f.inspect();
    let name = Path::new(OsStr::from_bytes(b"nested/\xff"));
    std::fs::write(f.source.join(name), b"raw name").unwrap();
    assert_eq!(
        contents(
            observed
                .open_source_descendant(SourcePath::File(name), Wait::Try)
                .unwrap()
        ),
        b"raw name"
    );
}
