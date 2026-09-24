use super::*;
use crate::store::foundation::scratch_name_probe::NameProbeLimits;
use crate::store::foundation::topology::DestinationInspection;
use crate::store::foundation::tree_plan::{TreeLimits, TreePlan};
use crate::store::foundation::Cancellation;
use lightr_core::{Digest, Entry, Manifest};
use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tempfile::TempDir;

struct Fixture {
    _root: TempDir,
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
            _root: root,
            store,
            parent,
        }
    }

    fn inspect(&self, destination: &Path) -> DestinationInspection {
        DestinationInspection::inspect(&[&self.store], destination, Wait::Try).unwrap()
    }
}

fn manifest(entries: Vec<Entry>) -> Manifest {
    Manifest {
        version: 1,
        total_size: 0,
        entries,
    }
}

fn file(path: &str) -> Entry {
    Entry::File {
        path: path.into(),
        mode: 0o644,
        size: 0,
        digest: Digest([0; 32]),
    }
}

fn directory(path: &str) -> Entry {
    Entry::Dir { path: path.into() }
}

fn link(path: &str, target: &str) -> Entry {
    Entry::Symlink {
        path: path.into(),
        target: target.into(),
    }
}

fn plan(manifest: &Manifest) -> TreePlan<'_> {
    TreePlan::validate(
        manifest,
        TreeLimits {
            max_entries: 8192,
            max_components: 100_000,
            max_text_bytes: 1_000_000,
        },
        Wait::Try,
    )
    .unwrap()
}

fn limits() -> NameProbeLimits {
    NameProbeLimits {
        max_nodes: 256,
        max_depth: 32,
    }
}

#[test]
fn destination_name_probe_empty_tree_is_noop() {
    let f = Fixture::new();
    let output = f.parent.join("output");
    fs::create_dir(&output).unwrap();
    let observed = f.inspect(&output);
    let anchor = observed.anchor_empty(Wait::Try).unwrap();
    let manifest = manifest(vec![]);

    let result = anchor
        .probe_tree_names(
            &plan(&manifest),
            NameProbeLimits {
                max_nodes: 0,
                max_depth: 0,
            },
            Wait::Try,
        )
        .unwrap();

    assert_eq!(
        result,
        DestinationNameObservation {
            directories: 0,
            leaf_names: 0
        }
    );
    assert_eq!(fs::read_dir(&output).unwrap().count(), 0);
    anchor.revalidate(Wait::Try).unwrap();
}

#[test]
fn destination_name_probe_whole_tree_uses_actual_root_and_cleans() {
    let f = Fixture::new();
    let output = f.parent.join("output");
    fs::create_dir(&output).unwrap();
    let observed = f.inspect(&output);
    let anchor = observed.anchor_empty(Wait::Try).unwrap();
    let manifest = manifest(vec![
        file("nested/deeper/data"),
        directory("empty"),
        file(".hidden"),
        link("dangling", "../external/unchanged"),
    ]);
    let expected_directories = ["nested", "nested/deeper", "empty"];
    let mut seen = BTreeSet::new();

    let result = probe_checked(
        &anchor,
        &plan(&manifest),
        limits(),
        Wait::Try,
        |step, path, _| {
            if step == ProbeStep::Created {
                assert!(seen.insert(path.to_string()));
                let entry = output.join(path);
                if expected_directories.contains(&path) {
                    assert!(entry.is_dir());
                } else {
                    assert!(fs::symlink_metadata(entry)?.is_file());
                }
            }
            Ok(())
        },
    )
    .unwrap();

    assert_eq!(
        result,
        DestinationNameObservation {
            directories: 3,
            leaf_names: 3
        }
    );
    assert_eq!(seen.len(), 6);
    assert_eq!(fs::read_dir(&output).unwrap().count(), 0);
    assert_eq!(fs::read(f.store.join("sentinel")).unwrap(), b"store");
}

#[test]
fn destination_name_probe_link_entries_are_name_markers_not_links() {
    let f = Fixture::new();
    let output = f.parent.join("output");
    fs::create_dir(&output).unwrap();
    let observed = f.inspect(&output);
    let anchor = observed.anchor_empty(Wait::Try).unwrap();
    let manifest = manifest(vec![link("link-name", "../must-not-be-followed")]);

    probe_checked(
        &anchor,
        &plan(&manifest),
        limits(),
        Wait::Try,
        |step, path, _| {
            if step == ProbeStep::Created && path == "link-name" {
                let metadata = fs::symlink_metadata(output.join(path))?;
                assert!(metadata.is_file());
                assert!(!metadata.file_type().is_symlink());
                assert!(!f.parent.join("must-not-be-followed").exists());
            }
            Ok(())
        },
    )
    .unwrap();

    assert_eq!(fs::read_dir(&output).unwrap().count(), 0);
}

#[test]
fn destination_name_probe_native_case_and_unicode_match_same_directory_oracle() {
    for (first, second) in [("A", "a"), ("é", "e\u{301}"), ("alpha", "alphabeta")] {
        let f = Fixture::new();
        let output = f.parent.join("output");
        fs::create_dir(&output).unwrap();

        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(output.join(first))
            .unwrap();
        let oracle = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(output.join(second));
        let oracle_error = oracle.as_ref().err().and_then(io::Error::raw_os_error);
        if oracle.is_ok() {
            fs::remove_file(output.join(second)).unwrap();
        }
        fs::remove_file(output.join(first)).unwrap();
        assert_eq!(fs::read_dir(&output).unwrap().count(), 0);

        let observed = f.inspect(&output);
        let anchor = observed.anchor_empty(Wait::Try).unwrap();
        let manifest = manifest(vec![file(first), file(second)]);
        let actual = anchor.probe_tree_names(&plan(&manifest), limits(), Wait::Try);

        match oracle_error {
            None => assert_eq!(actual.unwrap().leaf_names, 2),
            Some(code) => {
                let failure = actual.unwrap_err();
                assert_eq!(failure.primary.unwrap().raw_os_error(), Some(code));
                assert!(failure.cleanup.is_empty());
                assert!(failure.cleanup_complete);
            }
        }
        assert_eq!(fs::read_dir(&output).unwrap().count(), 0);
    }
}

#[test]
fn destination_name_probe_budgets_precede_mutation() {
    let f = Fixture::new();
    let output = f.parent.join("output");
    fs::create_dir(&output).unwrap();
    let observed = f.inspect(&output);
    let anchor = observed.anchor_empty(Wait::Try).unwrap();

    for (manifest, bounds) in [
        (
            manifest(vec![file("a/b")]),
            NameProbeLimits {
                max_nodes: 1,
                max_depth: 32,
            },
        ),
        (
            manifest(vec![file("a/b")]),
            NameProbeLimits {
                max_nodes: 2,
                max_depth: 1,
            },
        ),
    ] {
        let failure = anchor
            .probe_tree_names(&plan(&manifest), bounds, Wait::Try)
            .unwrap_err();
        assert_eq!(failure.primary.unwrap().kind(), io::ErrorKind::InvalidInput);
        assert!(failure.cleanup.is_empty());
        assert!(failure.cleanup_complete);
        assert_eq!(fs::read_dir(&output).unwrap().count(), 0);
    }
}

#[test]
fn destination_name_probe_late_native_error_cleans_prior_names() {
    let f = Fixture::new();
    let output = f.parent.join("output");
    fs::create_dir(&output).unwrap();
    let observed = f.inspect(&output);
    let anchor = observed.anchor_empty(Wait::Try).unwrap();
    let manifest = manifest(vec![file("a"), file(&"z".repeat(512))]);

    let failure = anchor
        .probe_tree_names(&plan(&manifest), limits(), Wait::Try)
        .unwrap_err();

    assert_eq!(
        failure.primary.unwrap().raw_os_error(),
        Some(libc::ENAMETOOLONG)
    );
    assert!(failure.cleanup.is_empty());
    assert!(failure.cleanup_complete);
    assert_eq!(fs::read_dir(&output).unwrap().count(), 0);
}

#[test]
fn destination_name_probe_created_handles_are_close_on_exec() {
    let f = Fixture::new();
    let output = f.parent.join("output");
    fs::create_dir(&output).unwrap();
    let observed = f.inspect(&output);
    let anchor = observed.anchor_empty(Wait::Try).unwrap();
    let manifest = manifest(vec![directory("dir"), file("file")]);
    let mut checked = 0;

    probe_checked(
        &anchor,
        &plan(&manifest),
        limits(),
        Wait::Try,
        |step, _, handle| {
            if step == ProbeStep::Created {
                let fd = handle.unwrap().as_raw_fd();
                let flags = unsafe { libc::fcntl(fd, libc::F_GETFD) };
                assert!(flags >= 0);
                assert_ne!(flags & libc::FD_CLOEXEC, 0);
                checked += 1;
            }
            Ok(())
        },
    )
    .unwrap();

    assert_eq!(checked, 2);
    assert_eq!(fs::read_dir(&output).unwrap().count(), 0);
}

#[test]
fn destination_name_probe_created_anchor_root_can_rollback_after_success() {
    let f = Fixture::new();
    let output = f.parent.join("missing/output");
    let observed = f.inspect(&output);
    let anchor = observed.anchor_empty(Wait::Try).unwrap();
    assert!(anchor.was_created());
    let manifest = manifest(vec![file("a"), directory("empty")]);

    anchor
        .probe_tree_names(&plan(&manifest), limits(), Wait::Try)
        .unwrap();
    assert_eq!(fs::read_dir(&output).unwrap().count(), 0);
    anchor.rollback_empty().unwrap();

    assert!(!f.parent.join("missing").exists());
}

#[path = "destination_name_probe_race_tests.rs"]
mod races;
