//! All mutations below are confined to owned test fixtures; no global hooks.
use super::*;
use crate::store::foundation::Cancellation;
use std::fs;
use std::os::unix::fs::symlink;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tempfile::TempDir;

fn limits() -> DirectoryLimits {
    DirectoryLimits {
        max_entries: 32,
        max_name_bytes: 1024,
    }
}
struct Fixture {
    root: TempDir,
    source: PathBuf,
    store: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let root = TempDir::new().unwrap();
        let source = root.path().join("source");
        let store = root.path().join("store");
        fs::create_dir(&source).unwrap();
        fs::create_dir(&store).unwrap();
        Self {
            root,
            source,
            store,
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

#[test]
fn listing_empty_directory_fits_zero_budgets() {
    let f = Fixture::new();
    let result = f
        .inspect()
        .list_source_directory(
            None,
            DirectoryLimits {
                max_entries: 0,
                max_name_bytes: 0,
            },
            Wait::Try,
        )
        .unwrap();
    assert!(result.is_empty());
    assert_eq!(fs::read_dir(&f.source).unwrap().count(), 0);
    assert_eq!(fs::read_dir(&f.store).unwrap().count(), 0);
}

#[test]
fn listing_keeps_hidden_directory_and_dangling_link_names() {
    let f = Fixture::new();
    fs::write(f.source.join(".hidden"), b"preserved").unwrap();
    fs::create_dir(f.source.join("nested")).unwrap();
    symlink("absent", f.source.join("dangling")).unwrap();
    fs::write(f.source.join("zé"), b"utf8").unwrap();
    assert_eq!(
        f.inspect()
            .list_source_directory(None, limits(), Wait::Try)
            .unwrap(),
        [".hidden", "dangling", "nested", "zé"]
    );
    assert_eq!(fs::read(f.source.join(".hidden")).unwrap(), b"preserved");
    assert_eq!(
        fs::read_link(f.source.join("dangling")).unwrap(),
        Path::new("absent")
    );
}

#[test]
fn listing_nested_directory_uses_no_follow_descendant_acquisition() {
    let f = Fixture::new();
    fs::create_dir(f.source.join("nested")).unwrap();
    fs::write(f.source.join("nested/data"), b"input").unwrap();
    symlink("nested", f.source.join("alias")).unwrap();
    let inspected = f.inspect();
    assert_eq!(
        inspected
            .list_source_directory(Some(Path::new("nested")), limits(), Wait::Try)
            .unwrap(),
        ["data"]
    );
    assert!(inspected
        .list_source_directory(Some(Path::new("alias")), limits(), Wait::Try)
        .is_err());
    assert!(inspected
        .list_source_directory(Some(Path::new("nested/..")), limits(), Wait::Try)
        .is_err());
    assert!(inspected
        .list_source_directory(Some(Path::new("nested/data")), limits(), Wait::Try)
        .is_err());
}

#[test]
fn listing_native_entry_and_utf8_byte_budgets_are_exact() {
    let f = Fixture::new();
    fs::write(f.source.join("é"), b"bytes").unwrap();
    let inspected = f.inspect();
    let exact = DirectoryLimits {
        max_entries: 1,
        max_name_bytes: 2,
    };
    assert_eq!(
        inspected
            .list_source_directory(None, exact, Wait::Try)
            .unwrap(),
        ["é"]
    );
    for bound in [
        DirectoryLimits {
            max_entries: 0,
            max_name_bytes: 2,
        },
        DirectoryLimits {
            max_entries: 1,
            max_name_bytes: 1,
        },
    ] {
        assert_eq!(
            inspected
                .list_source_directory(None, bound, Wait::Try)
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidInput
        );
    }
    assert_eq!(fs::read(f.source.join("é")).unwrap(), b"bytes");
}

#[test]
fn listing_ignores_an_offset_previously_advanced_on_the_held_source() {
    let f = Fixture::new();
    fs::write(f.source.join("data"), b"input").unwrap();
    let inspected = f.inspect();
    let mut shared = Stream::open(inspected.source_handle().try_clone().unwrap()).unwrap();
    while shared.next_name().unwrap().is_some() {}
    shared.close().unwrap();
    for _ in 0..2 {
        assert_eq!(
            inspected
                .list_source_directory(None, limits(), Wait::Try)
                .unwrap(),
            ["data"]
        );
    }
    assert!(inspected.source_handle().metadata().unwrap().is_dir());
}

#[test]
fn listing_revalidates_source_root_after_native_enumeration() {
    let f = Fixture::new();
    fs::write(f.source.join("input"), b"original").unwrap();
    let inspected = f.inspect();
    let mut changed = false;
    let retained = f.root.path().join("retained");
    let result = list_checked(&inspected, None, limits(), Wait::Try, || {
        if !changed {
            fs::rename(&f.source, &retained)?;
            fs::create_dir(&f.source)?;
            changed = true;
        }
        Ok(())
    });
    assert!(result.is_err());
    assert_eq!(fs::read(retained.join("input")).unwrap(), b"original");
    assert_eq!(fs::read_dir(&f.source).unwrap().count(), 0);
}

#[test]
fn listing_revalidates_nested_binding_after_native_enumeration() {
    let f = Fixture::new();
    let target = f.source.join("nested");
    fs::create_dir(&target).unwrap();
    fs::write(target.join("input"), b"original").unwrap();
    let inspected = f.inspect();
    let mut changed = false;
    let result = list_checked(
        &inspected,
        Some(Path::new("nested")),
        limits(),
        Wait::Try,
        || {
            if !changed {
                fs::rename(&target, f.source.join("retained"))?;
                fs::create_dir(&target)?;
                changed = true;
            }
            Ok(())
        },
    );
    assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidData);
    assert_eq!(
        fs::read(f.source.join("retained/input")).unwrap(),
        b"original"
    );
    assert_eq!(fs::read_dir(target).unwrap().count(), 0);
}

#[test]
fn listing_observations_are_fresh_but_do_not_freeze_contents() {
    let f = Fixture::new();
    let inspected = f.inspect();
    assert!(inspected
        .list_source_directory(None, limits(), Wait::Try)
        .unwrap()
        .is_empty());
    fs::write(f.source.join("later"), b"new").unwrap();
    assert_eq!(
        inspected
            .list_source_directory(None, limits(), Wait::Try)
            .unwrap(),
        ["later"]
    );
    assert_eq!(fs::read_dir(&f.store).unwrap().count(), 0);
}

#[test]
fn listing_precancel_and_expired_deadline_are_terminal() {
    let f = Fixture::new();
    let inspected = f.inspect();
    let cancellation = Cancellation::default();
    let expired = Wait::Until {
        deadline: Instant::now() - Duration::from_secs(1),
        cancellation: &cancellation,
    };
    assert_eq!(
        inspected
            .list_source_directory(None, limits(), expired)
            .unwrap_err()
            .kind(),
        io::ErrorKind::TimedOut
    );
    cancellation.cancel();
    let cancelled = Wait::Until {
        deadline: Instant::now() + Duration::from_secs(60),
        cancellation: &cancellation,
    };
    assert_eq!(
        inspected
            .list_source_directory(None, limits(), cancelled)
            .unwrap_err()
            .kind(),
        io::ErrorKind::Interrupted
    );
    assert_eq!(fs::read_dir(&f.source).unwrap().count(), 0);
}

#[test]
fn listing_missing_directory_and_file_source_keep_native_errors() {
    let f = Fixture::new();
    let inspected = f.inspect();
    assert_eq!(
        inspected
            .list_source_directory(Some(Path::new("absent")), limits(), Wait::Try)
            .unwrap_err()
            .raw_os_error(),
        Some(libc::ENOENT)
    );
    fs::write(f.source.join("file"), b"input").unwrap();
    let single = TopologyInspection::inspect(
        &[&f.store],
        SourcePath::File(&f.source.join("file")),
        None,
        Wait::Try,
    )
    .unwrap();
    assert_eq!(
        single
            .list_source_directory(None, limits(), Wait::Try)
            .unwrap_err()
            .raw_os_error(),
        Some(libc::ENOTDIR)
    );
}

#[cfg(target_os = "linux")]
#[test]
fn listing_non_utf8_native_name_is_an_error_not_an_omission() {
    use std::os::unix::ffi::OsStrExt;
    let f = Fixture::new();
    let path = f.source.join(std::ffi::OsStr::from_bytes(b"\xff"));
    fs::write(&path, b"preserved").unwrap();
    assert_eq!(
        f.inspect()
            .list_source_directory(None, limits(), Wait::Try)
            .unwrap_err()
            .kind(),
        io::ErrorKind::InvalidData
    );
    assert_eq!(fs::read(path).unwrap(), b"preserved");
}

#[path = "topology_listing_collect_tests.rs"]
mod collection;
