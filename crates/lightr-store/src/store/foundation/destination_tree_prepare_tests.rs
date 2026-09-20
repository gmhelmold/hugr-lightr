use super::*;
use crate::store::foundation::scratch_name_probe::NameProbeLimits;
use crate::store::foundation::topology::DestinationInspection;
use crate::store::foundation::tree_plan::{TreeLimits, TreePlan};
use crate::store::foundation::Cancellation;
use lightr_core::{Digest, Entry, Manifest};
use std::fs;
use std::io::{self, Write};
use std::os::fd::AsRawFd;
use std::os::unix::fs::PermissionsExt;
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
        total_size: entries
            .iter()
            .map(|entry| match entry {
                Entry::File { size, .. } => *size,
                _ => 0,
            })
            .sum(),
        entries,
    }
}

fn file(path: &str, size: u64) -> Entry {
    Entry::File {
        path: path.into(),
        mode: 0o640,
        size,
        digest: Digest([7; 32]),
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
fn prepared_tree_empty_is_noop_and_rolls_back_cleanly() {
    let f = Fixture::new();
    let output = f.parent.join("output");
    fs::create_dir(&output).unwrap();
    let observed = f.inspect(&output);
    let anchor = observed.anchor_empty(Wait::Try).unwrap();
    let manifest = manifest(vec![]);
    let prepared = anchor
        .prepare_tree(&plan(&manifest), limits(), Wait::Try)
        .unwrap();

    assert_eq!(prepared.directory_count(), 0);
    assert_eq!(prepared.file_count(), 0);
    assert_eq!(prepared.link_count(), 0);
    prepared.revalidate(Wait::Try).unwrap();
    prepared.rollback().unwrap();
    assert_eq!(fs::read_dir(&output).unwrap().count(), 0);
}

#[test]
fn prepared_tree_creates_exact_namespace_and_explicit_rollback_cleans_it() {
    let f = Fixture::new();
    let output = f.parent.join("output");
    fs::create_dir(&output).unwrap();
    let observed = f.inspect(&output);
    let anchor = observed.anchor_empty(Wait::Try).unwrap();
    let manifest = manifest(vec![
        file("nested/data", 0),
        directory("empty"),
        link("dangling", "../outside/unchanged"),
    ]);
    let prepared = anchor
        .prepare_tree(&plan(&manifest), limits(), Wait::Try)
        .unwrap();

    assert_eq!(prepared.directory_count(), 2);
    assert_eq!(prepared.file_count(), 1);
    assert_eq!(prepared.link_count(), 1);
    assert!(output.join("nested").is_dir());
    assert!(output.join("empty").is_dir());
    assert!(fs::symlink_metadata(output.join("nested/data"))
        .unwrap()
        .is_file());
    assert_eq!(
        fs::read_link(output.join("dangling")).unwrap(),
        Path::new("../outside/unchanged")
    );
    assert!(!f.parent.join("outside").exists());
    prepared.revalidate(Wait::Try).unwrap();
    prepared.rollback().unwrap();
    assert_eq!(fs::read_dir(&output).unwrap().count(), 0);
    assert_eq!(fs::read(f.store.join("sentinel")).unwrap(), b"store");
}

#[test]
fn prepared_file_handle_is_cloexec_and_ready_for_future_payload_writer() {
    let f = Fixture::new();
    let output = f.parent.join("output");
    fs::create_dir(&output).unwrap();
    let observed = f.inspect(&output);
    let anchor = observed.anchor_empty(Wait::Try).unwrap();
    let manifest = manifest(vec![file("data", 3)]);
    let mut prepared = anchor
        .prepare_tree(&plan(&manifest), limits(), Wait::Try)
        .unwrap();

    let file = prepared.file_mut(0).unwrap();
    let flags = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFD) };
    assert!(flags >= 0);
    assert_ne!(flags & libc::FD_CLOEXEC, 0);
    file.write_all(b"abc").unwrap();
    file.sync_data().unwrap();
    assert_eq!(fs::read(output.join("data")).unwrap(), b"abc");

    prepared.revalidate(Wait::Try).unwrap();
    prepared.rollback().unwrap();
    assert_eq!(fs::read_dir(&output).unwrap().count(), 0);
}

#[test]
fn prepared_entries_use_private_intermediate_modes() {
    let f = Fixture::new();
    let output = f.parent.join("output");
    fs::create_dir(&output).unwrap();
    let observed = f.inspect(&output);
    let anchor = observed.anchor_empty(Wait::Try).unwrap();
    let manifest = manifest(vec![directory("dir"), file("file", 0)]);
    let prepared = anchor
        .prepare_tree(&plan(&manifest), limits(), Wait::Try)
        .unwrap();

    assert_eq!(
        fs::metadata(output.join("dir")).unwrap().permissions().mode() & 0o077,
        0
    );
    assert_eq!(
        fs::metadata(output.join("file")).unwrap().permissions().mode() & 0o077,
        0
    );
    prepared.rollback().unwrap();
}

#[test]
fn prepared_tree_representation_probe_finishes_before_persistent_creation() {
    let f = Fixture::new();
    let output = f.parent.join("output");
    fs::create_dir(&output).unwrap();
    let observed = f.inspect(&output);
    let anchor = observed.anchor_empty(Wait::Try).unwrap();
    let manifest = manifest(vec![file("a", 0), directory("dir")]);

    let prepared = prepare_checked(
        &anchor,
        &plan(&manifest),
        limits(),
        Wait::Try,
        |step, _, _| {
            if step == PrepareStep::AfterRepresentation {
                assert_eq!(fs::read_dir(&output)?.count(), 0);
            }
            Ok(())
        },
    )
    .unwrap();

    assert!(output.join("a").is_file());
    assert!(output.join("dir").is_dir());
    prepared.rollback().unwrap();
}

#[test]
fn prepared_tree_late_native_failure_cleans_prior_entries() {
    let f = Fixture::new();
    let output = f.parent.join("output");
    fs::create_dir(&output).unwrap();
    let observed = f.inspect(&output);
    let anchor = observed.anchor_empty(Wait::Try).unwrap();
    let manifest = manifest(vec![file("a", 0), file(&"z".repeat(512), 0)]);

    let failure = anchor
        .prepare_tree(&plan(&manifest), limits(), Wait::Try)
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
fn prepared_tree_created_anchor_root_remains_for_anchor_rollback() {
    let f = Fixture::new();
    let output = f.parent.join("missing/output");
    let observed = f.inspect(&output);
    let anchor = observed.anchor_empty(Wait::Try).unwrap();
    let manifest = manifest(vec![file("a", 0)]);

    let prepared = anchor
        .prepare_tree(&plan(&manifest), limits(), Wait::Try)
        .unwrap();
    prepared.rollback().unwrap();
    assert_eq!(fs::read_dir(&output).unwrap().count(), 0);
    anchor.rollback_empty().unwrap();
    assert!(!f.parent.join("missing").exists());
}

#[path = "destination_tree_prepare_race_tests.rs"]
mod races;
