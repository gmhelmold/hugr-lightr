//! A cancelled inspection leader wakes its waiter without publishing any bytes.
use super::publication_tests::{legacy, path};
use super::{Cancellation, Phase, PreparationWork, PublicationOutcome, StoreLocks, Wait};
use std::fs;
use std::io;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tempfile::TempDir;

#[test]
fn inspection_cancellation_wakes_waiter_and_preserves_distinct_progress() {
    let root = TempDir::new().unwrap();
    let bytes = b"existing unconfirmed object";
    let digest = legacy(root.path(), bytes);
    let locks = StoreLocks::open_existing(root.path()).unwrap();
    let lease = locks.shared(Wait::Try).unwrap();
    let leader_cancel = Cancellation::default();
    let waiter_cancel = Cancellation::default();
    let (entered_tx, entered_rx) = mpsc::channel();
    let (resume_tx, resume_rx) = mpsc::channel();
    let resume_rx = Mutex::new(resume_rx);
    let (waiting_tx, waiting_rx) = mpsc::channel();
    thread::scope(|scope| {
        let leader = scope.spawn(|| {
            lease
                .prepare_with(
                    digest,
                    None,
                    Wait::Until {
                        deadline: Instant::now() + Duration::from_secs(10),
                        cancellation: &leader_cancel,
                    },
                    [78; 16],
                    &|phase, _| {
                        if phase == Phase::Verify {
                            entered_tx.send(()).unwrap();
                            resume_rx
                                .lock()
                                .unwrap()
                                .recv_timeout(Duration::from_secs(10))
                                .unwrap();
                        }
                        Ok(())
                    },
                )
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
                        cancellation: &waiter_cancel,
                    },
                    [79; 16],
                    || panic!("waiter became a producer"),
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
        let other = lease
            .prepare_reader(&mut b"independent".as_slice(), None, Wait::Try, [80; 16])
            .unwrap();
        assert_eq!(other.work(), PreparationWork::Created);
        leader_cancel.cancel();
        resume_tx.send(()).unwrap();
        let failure = leader.join().unwrap();
        let observed = waiter.join().unwrap();
        assert!(Arc::ptr_eq(&failure, &observed));
        assert_eq!(observed.phase, Phase::Verify);
        assert_eq!(observed.outcome, PublicationOutcome::NotPublished);
        assert_eq!(observed.operation_id, [78; 16]);
        assert_eq!(observed.original_io().kind(), io::ErrorKind::Interrupted);
    });
    assert_eq!(
        fs::read(path(root.path(), "objects", digest)).unwrap(),
        bytes
    );
    assert!(!path(root.path(), "objects-ready", digest).exists());
    assert_eq!(
        fs::read_dir(root.path().join(".si01-staging"))
            .unwrap()
            .count(),
        0
    );
}
