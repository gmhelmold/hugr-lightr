//! Deterministic regressions at the scan -> ingest -> publish boundary.
//! Captured manifests model scan output, so source changes need no timing race
//! or process-global hook. Existing roundtrip tests exercise public snapshot().

use super::publish_snapshot;
use crate::index::hydrate::hydrate;
use lightr_core::{Digest, Entry, LightrError, Manifest};
use lightr_store::Store;
use std::fs;
use tempfile::TempDir;

fn captured(files: &[(&str, &[u8])]) -> Manifest {
    let mut entries: Vec<Entry> = files
        .iter()
        .map(|(path, bytes)| Entry::File {
            path: (*path).to_string(),
            mode: 0o644,
            size: bytes.len() as u64,
            digest: Digest::of_bytes(bytes),
        })
        .collect();
    entries.sort_by(|a, b| a.path().cmp(b.path()));
    Manifest {
        version: 1,
        total_size: files.iter().map(|(_, bytes)| bytes.len() as u64).sum(),
        entries,
    }
}

fn assert_unpublished(store: &Store, name: &str, manifest: &Manifest) {
    assert!(store.ref_get(name).unwrap().is_none());
    assert!(store.ref_log(name).unwrap().is_empty());
    assert!(!store.exists(&manifest.digest()));
}

fn setup() -> (TempDir, Store, std::path::PathBuf) {
    let temp = TempDir::new().unwrap();
    let store = Store::open(temp.path().join("store")).unwrap();
    let source = temp.path().join("source");
    fs::create_dir(&source).unwrap();
    (temp, store, source)
}

#[test]
fn removed_source_does_not_publish_a_snapshot() {
    let (_temp, store, source) = setup();
    fs::write(source.join("data"), b"before").unwrap();
    let manifest = captured(&[("data", b"before")]);
    fs::remove_file(source.join("data")).unwrap();

    let result = publish_snapshot(&source, &store, "@test/removed", manifest.clone());
    assert!(matches!(result, Err(LightrError::Io(_))));
    assert_unpublished(&store, "@test/removed", &manifest);
}

#[test]
fn changed_source_is_rejected_even_when_its_size_is_unchanged() {
    let (_temp, store, source) = setup();
    fs::write(source.join("data"), b"before").unwrap();
    let manifest = captured(&[("data", b"before")]);
    fs::write(source.join("data"), b"after!").unwrap();

    let result = publish_snapshot(&source, &store, "@test/changed", manifest.clone());
    match result {
        Err(LightrError::Integrity { expected, actual }) => {
            assert_eq!(expected, Digest::of_bytes(b"before"));
            assert_eq!(actual, Digest::of_bytes(b"after!"));
        }
        _ => panic!("changed input must fail with a digest mismatch"),
    }
    assert_unpublished(&store, "@test/changed", &manifest);
}

#[test]
fn ingestion_failure_does_not_advance_existing_ref_or_history() {
    let (_temp, store, source) = setup();
    fs::write(source.join("data"), b"old").unwrap();
    let old = captured(&[("data", b"old")]);
    let name = "@test/keep";
    publish_snapshot(&source, &store, name, old).unwrap();
    let before = store.ref_get(name).unwrap().unwrap();
    let history_len = store.ref_log(name).unwrap().len();

    let next = captured(&[("missing", b"new")]);
    assert!(publish_snapshot(&source, &store, name, next.clone()).is_err());
    let after = store.ref_get(name).unwrap().unwrap();
    assert_eq!(after.root, before.root);
    assert_eq!(after.parent, before.parent);
    assert_eq!(store.ref_log(name).unwrap().len(), history_len);
    assert!(!store.exists(&next.digest()));

    // Failed publication must not prevent recovery of the previous snapshot.
    let restored = source.parent().unwrap().join("restored-old");
    hydrate(&restored, &store, name).unwrap();
    assert_eq!(fs::read(restored.join("data")).unwrap(), b"old");
}

#[test]
fn one_failed_parallel_ingestion_prevents_partial_publication() {
    let (_temp, store, source) = setup();
    fs::write(source.join("good"), b"preserved").unwrap();
    let manifest = captured(&[("good", b"preserved"), ("missing", b"unavailable")]);

    assert!(publish_snapshot(&source, &store, "@test/partial", manifest.clone()).is_err());
    // Rayon may ingest the good file before observing the failure. Its presence
    // is immaterial: there must be no ref or manifest claiming a complete tree.
    assert_unpublished(&store, "@test/partial", &manifest);
}

#[test]
fn retry_with_a_fresh_manifest_restores_the_new_bytes() {
    let (_temp, store, source) = setup();
    let name = "@test/retry";
    let stale = captured(&[("data", b"before")]);
    fs::write(source.join("data"), b"after!").unwrap();
    assert!(publish_snapshot(&source, &store, name, stale).is_err());

    let fresh = captured(&[("data", b"after!")]);
    let report = publish_snapshot(&source, &store, name, fresh.clone()).unwrap();
    assert_eq!(report.root, fresh.digest());
    assert_eq!(report.files, 1);
    assert_eq!(report.bytes_total, 6);
    let restored = source.parent().unwrap().join("restored-new");
    hydrate(&restored, &store, name).unwrap();
    assert_eq!(fs::read(restored.join("data")).unwrap(), b"after!");
}

#[test]
fn already_preserved_content_can_be_reused_without_the_live_source() {
    let (_temp, store, source) = setup();
    let bytes = b"already stored";
    let digest = store.put_bytes(bytes).unwrap();
    let manifest = captured(&[("data", bytes)]);
    assert!(!source.join("data").exists());

    let report = publish_snapshot(&source, &store, "@test/reuse", manifest.clone()).unwrap();
    assert_eq!(report.root, manifest.digest());
    assert_eq!(report.objects_new, 0);
    assert_eq!(store.get_bytes(&digest).unwrap(), bytes);
}

#[test]
fn publication_failure_releases_the_gc_lock() {
    let (_temp, store, source) = setup();
    let manifest = captured(&[("missing", b"data")]);
    assert!(publish_snapshot(&source, &store, "@test/unlock", manifest).is_err());

    // Bound the wait so a leaked publication guard fails instead of hanging CI.
    let (tx, rx) = std::sync::mpsc::channel();
    let checker = std::thread::spawn(move || {
        let _guard = store.gc_guard().unwrap();
        let _ = tx.send(());
    });
    assert!(rx.recv_timeout(std::time::Duration::from_secs(5)).is_ok());
    checker.join().unwrap();
}
