use super::{
    Cancellation, LeasedStagedFile, Phase, PreparationWork, PublicationOutcome, StoreLocks, Wait,
};
use lightr_core::Digest;
use std::io::{self, Cursor};
use std::sync::{mpsc, Arc, Barrier, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tempfile::TempDir;

#[test]
fn publication_shared_lease_publishes_once_and_counts_only_the_leader() {
    let root = TempDir::new().unwrap();
    let locks = StoreLocks::open_existing(root.path()).unwrap();
    let lease = locks.shared(Wait::Try).unwrap();
    let cancel = Cancellation::default();
    let barrier = Barrier::new(8);
    let works = thread::scope(|scope| {
        let jobs: Vec<_> = (0..8)
            .map(|n| {
                let lease = &lease;
                let barrier = &barrier;
                let cancel = &cancel;
                scope.spawn(move || {
                    barrier.wait();
                    lease
                        .prepare_reader(
                            &mut Cursor::new(b"duplicate"),
                            None,
                            Wait::Until {
                                deadline: Instant::now() + Duration::from_secs(10),
                                cancellation: cancel,
                            },
                            [n; 16],
                        )
                        .unwrap()
                        .work()
                })
            })
            .collect();
        jobs.into_iter()
            .map(|j| j.join().unwrap())
            .collect::<Vec<_>>()
    });
    assert_eq!(
        works
            .iter()
            .filter(|&&w| w == PreparationWork::Created)
            .count(),
        1
    );
    assert_eq!(
        works
            .iter()
            .filter(|&&w| w == PreparationWork::LeaseCached)
            .count(),
        7
    );
}

#[test]
fn publication_concurrent_leases_keep_one_confirmed_payload() {
    let root = TempDir::new().unwrap();
    let locks = StoreLocks::open_existing(root.path()).unwrap();
    let cancel = Cancellation::default();
    let barrier = Barrier::new(6);
    let works = thread::scope(|scope| {
        let jobs: Vec<_> = (0..6)
            .map(|n| {
                let locks = &locks;
                let barrier = &barrier;
                let cancel = &cancel;
                scope.spawn(move || {
                    let lease = locks.shared(Wait::Try).unwrap();
                    barrier.wait();
                    lease
                        .prepare_reader(
                            &mut Cursor::new(b"cross operation"),
                            None,
                            Wait::Until {
                                deadline: Instant::now() + Duration::from_secs(10),
                                cancellation: cancel,
                            },
                            [n; 16],
                        )
                        .unwrap()
                        .work()
                })
            })
            .collect();
        jobs.into_iter()
            .map(|j| j.join().unwrap())
            .collect::<Vec<_>>()
    });
    assert_eq!(
        works
            .iter()
            .filter(|&&w| w == PreparationWork::Created)
            .count(),
        1
    );
    assert_eq!(
        works
            .iter()
            .filter(|&&w| w == PreparationWork::Reused)
            .count(),
        5
    );
}

#[test]
fn publication_inflight_failure_is_shared_and_distinct_digest_progresses() {
    let root = TempDir::new().unwrap();
    let locks = StoreLocks::open_existing(root.path()).unwrap();
    let lease = locks.shared(Wait::Try).unwrap();
    let (arrived_tx, arrived_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let release_rx = Mutex::new(release_rx);
    let digest = Digest::of_bytes(b"blocked");
    thread::scope(|scope| {
        let worker = scope.spawn(|| {
            let stage =
                LeasedStagedFile::copy(&lease, &mut Cursor::new(b"blocked"), None, [31; 16])
                    .unwrap();
            lease
                .prepare_with(digest, Some(stage), Wait::Try, [31; 16], &|phase, _| {
                    if phase == Phase::PayloadSync {
                        arrived_tx.send(()).unwrap();
                        release_rx
                            .lock()
                            .unwrap()
                            .recv_timeout(Duration::from_secs(10))
                            .unwrap();
                        return Err(io::Error::from_raw_os_error(5));
                    }
                    Ok(())
                })
                .unwrap_err()
        });
        arrived_rx.recv_timeout(Duration::from_secs(10)).unwrap();
        let busy = lease
            .prepare_existing(digest, Wait::Try, [32; 16])
            .unwrap_err();
        assert_eq!(busy.original_io().kind(), io::ErrorKind::WouldBlock);
        let other = lease
            .prepare_reader(&mut Cursor::new(b"independent"), None, Wait::Try, [33; 16])
            .unwrap();
        assert_eq!(other.work(), PreparationWork::Created);
        release_tx.send(()).unwrap();
        let failure = worker.join().unwrap();
        let observed = lease
            .prepare_existing(digest, Wait::Try, [34; 16])
            .unwrap_err();
        assert!(Arc::ptr_eq(&failure, &observed));
        assert_eq!(observed.operation_id, [31; 16]);
    });
}

#[test]
fn publication_unwinding_leader_marks_failure_instead_of_leaving_pending_work() {
    let root = TempDir::new().unwrap();
    let locks = StoreLocks::open_existing(root.path()).unwrap();
    let lease = locks.shared(Wait::Try).unwrap();
    let digest = Digest::of_bytes(b"panic");
    let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let stage =
            LeasedStagedFile::copy(&lease, &mut Cursor::new(b"panic"), None, [35; 16]).unwrap();
        lease.prepare_with(digest, Some(stage), Wait::Try, [35; 16], &|phase, _| {
            if phase == Phase::PayloadSync {
                panic!("controlled worker unwind");
            }
            Ok(())
        })
    }));
    assert!(panicked.is_err());
    let error = lease
        .prepare_existing(digest, Wait::Try, [36; 16])
        .unwrap_err();
    assert_eq!(error.outcome, PublicationOutcome::RecoveryRequired);
    assert_eq!(error.operation_id, [35; 16]);
    drop(lease);
    let lease = locks.shared(Wait::Try).unwrap();
    assert!(lease
        .prepare_reader(&mut Cursor::new(b"panic"), None, Wait::Try, [37; 16])
        .is_ok());
}

#[test]
fn publication_observed_waiter_receives_the_exact_leader_failure() {
    let root = TempDir::new().unwrap();
    let locks = StoreLocks::open_existing(root.path()).unwrap();
    let lease = locks.shared(Wait::Try).unwrap();
    let digest = Digest::of_bytes(b"wake");
    let cancel = Cancellation::default();
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let release_rx = Mutex::new(release_rx);
    let (waiting_tx, waiting_rx) = mpsc::channel();
    thread::scope(|scope| {
        let leader = scope.spawn(|| {
            let staged =
                LeasedStagedFile::copy(&lease, &mut Cursor::new(b"wake"), None, [38; 16]).unwrap();
            lease
                .prepare_with(digest, Some(staged), Wait::Try, [38; 16], &|phase, _| {
                    if phase == Phase::PayloadDirectory {
                        entered_tx.send(()).unwrap();
                        release_rx
                            .lock()
                            .unwrap()
                            .recv_timeout(Duration::from_secs(10))
                            .unwrap();
                        return Err(io::Error::from_raw_os_error(5));
                    }
                    Ok(())
                })
                .unwrap_err()
        });
        entered_rx.recv_timeout(Duration::from_secs(10)).unwrap();
        let waiter = scope.spawn(|| {
            let mut first = true;
            lease
                .preparations
                .run_observed(
                    digest,
                    Wait::Until {
                        deadline: Instant::now() + Duration::from_secs(10),
                        cancellation: &cancel,
                    },
                    [39; 16],
                    || panic!("waiter must not become leader"),
                    || {
                        if first {
                            first = false;
                            waiting_tx.send(()).unwrap();
                        }
                    },
                )
                .unwrap_err()
        });
        waiting_rx.recv_timeout(Duration::from_secs(10)).unwrap();
        release_tx.send(()).unwrap();
        let failure = leader.join().unwrap();
        let observed = waiter.join().unwrap();
        assert!(Arc::ptr_eq(&failure, &observed));
        assert_eq!(observed.phase, Phase::PayloadDirectory);
    });
}

#[test]
fn publication_cancelled_observed_waiter_does_not_cancel_or_poison_leader() {
    let root = TempDir::new().unwrap();
    let locks = StoreLocks::open_existing(root.path()).unwrap();
    let lease = locks.shared(Wait::Try).unwrap();
    let digest = Digest::of_bytes(b"cancel waiter");
    let cancel = Cancellation::default();
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let release_rx = Mutex::new(release_rx);
    let (waiting_tx, waiting_rx) = mpsc::channel();
    thread::scope(|scope| {
        let leader = scope.spawn(|| {
            let staged =
                LeasedStagedFile::copy(&lease, &mut Cursor::new(b"cancel waiter"), None, [40; 16])
                    .unwrap();
            lease
                .prepare_with(digest, Some(staged), Wait::Try, [40; 16], &|phase, _| {
                    if phase == Phase::PayloadSync {
                        entered_tx.send(()).unwrap();
                        release_rx
                            .lock()
                            .unwrap()
                            .recv_timeout(Duration::from_secs(10))
                            .unwrap();
                    }
                    Ok(())
                })
                .unwrap()
                .work()
        });
        entered_rx.recv_timeout(Duration::from_secs(10)).unwrap();
        let waiter = scope.spawn(|| {
            let mut first = true;
            lease
                .preparations
                .run_observed(
                    digest,
                    Wait::Until {
                        deadline: Instant::now() + Duration::from_secs(10),
                        cancellation: &cancel,
                    },
                    [41; 16],
                    || panic!("unexpected leader"),
                    || {
                        if first {
                            first = false;
                            waiting_tx.send(()).unwrap();
                        }
                    },
                )
                .unwrap_err()
        });
        waiting_rx.recv_timeout(Duration::from_secs(10)).unwrap();
        cancel.cancel();
        assert_eq!(
            waiter.join().unwrap().original_io().kind(),
            io::ErrorKind::Interrupted
        );
        release_tx.send(()).unwrap();
        assert_eq!(leader.join().unwrap(), PreparationWork::Created);
    });
    assert_eq!(
        lease
            .prepare_existing(digest, Wait::Try, [42; 16])
            .unwrap()
            .work(),
        PreparationWork::LeaseCached
    );
}
