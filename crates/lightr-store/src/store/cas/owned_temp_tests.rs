use super::OwnedTemp;
use crate::store::cas::atomic_write;
use crate::Store;
use std::fs;
use std::io;
use std::sync::{Arc, Barrier};
use tempfile::TempDir;

#[test]
fn forced_collision_never_claims_or_cleans_another_allocation() {
    let root = TempDir::new().unwrap();
    let first = OwnedTemp::reserve(root.path(), || ".tmp-fixed".into()).unwrap();
    fs::write(first.payload(), b"first-owner").unwrap();
    let mut attempts = 0;
    let second = OwnedTemp::reserve(root.path(), || {
        attempts += 1;
        ".tmp-fixed".into()
    });
    assert!(matches!(second, Err(e) if e.kind() == io::ErrorKind::AlreadyExists));
    assert_eq!(attempts, 32);
    assert_eq!(fs::read(first.payload()).unwrap(), b"first-owner");
    drop(first);
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
}

#[test]
fn normal_allocations_are_distinct_and_payloads_start_absent() {
    let root = TempDir::new().unwrap();
    let mut allocations = Vec::new();
    for _ in 0..64 {
        let allocation = OwnedTemp::new(root.path(), "same").unwrap();
        assert!(!allocation.payload().exists());
        allocations.push(allocation);
    }
    let paths: std::collections::HashSet<_> = allocations.iter().map(OwnedTemp::payload).collect();
    assert_eq!(paths.len(), 64);
    drop(allocations);
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
}

#[test]
fn cleanup_does_not_recursively_remove_unrecognized_children() {
    let root = TempDir::new().unwrap();
    let staging = OwnedTemp::new(root.path(), "cleanup").unwrap();
    let other = staging.dir.join("unexpected");
    fs::write(staging.payload(), b"owned").unwrap();
    fs::write(&other, b"not-the-payload").unwrap();
    let payload = staging.payload();
    drop(staging);
    assert!(!payload.exists());
    assert_eq!(fs::read(other).unwrap(), b"not-the-payload");
}

#[test]
fn public_parallel_ingestion_of_identical_bytes_keeps_every_result_valid() {
    let root = TempDir::new().unwrap();
    let store = Arc::new(Store::open(root.path().join("store")).unwrap());
    let source = root.path().join("source");
    let bytes = vec![b'x'; 16384];
    fs::write(&source, &bytes).unwrap();
    let barrier = Arc::new(Barrier::new(16));
    let mut workers = Vec::new();
    for _ in 0..16 {
        let (barrier, source, store) = (barrier.clone(), source.clone(), store.clone());
        workers.push(std::thread::spawn(move || {
            barrier.wait();
            store.ingest_file(&source)
        }));
    }
    for worker in workers {
        let digest = worker.join().unwrap().unwrap();
        assert_eq!(store.get_bytes(&digest).unwrap(), bytes);
    }
    assert_eq!(fs::read(source).unwrap(), bytes);
    for shard in fs::read_dir(store.root().join("objects")).unwrap() {
        for entry in fs::read_dir(shard.unwrap().path()).unwrap() {
            assert!(!entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".tmp-"));
        }
    }
}

#[test]
fn concurrent_atomic_metadata_writes_own_different_staging() {
    let root = TempDir::new().unwrap();
    let barrier = Arc::new(Barrier::new(16));
    let mut workers = Vec::new();
    for i in 0..16u8 {
        let (parent, barrier) = (root.path().to_path_buf(), barrier.clone());
        workers.push(std::thread::spawn(move || {
            let dest = parent.join(format!("record-{i}"));
            barrier.wait();
            atomic_write(&parent, &dest, &[i; 1024]).unwrap();
            assert_eq!(fs::read(dest).unwrap(), vec![i; 1024]);
        }));
    }
    for worker in workers {
        worker.join().unwrap();
    }
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 16);
}

#[test]
fn successful_explicit_cleanup_does_not_clean_a_successor_allocation() {
    let parent = tempfile::TempDir::new().unwrap();
    let owned = super::OwnedTemp::new(parent.path(), "finish-witness").unwrap();
    let location = owned.dir.clone();
    owned
        .finish_with(|dir| {
            std::fs::remove_dir(dir)?;
            // Deterministic interleaving: a new owner reserves the retired name
            // after successful cleanup, but before the old guard's Drop runs.
            std::fs::create_dir(dir)?;
            std::fs::write(dir.join("payload"), b"belongs-to-successor")
        })
        .unwrap();
    assert_eq!(
        std::fs::read(location.join("payload")).unwrap(),
        b"belongs-to-successor"
    );
}

#[test]
fn readonly_cleanup_is_local_to_owned_payload() {
    let parent = TempDir::new().unwrap();
    let source = parent.path().join("source");
    fs::write(&source, b"preserve-source").unwrap();
    let saved = fs::metadata(&source).unwrap().permissions();
    let mut readonly = saved.clone();
    readonly.set_readonly(true);
    fs::set_permissions(&source, readonly).unwrap();
    let allocation = OwnedTemp::new(parent.path(), "readonly-witness").unwrap();
    let payload = allocation.payload();
    let reservation = allocation.dir.clone();
    fs::copy(&source, &payload).unwrap();
    assert!(fs::metadata(&payload).unwrap().permissions().readonly());
    drop(allocation);
    assert!(!payload.exists(), "owned readonly payload leaked");
    assert!(!reservation.exists(), "owned reservation leaked");
    assert_eq!(fs::read(&source).unwrap(), b"preserve-source");
    assert!(fs::metadata(&source).unwrap().permissions().readonly());
    fs::set_permissions(source, saved).unwrap();
}
