//! Disposable fixtures; no live Store, process-global hooks or user-data writes.
use super::*;
use crate::store::foundation::Cancellation;
use std::fs;
use std::os::unix::fs::symlink;
use std::path::PathBuf;
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
        fs::create_dir(&store).unwrap();
        fs::create_dir_all(source.join("nested")).unwrap();
        fs::write(store.join("sentinel"), b"protected").unwrap();
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
    fn link(&self, name: &str, target: &str) {
        symlink(target, self.source.join(name)).unwrap();
    }
}
fn read(inspection: &TopologyInspection, path: &str) -> io::Result<String> {
    inspection.read_source_link(Path::new(path), 65535, Wait::Try)
}

#[test]
fn source_link_preserves_dangling_and_exact_target_text() {
    let f = Fixture::new();
    let inspection = f.inspect();
    for (index, target) in [
        "missing",
        "../missing/../é",
        "./a//b",
        r"C:\raw\target",
        "/no/such/target",
    ]
    .iter()
    .enumerate()
    {
        let name = format!("nested/link{index}");
        f.link(&name, target);
        assert_eq!(
            read(&inspection, &name).expect("dangling link text not preserved"),
            *target
        );
        assert_eq!(
            fs::read_link(f.source.join(&name)).unwrap(),
            Path::new(target)
        );
    }
    assert_eq!(fs::read(f.store.join("sentinel")).unwrap(), b"protected");
}

#[test]
fn source_link_reads_file_directory_and_protected_target_without_following() {
    let f = Fixture::new();
    fs::write(f.source.join("data"), b"not link text").unwrap();
    f.link("file-link", "data");
    f.link("dir-link", "nested");
    symlink(&f.store, f.source.join("store-link")).unwrap();
    let inspection = f.inspect();
    assert_eq!(read(&inspection, "file-link").unwrap(), "data");
    assert_eq!(read(&inspection, "dir-link").unwrap(), "nested");
    assert_eq!(
        read(&inspection, "store-link").unwrap(),
        f.store.to_str().unwrap()
    );
    assert_eq!(fs::read(f.source.join("data")).unwrap(), b"not link text");
    assert_eq!(fs::read_dir(&f.store).unwrap().count(), 1);
}

#[test]
fn source_link_rejects_intermediate_aliases_and_cycles() {
    let f = Fixture::new();
    f.link("nested/leaf", "absent");
    f.link("alias", "nested");
    f.link("cycle", "cycle");
    let inspection = f.inspect();
    for path in ["alias/leaf", "cycle/leaf"] {
        assert!(
            read(&inspection, path).is_err(),
            "intermediate link followed"
        );
    }
    assert_eq!(read(&inspection, "cycle").unwrap(), "cycle");
}

#[test]
fn source_link_rejects_non_normal_paths_and_work_budgets() {
    let f = Fixture::new();
    let inspection = f.inspect();
    for path in [
        "",
        ".",
        "..",
        "/leaf",
        "nested/../leaf",
        "nested/./leaf",
        "nested//leaf",
        "leaf/",
        "leaf\0",
    ] {
        assert_eq!(
            read(&inspection, path).unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
    }
    for path in ["x".repeat(4097), vec!["x"; 129].join("/")] {
        assert_eq!(
            read(&inspection, &path).unwrap_err().kind(),
            io::ErrorKind::Unsupported
        );
    }
    for budget in [0, 65536, usize::MAX] {
        assert_eq!(
            inspection
                .read_source_link(Path::new("missing"), budget, Wait::Try)
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidInput
        );
    }
}

#[test]
fn source_link_exact_byte_budget_never_accepts_truncation() {
    let f = Fixture::new();
    f.link("leaf", "éé");
    let inspection = f.inspect();
    assert_eq!(
        inspection
            .read_source_link(Path::new("leaf"), 4, Wait::Try)
            .unwrap(),
        "éé"
    );
    for budget in [1, 2, 3] {
        assert_eq!(
            inspection
                .read_source_link(Path::new("leaf"), budget, Wait::Try)
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidInput
        );
    }
    assert_eq!(
        fs::read_link(f.source.join("leaf")).unwrap(),
        Path::new("éé")
    );
}

#[test]
fn source_link_preserves_missing_and_wrong_type_errors() {
    let f = Fixture::new();
    fs::write(f.source.join("regular"), b"retained").unwrap();
    let inspection = f.inspect();
    assert_eq!(
        read(&inspection, "missing").unwrap_err().raw_os_error(),
        Some(libc::ENOENT)
    );
    for path in ["regular", "nested"] {
        assert_eq!(
            read(&inspection, path).unwrap_err().raw_os_error(),
            Some(libc::EINVAL)
        );
    }
    assert_eq!(fs::read(f.source.join("regular")).unwrap(), b"retained");
    let file_root = TopologyInspection::inspect(
        &[&f.store],
        SourcePath::File(&f.source.join("regular")),
        None,
        Wait::Try,
    )
    .unwrap();
    assert_eq!(
        read(&file_root, "leaf").unwrap_err().kind(),
        io::ErrorKind::InvalidInput
    );
}

#[test]
fn source_link_detects_replacement_after_read() {
    let f = Fixture::new();
    f.link("nested/leaf", "before");
    let inspection = f.inspect();
    let result = read_checked(
        &inspection,
        Path::new("nested/leaf"),
        100,
        Wait::Try,
        |stage| {
            if stage == Stage::Read {
                fs::rename(
                    f.source.join("nested/leaf"),
                    f.source.join("nested/retained"),
                )?;
                f.link("nested/leaf", "after");
            }
            Ok(())
        },
    );
    assert_eq!(
        result.expect_err("replaced source link accepted").kind(),
        io::ErrorKind::InvalidData
    );
    assert_eq!(read(&inspection, "nested/leaf").unwrap(), "after");
    assert_eq!(
        fs::read_link(f.source.join("nested/retained")).unwrap(),
        Path::new("before")
    );
}

#[test]
fn source_link_detects_replaced_parent_without_retargeting() {
    let f = Fixture::new();
    f.link("nested/leaf", "same-text");
    let inspection = f.inspect();
    let result = read_checked(
        &inspection,
        Path::new("nested/leaf"),
        100,
        Wait::Try,
        |stage| {
            if stage == Stage::Read {
                fs::rename(f.source.join("nested"), f.source.join("old-nested"))?;
                fs::create_dir(f.source.join("nested"))?;
                f.link("nested/leaf", "same-text");
            }
            Ok(())
        },
    );
    assert_eq!(
        result
            .expect_err("replaced source-link parent accepted")
            .kind(),
        io::ErrorKind::InvalidData
    );
    assert_eq!(
        fs::read_link(f.source.join("old-nested/leaf")).unwrap(),
        Path::new("same-text")
    );
}

#[test]
fn source_link_detects_original_source_replacement() {
    let f = Fixture::new();
    f.link("leaf", "before");
    let inspection = f.inspect();
    fs::rename(&f.source, f._root.path().join("old-source")).unwrap();
    fs::create_dir(&f.source).unwrap();
    f.link("leaf", "after");
    assert!(read(&inspection, "leaf").is_err());
    assert_eq!(
        fs::read_link(f._root.path().join("old-source/leaf")).unwrap(),
        Path::new("before")
    );
}

#[test]
fn source_link_returned_text_does_not_depend_on_later_target_existence() {
    let f = Fixture::new();
    f.link("leaf", "missing");
    let inspection = f.inspect();
    let target = read(&inspection, "leaf").unwrap();
    fs::remove_file(f.source.join("leaf")).unwrap();
    assert_eq!(target, "missing");
    assert_eq!(
        read(&inspection, "leaf").unwrap_err().raw_os_error(),
        Some(libc::ENOENT)
    );
}

#[test]
fn source_link_cancellation_and_deadline_precede_access() {
    let f = Fixture::new();
    f.link("leaf", "missing");
    let inspection = f.inspect();
    let cancellation = Cancellation::default();
    let expired = Wait::Until {
        deadline: Instant::now() - Duration::from_secs(1),
        cancellation: &cancellation,
    };
    assert_eq!(
        inspection
            .read_source_link(Path::new("leaf"), 10, expired)
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
        inspection
            .read_source_link(Path::new("missing"), 10, cancelled)
            .unwrap_err()
            .kind(),
        io::ErrorKind::Interrupted
    );
    assert_eq!(
        fs::read_link(f.source.join("leaf")).unwrap(),
        Path::new("missing")
    );
}

#[test]
fn source_link_cancellation_after_completed_validation_is_not_success() {
    let f = Fixture::new();
    f.link("leaf", "missing");
    let inspection = f.inspect();
    let cancellation = Cancellation::default();
    let wait = Wait::Until {
        deadline: Instant::now() + Duration::from_secs(60),
        cancellation: &cancellation,
    };
    let result = read_checked(&inspection, Path::new("leaf"), 20, wait, |stage| {
        if stage == Stage::Validated {
            cancellation.cancel();
        }
        Ok(())
    });
    assert_eq!(
        result
            .expect_err("completed cancelled link read accepted")
            .kind(),
        io::ErrorKind::Interrupted
    );
}

#[test]
fn source_link_scoped_errors_preserve_original_cause() {
    let f = Fixture::new();
    f.link("leaf", "missing");
    let inspection = f.inspect();
    for expected_stage in [Stage::Pinned, Stage::Read, Stage::Validated] {
        let result = read_checked(&inspection, Path::new("leaf"), 20, Wait::Try, |stage| {
            if stage == expected_stage {
                Err(io::Error::from_raw_os_error(libc::EIO))
            } else {
                Ok(())
            }
        });
        assert_eq!(result.unwrap_err().raw_os_error(), Some(libc::EIO));
        assert_eq!(read(&inspection, "leaf").unwrap(), "missing");
    }
}

#[test]
fn source_link_pinned_handle_is_symlink_and_close_on_exec() {
    let f = Fixture::new();
    f.link("leaf", "absent");
    let inspection = f.inspect();
    let pinned = pin(inspection.source_handle(), c"leaf", || Ok(())).unwrap();
    assert!(pinned.metadata().unwrap().file_type().is_symlink());
    // SAFETY: queries only the locally owned live descriptor.
    let flags = unsafe { libc::fcntl(pinned.as_raw_fd(), libc::F_GETFD) };
    assert!(flags >= 0);
    assert_ne!(flags & libc::FD_CLOEXEC, 0);
}

#[cfg(target_os = "linux")]
#[path = "topology_link_linux_tests.rs"]
mod linux;
