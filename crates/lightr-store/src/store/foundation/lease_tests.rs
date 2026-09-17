use super::{CacheLocks, Cancellation, LeasedStagedFile, StoreLocks, Wait};
use lightr_core::Digest;
use std::fs;
use std::io::{self, Cursor, Read};
use std::sync::mpsc;
use std::time::{Duration, Instant};
use tempfile::TempDir;

#[test]
fn lease_shared_excludes_gc_and_preserves_lock_file() {
    let root = TempDir::new().unwrap();
    let lock = root.path().join(".gc.lock");
    fs::write(&lock, b"do-not-truncate").unwrap();
    let domain = StoreLocks::open_existing(root.path()).unwrap();
    let a = domain.shared(Wait::Try).unwrap();
    let b = domain.shared(Wait::Try).unwrap();
    assert!(a.belongs_to(&domain));
    assert_eq!(domain.exclusive(Wait::Try).unwrap_err().kind(), io::ErrorKind::WouldBlock);
    drop(a);
    assert_eq!(domain.exclusive(Wait::Try).unwrap_err().kind(), io::ErrorKind::WouldBlock);
    drop(b);
    let exclusive = domain.exclusive(Wait::Try).unwrap();
    assert!(exclusive.belongs_to(&domain));
    assert_eq!(domain.shared(Wait::Try).unwrap_err().kind(), io::ErrorKind::WouldBlock);
    drop(exclusive);
    assert_eq!(fs::read(lock).unwrap(), b"do-not-truncate");
    assert!(domain.shared(Wait::Try).is_ok());
}

#[test]
fn lease_interoperates_with_existing_gc_guards() {
    let root = TempDir::new().unwrap();
    let domain = StoreLocks::open_existing(root.path()).unwrap();
    let old_shared = crate::store::lock::write_guard(root.path()).unwrap();
    assert_eq!(domain.exclusive(Wait::Try).unwrap_err().kind(), io::ErrorKind::WouldBlock);
    drop(old_shared);
    let old_exclusive = crate::store::lock::gc_guard(root.path()).unwrap();
    assert_eq!(domain.shared(Wait::Try).unwrap_err().kind(), io::ErrorKind::WouldBlock);
    drop(old_exclusive);
    assert!(domain.exclusive(Wait::Try).is_ok());
}

#[test]
fn lease_cancellation_after_observed_contention_releases_waiter() {
    let root = TempDir::new().unwrap();
    let domain = StoreLocks::open_existing(root.path()).unwrap();
    let holder = domain.shared(Wait::Try).unwrap();
    let directory = super::lease_io::Directory::open(root.path()).unwrap();
    let cancellation = Cancellation::default();
    let wait = Wait::Until { deadline: Instant::now() + Duration::from_secs(5), cancellation: &cancellation };
    let mut observed = false;
    let error = super::lease_io::NativeLock::acquire_observed(&directory, ".gc.lock", false, wait, || {
        observed = true;
        cancellation.cancel();
    }).unwrap_err();
    assert!(observed, "test must observe the native WouldBlock before cancelling");
    assert_eq!(error.kind(), io::ErrorKind::Interrupted);
    drop(holder);
    assert!(domain.exclusive(Wait::Try).is_ok());
}

#[test]
fn lease_deadline_and_precancel_do_not_publish_or_poison() {
    let root = TempDir::new().unwrap();
    let domain = StoreLocks::open_existing(root.path()).unwrap();
    let cancel = Cancellation::default();
    cancel.cancel();
    let cancelled = Wait::Until { deadline: Instant::now() + Duration::from_secs(5), cancellation: &cancel };
    assert_eq!(domain.shared(cancelled).unwrap_err().kind(), io::ErrorKind::Interrupted);
    assert!(!root.path().join(".gc.lock").exists());
    let holder = domain.shared(Wait::Try).unwrap();
    let not_cancelled = Cancellation::default();
    let deadline = Wait::Until { deadline: Instant::now() + Duration::from_millis(10), cancellation: &not_cancelled };
    assert_eq!(domain.exclusive(deadline).unwrap_err().kind(), io::ErrorKind::TimedOut);
    drop(holder);
    assert!(domain.exclusive(Wait::Try).is_ok());
}

#[test]
fn lease_threads_share_one_guard_without_reacquiring() {
    let root = TempDir::new().unwrap();
    let domain = StoreLocks::open_existing(root.path()).unwrap();
    let lease = domain.shared(Wait::Try).unwrap();
    let (arrived, arrivals) = mpsc::channel();
    let (release, released) = mpsc::channel();
    std::thread::scope(|scope| {
        let lease_ref = &lease;
        scope.spawn(move || {
            let _worker = lease_ref.worker();
            arrived.send(()).unwrap();
            released.recv_timeout(Duration::from_secs(5)).unwrap();
        });
        arrivals.recv_timeout(Duration::from_secs(5)).unwrap();
        // Try has reached the actual lock boundary, not a timing inference.
        let error = domain.exclusive(Wait::Try).unwrap_err();
        release.send(()).unwrap();
        assert_eq!(error.kind(), io::ErrorKind::WouldBlock);
    });
    drop(lease);
    assert!(domain.exclusive(Wait::Try).is_ok());
}

#[test]
fn lease_store_aliases_share_identity_and_domain() {
    let root = TempDir::new().unwrap();
    fs::create_dir(root.path().join("child")).unwrap();
    let domain = StoreLocks::open_existing(root.path()).unwrap();
    let alias = StoreLocks::open_existing(&root.path().join("child/..")).unwrap();
    assert!(domain.same_store(&alias));
    let lease = domain.shared(Wait::Try).unwrap();
    assert!(lease.belongs_to(&alias));
    assert_eq!(alias.exclusive(Wait::Try).unwrap_err().kind(), io::ErrorKind::WouldBlock);
}

#[cfg(unix)]
#[test]
fn lease_symlink_aliases_converge_but_internal_links_are_rejected() {
    use std::os::unix::fs::symlink;
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().join("store");
    fs::create_dir(&root).unwrap();
    symlink(&root, tmp.path().join("alias")).unwrap();
    let domain = StoreLocks::open_existing(&root).unwrap();
    let alias = StoreLocks::open_existing(&tmp.path().join("alias")).unwrap();
    assert!(domain.same_store(&alias));
    let lease = domain.shared(Wait::Try).unwrap();
    assert_eq!(alias.exclusive(Wait::Try).unwrap_err().kind(), io::ErrorKind::WouldBlock);
    drop(lease);
    fs::remove_file(root.join(".gc.lock")).unwrap(); // Test-only hostile fixture.
    let sentinel = tmp.path().join("sentinel");
    fs::write(&sentinel, b"retained").unwrap();
    symlink(&sentinel, root.join(".gc.lock")).unwrap();
    assert!(domain.shared(Wait::Try).is_err());
    assert_eq!(fs::read(sentinel).unwrap(), b"retained");
}

#[cfg(unix)]
#[test]
fn lease_rejects_replaced_root_before_creating_lock() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().join("store");
    fs::create_dir(&root).unwrap();
    let domain = StoreLocks::open_existing(&root).unwrap();
    fs::rename(&root, tmp.path().join("old")).unwrap();
    fs::create_dir(&root).unwrap();
    assert_eq!(domain.shared(Wait::Try).unwrap_err().kind(), io::ErrorKind::InvalidData);
    assert!(!root.join(".gc.lock").exists());
}

#[test]
fn lease_foreign_store_exclusive_is_not_shared_cache_exclusion() {
    let a = TempDir::new().unwrap();
    let b = TempDir::new().unwrap();
    let cache = TempDir::new().unwrap();
    let store_a = StoreLocks::open_existing(a.path()).unwrap();
    let store_b = StoreLocks::open_existing(b.path()).unwrap();
    let cache_b = CacheLocks::open_existing(cache.path()).unwrap();
    let cache_a = CacheLocks::open_existing(&cache.path().join(".")).unwrap();
    assert!(!store_a.same_store(&store_b));
    assert!(cache_a.same_cache(&cache_b));
    let lease_b = store_b.shared(Wait::Try).unwrap();
    let mut worker = lease_b.worker();
    let cache_guard = worker.cache(&cache_b, Wait::Try).unwrap();
    let foreign_gc = store_a.exclusive(Wait::Try).unwrap();
    assert_eq!(cache_a.exclusive(Wait::Try).unwrap_err().kind(), io::ErrorKind::WouldBlock);
    drop(cache_guard);
    let standalone = cache_a.exclusive(Wait::Try).unwrap();
    assert_eq!(worker.cache(&cache_b, Wait::Try).unwrap_err().kind(), io::ErrorKind::WouldBlock);
    drop(standalone);
    drop(foreign_gc);
    assert!(worker.cache(&cache_b, Wait::Try).is_ok());
}

#[test]
fn lease_digest_set_is_sorted_and_failed_partial_acquisition_is_released() {
    let root = TempDir::new().unwrap();
    let domain = StoreLocks::open_existing(root.path()).unwrap();
    let a = domain.shared(Wait::Try).unwrap();
    let b = domain.shared(Wait::Try).unwrap();
    let low = Digest([1; 32]);
    let high = Digest([2; 32]);
    let mut first = a.worker();
    let mut second = b.worker();
    let high_guard = first.digests(&[high], Wait::Try).unwrap();
    assert_eq!(second.digests(&[high, low, low], Wait::Try).unwrap_err().kind(), io::ErrorKind::WouldBlock);
    // If failed multi-key acquisition leaked low, this independent probe fails.
    let mut third = a.worker();
    let low_guard = third.digests(&[low], Wait::Try).unwrap();
    drop(low_guard);
    drop(high_guard);
    let all = second.digests(&[high, low, high], Wait::Try).unwrap();
    assert_eq!(all.keys(), &[low, high]);
}

#[test]
fn lease_distinct_digest_workers_progress_independently() {
    let root = TempDir::new().unwrap();
    let domain = StoreLocks::open_existing(root.path()).unwrap();
    let lease = domain.shared(Wait::Try).unwrap();
    let a = Digest([1; 32]);
    let b = Digest([2; 32]);
    let mut one = lease.worker();
    let mut two = lease.worker();
    let held = one.digests(&[a], Wait::Try).unwrap();
    assert!(two.digests(&[b], Wait::Try).is_ok());
    assert_eq!(two.digests(&[a], Wait::Try).unwrap_err().kind(), io::ErrorKind::WouldBlock);
    drop(held);
    assert!(two.digests(&[a], Wait::Try).is_ok());
}

#[test]
fn lease_staging_borrows_guard_and_keeps_source_and_sentinel() {
    let root = TempDir::new().unwrap();
    let domain = StoreLocks::open_existing(root.path()).unwrap();
    let lease = domain.shared(Wait::Try).unwrap();
    let sentinel = root.path().join("sentinel");
    fs::write(&sentinel, b"keep").unwrap();
    let bytes = vec![7u8; 131_073];
    let mut staged = LeasedStagedFile::copy(&lease, &mut Cursor::new(&bytes), Some((Digest::of_bytes(&bytes), bytes.len() as u64)), [1;16]).unwrap();
    assert_eq!(domain.exclusive(Wait::Try).unwrap_err().kind(), io::ErrorKind::WouldBlock);
    let mut actual = Vec::new();
    staged.read_to_end(&mut actual).unwrap();
    assert_eq!(actual, bytes);
    staged.rewind().unwrap();
    assert_eq!(staged.len(), bytes.len() as u64);
    drop(staged);
    assert_eq!(fs::read_dir(root.path().join(".si01-staging")).unwrap().count(), 0);
    assert_eq!(fs::read(sentinel).unwrap(), b"keep");
    assert_eq!(domain.exclusive(Wait::Try).unwrap_err().kind(), io::ErrorKind::WouldBlock);
    drop(lease);
    assert!(domain.exclusive(Wait::Try).is_ok());
}

#[test]
fn lease_missing_or_nondirectory_roots_do_not_get_created() {
    let tmp = TempDir::new().unwrap();
    let absent = tmp.path().join("absent");
    assert_eq!(StoreLocks::open_existing(&absent).unwrap_err().kind(), io::ErrorKind::NotFound);
    assert!(!absent.exists());
    fs::write(tmp.path().join("file"), b"keep").unwrap();
    assert!(StoreLocks::open_existing(&tmp.path().join("file")).is_err());
}
