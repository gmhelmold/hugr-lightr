use super::*;
use crate::store::foundation::scratch_name_probe::NameProbeLimits;
use crate::store::foundation::topology::DestinationInspection;
use crate::store::foundation::tree_plan::{TreeLimits, TreePlan};
use crate::store::foundation::Cancellation;
use crate::Store;
use lightr_core::{Digest, Entry, Manifest};
use std::fs;
use std::io::{self, Cursor, Read, Write};
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

    let file = prepared.test_first_file_mut().unwrap();
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
fn payload_writer_verifies_stream_before_applying_final_mode() {
    let f = Fixture::new();
    let output = f.parent.join("output");
    fs::create_dir(&output).unwrap();
    let observed = f.inspect(&output);
    let anchor = observed.anchor_empty(Wait::Try).unwrap();
    let bytes = b"abc";
    let manifest = Manifest {
        version: 1,
        total_size: bytes.len() as u64,
        entries: vec![Entry::File {
            path: "data".into(),
            mode: 0o754,
            size: bytes.len() as u64,
            digest: Digest::of_bytes(bytes),
        }],
    };
    let mut prepared = anchor
        .prepare_tree(&plan(&manifest), limits(), Wait::Try)
        .unwrap();
    let mut source = Cursor::new(bytes);

    prepared
        .write_file_payload("data", &mut source, Wait::Try)
        .unwrap();
    assert_eq!(fs::read(output.join("data")).unwrap(), bytes);
    assert_eq!(
        fs::metadata(output.join("data"))
            .unwrap()
            .permissions()
            .mode()
            & 0o7777,
        0o754
    );
    let mut second = Cursor::new(bytes);
    assert_eq!(
        prepared
            .write_file_payload("data", &mut second, Wait::Try)
            .unwrap_err()
            .kind(),
        io::ErrorKind::AlreadyExists
    );
    prepared.rollback().unwrap();
}

#[test]
fn payload_writer_rejects_digest_mismatch_without_final_mode() {
    let f = Fixture::new();
    let output = f.parent.join("output");
    fs::create_dir(&output).unwrap();
    let observed = f.inspect(&output);
    let anchor = observed.anchor_empty(Wait::Try).unwrap();
    let manifest = Manifest {
        version: 1,
        total_size: 3,
        entries: vec![Entry::File {
            path: "data".into(),
            mode: 0o754,
            size: 3,
            digest: Digest::of_bytes(b"abc"),
        }],
    };
    let mut prepared = anchor
        .prepare_tree(&plan(&manifest), limits(), Wait::Try)
        .unwrap();
    let mut source = Cursor::new(b"abd");

    assert_eq!(
        prepared
            .write_file_payload("data", &mut source, Wait::Try)
            .unwrap_err()
            .kind(),
        io::ErrorKind::InvalidData
    );
    assert_eq!(
        fs::metadata(output.join("data"))
            .unwrap()
            .permissions()
            .mode()
            & 0o077,
        0
    );
    prepared.rollback().unwrap();
}

#[test]
fn payload_writer_rejects_short_and_long_streams() {
    let f = Fixture::new();
    let output = f.parent.join("output");
    fs::create_dir(&output).unwrap();
    let observed = f.inspect(&output);
    let anchor = observed.anchor_empty(Wait::Try).unwrap();
    let entries = ["short", "long"]
        .into_iter()
        .map(|path| Entry::File {
            path: path.into(),
            mode: 0o754,
            size: 3,
            digest: Digest::of_bytes(b"abc"),
        })
        .collect();
    let manifest = Manifest {
        version: 1,
        total_size: 6,
        entries,
    };
    let mut prepared = anchor
        .prepare_tree(&plan(&manifest), limits(), Wait::Try)
        .unwrap();

    let mut short = Cursor::new(b"ab");
    assert_eq!(
        prepared
            .write_file_payload("short", &mut short, Wait::Try)
            .unwrap_err()
            .kind(),
        io::ErrorKind::InvalidData
    );
    let mut long = Cursor::new(b"abcd");
    assert_eq!(
        prepared
            .write_file_payload("long", &mut long, Wait::Try)
            .unwrap_err()
            .kind(),
        io::ErrorKind::InvalidData
    );
    for path in ["short", "long"] {
        assert_eq!(
            fs::metadata(output.join(path))
                .unwrap()
                .permissions()
                .mode()
                & 0o077,
            0
        );
    }
    prepared.rollback().unwrap();
}

struct CancellingReader<'a> {
    cancellation: &'a Cancellation,
    bytes: &'static [u8],
    read: bool,
}

impl Read for CancellingReader<'_> {
    fn read(&mut self, destination: &mut [u8]) -> io::Result<usize> {
        if self.read {
            return Ok(0);
        }
        self.read = true;
        self.cancellation.cancel();
        destination[..self.bytes.len()].copy_from_slice(self.bytes);
        Ok(self.bytes.len())
    }
}

#[test]
fn payload_writer_cancellation_after_read_cannot_apply_final_mode() {
    let f = Fixture::new();
    let output = f.parent.join("output");
    fs::create_dir(&output).unwrap();
    let observed = f.inspect(&output);
    let anchor = observed.anchor_empty(Wait::Try).unwrap();
    let bytes = b"abc";
    let manifest = Manifest {
        version: 1,
        total_size: 3,
        entries: vec![Entry::File {
            path: "data".into(),
            mode: 0o754,
            size: 3,
            digest: Digest::of_bytes(bytes),
        }],
    };
    let mut prepared = anchor
        .prepare_tree(&plan(&manifest), limits(), Wait::Try)
        .unwrap();
    let cancellation = Cancellation::default();
    let wait = Wait::Until {
        deadline: Instant::now() + Duration::from_secs(1),
        cancellation: &cancellation,
    };
    let mut source = CancellingReader {
        cancellation: &cancellation,
        bytes,
        read: false,
    };

    assert_eq!(
        prepared
            .write_file_payload("data", &mut source, wait)
            .unwrap_err()
            .kind(),
        io::ErrorKind::Interrupted
    );
    assert_eq!(
        fs::metadata(output.join("data"))
            .unwrap()
            .permissions()
            .mode()
            & 0o077,
        0
    );
    prepared.rollback().unwrap();
}

#[test]
fn cas_coordinator_fills_every_file_and_preserves_manifest_modes() {
    let f = Fixture::new();
    let output = f.parent.join("output");
    fs::create_dir(&output).unwrap();
    let store = Store::open(&f.store).unwrap();
    let first = b"first";
    let second = b"second";
    let first_digest = store.put_bytes(first).unwrap();
    let second_digest = store.put_bytes(second).unwrap();
    let manifest = Manifest {
        version: 1,
        total_size: (first.len() + second.len()) as u64,
        entries: vec![
            Entry::File {
                path: "nested/first".into(),
                mode: 0o750,
                size: first.len() as u64,
                digest: first_digest,
            },
            Entry::File {
                path: "second".into(),
                mode: 0o640,
                size: second.len() as u64,
                digest: second_digest,
            },
        ],
    };
    let observed = f.inspect(&output);
    let anchor = observed.anchor_empty(Wait::Try).unwrap();
    let mut prepared = anchor
        .prepare_tree(&plan(&manifest), limits(), Wait::Try)
        .unwrap();

    prepared
        .write_all_payloads_from_store(&store, Wait::Try)
        .unwrap();
    assert_eq!(fs::read(output.join("nested/first")).unwrap(), first);
    assert_eq!(fs::read(output.join("second")).unwrap(), second);
    assert_eq!(mode(&output.join("nested/first")), 0o750);
    assert_eq!(mode(&output.join("second")), 0o640);
    prepared.complete(Wait::Try).unwrap();
    assert_eq!(fs::read(output.join("nested/first")).unwrap(), first);
    assert_eq!(fs::read(output.join("second")).unwrap(), second);
}

#[test]
fn cas_coordinator_missing_object_rolls_back_prior_payloads() {
    let f = Fixture::new();
    let output = f.parent.join("output");
    fs::create_dir(&output).unwrap();
    let store = Store::open(&f.store).unwrap();
    let first = b"first";
    let first_digest = store.put_bytes(first).unwrap();
    let manifest = Manifest {
        version: 1,
        total_size: 6,
        entries: vec![
            Entry::File {
                path: "first".into(),
                mode: 0o750,
                size: first.len() as u64,
                digest: first_digest,
            },
            Entry::File {
                path: "missing".into(),
                mode: 0o640,
                size: 1,
                digest: Digest([9; 32]),
            },
        ],
    };
    let observed = f.inspect(&output);
    let anchor = observed.anchor_empty(Wait::Try).unwrap();
    let mut prepared = anchor
        .prepare_tree(&plan(&manifest), limits(), Wait::Try)
        .unwrap();

    let failure = prepared
        .write_all_payloads_from_store(&store, Wait::Try)
        .unwrap_err();
    assert!(failure.primary.is_some());
    assert!(failure.cleanup.is_empty());
    assert!(failure.cleanup_complete);
    assert_eq!(fs::read_dir(&output).unwrap().count(), 0);
}

#[test]
fn complete_rejects_unwritten_file_and_rolls_back_tree() {
    let f = Fixture::new();
    let output = f.parent.join("output");
    fs::create_dir(&output).unwrap();
    let manifest = manifest(vec![file("data", 3)]);
    let observed = f.inspect(&output);
    let anchor = observed.anchor_empty(Wait::Try).unwrap();
    let prepared = anchor
        .prepare_tree(&plan(&manifest), limits(), Wait::Try)
        .unwrap();

    let failure = prepared.complete(Wait::Try).unwrap_err();
    assert_eq!(failure.primary.unwrap().kind(), io::ErrorKind::InvalidData);
    assert!(failure.cleanup.is_empty());
    assert!(failure.cleanup_complete);
    assert_eq!(fs::read_dir(&output).unwrap().count(), 0);
}

#[test]
fn complete_rejects_final_mode_change_and_rolls_back_tree() {
    let f = Fixture::new();
    let output = f.parent.join("output");
    fs::create_dir(&output).unwrap();
    let manifest = Manifest {
        version: 1,
        total_size: 0,
        entries: vec![Entry::File {
            path: "data".into(),
            mode: 0o640,
            size: 0,
            digest: Digest::of_bytes(&[]),
        }],
    };
    let observed = f.inspect(&output);
    let anchor = observed.anchor_empty(Wait::Try).unwrap();
    let mut prepared = anchor
        .prepare_tree(&plan(&manifest), limits(), Wait::Try)
        .unwrap();
    let mut source = Cursor::new(&[] as &[u8]);
    prepared
        .write_file_payload("data", &mut source, Wait::Try)
        .unwrap();
    fs::set_permissions(output.join("data"), fs::Permissions::from_mode(0o600)).unwrap();

    let failure = prepared.complete(Wait::Try).unwrap_err();
    assert_eq!(failure.primary.unwrap().kind(), io::ErrorKind::InvalidData);
    assert!(failure.cleanup.is_empty());
    assert!(failure.cleanup_complete);
    assert_eq!(fs::read_dir(&output).unwrap().count(), 0);
}

#[test]
fn cas_coordinator_rejects_object_symlink_without_touching_target() {
    let f = Fixture::new();
    let output = f.parent.join("output");
    let external = f.parent.join("external");
    fs::create_dir(&output).unwrap();
    fs::write(&external, b"external").unwrap();
    let store = Store::open(&f.store).unwrap();
    let digest = store.put_bytes(b"payload").unwrap();
    let object = crate::store::cas::object_path(&f.store, &digest);
    let retained = object.with_extension("retained");
    fs::rename(&object, &retained).unwrap();
    std::os::unix::fs::symlink(&external, &object).unwrap();
    let manifest = Manifest {
        version: 1,
        total_size: 7,
        entries: vec![Entry::File {
            path: "data".into(),
            mode: 0o640,
            size: 7,
            digest,
        }],
    };
    let observed = f.inspect(&output);
    let anchor = observed.anchor_empty(Wait::Try).unwrap();
    let mut prepared = anchor
        .prepare_tree(&plan(&manifest), limits(), Wait::Try)
        .unwrap();

    let failure = prepared
        .write_all_payloads_from_store(&store, Wait::Try)
        .unwrap_err();
    assert!(failure.primary.is_some());
    assert_eq!(fs::read(&external).unwrap(), b"external");
    assert_eq!(fs::read_dir(&output).unwrap().count(), 0);
}

#[test]
fn cas_coordinator_rejects_shard_symlink_without_touching_target() {
    let f = Fixture::new();
    let output = f.parent.join("output");
    let external = f.parent.join("external-shard");
    fs::create_dir(&output).unwrap();
    let store = Store::open(&f.store).unwrap();
    let digest = store.put_bytes(b"payload").unwrap();
    let shard = crate::store::cas::object_path(&f.store, &digest)
        .parent()
        .unwrap()
        .to_path_buf();
    fs::rename(&shard, &external).unwrap();
    std::os::unix::fs::symlink(&external, &shard).unwrap();
    let manifest = Manifest {
        version: 1,
        total_size: 7,
        entries: vec![Entry::File {
            path: "data".into(),
            mode: 0o640,
            size: 7,
            digest,
        }],
    };
    let observed = f.inspect(&output);
    let anchor = observed.anchor_empty(Wait::Try).unwrap();
    let mut prepared = anchor
        .prepare_tree(&plan(&manifest), limits(), Wait::Try)
        .unwrap();

    let failure = prepared
        .write_all_payloads_from_store(&store, Wait::Try)
        .unwrap_err();
    assert!(failure.primary.is_some());
    assert_eq!(fs::read_dir(&output).unwrap().count(), 0);
}

fn mode(path: &Path) -> u32 {
    fs::metadata(path).unwrap().permissions().mode() & 0o7777
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
        fs::metadata(output.join("dir"))
            .unwrap()
            .permissions()
            .mode()
            & 0o077,
        0
    );
    assert_eq!(
        fs::metadata(output.join("file"))
            .unwrap()
            .permissions()
            .mode()
            & 0o077,
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
        .err()
        .unwrap();

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
