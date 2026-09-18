//! Cancellation must stop inspection, not be discovered at receipt publication.
use super::publication_tests::{identity, legacy, path};
use super::{Cancellation, Phase, PreparationWork, PublicationOutcome, StoreLocks, Wait};
use lightr_core::Digest;
use std::cell::RefCell;
use std::fs;
use std::io;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tempfile::TempDir;

#[test]
fn inspection_cancelled_at_entry_stops_before_receipt_work() {
    for ready in [false, true] {
        let root = TempDir::new().unwrap();
        let locks = StoreLocks::open_existing(root.path()).unwrap();
        let bytes = vec![23; 131_073];
        let digest = if ready {
            let lease = locks.shared(Wait::Try).unwrap();
            lease
                .prepare_reader(&mut bytes.as_slice(), None, Wait::Try, [71; 16])
                .unwrap()
                .digest()
        } else {
            legacy(root.path(), &bytes)
        };
        let payload = path(root.path(), "objects", digest);
        let receipt = path(root.path(), "objects-ready", digest);
        let payload_id = identity(&payload);
        let receipt_bytes = fs::read(&receipt).ok();
        let receipt_id = ready.then(|| identity(&receipt));
        let lease = locks.shared(Wait::Try).unwrap();
        let cancellation = Cancellation::default();
        let phases = RefCell::new(Vec::new());
        let error = lease
            .prepare_with(
                digest,
                None,
                Wait::Until {
                    deadline: Instant::now() + Duration::from_secs(10),
                    cancellation: &cancellation,
                },
                [72; 16],
                &|phase, _| {
                    phases.borrow_mut().push(phase);
                    if phase == Phase::Verify {
                        cancellation.cancel();
                    }
                    Ok(())
                },
            )
            .unwrap_err();
        assert_eq!(
            error.phase,
            Phase::Verify,
            "cancellation must stop in inspection, before receipt work"
        );
        assert_eq!(
            error.outcome,
            PublicationOutcome::NotPublished,
            "inspection checkpoint cancellation must not imply recovery"
        );
        assert_eq!(error.operation_id, [72; 16]);
        assert_eq!(
            error
                .relative_path
                .as_ref()
                .unwrap()
                .canonicalize()
                .unwrap(),
            payload.canonicalize().unwrap()
        );
        assert_eq!(error.original_io().kind(), io::ErrorKind::Interrupted);
        assert_eq!(phases.borrow().last(), Some(&Phase::Verify));
        assert_eq!(fs::read(&payload).unwrap(), bytes);
        assert_eq!(identity(&payload), payload_id);
        assert_eq!(fs::read(&receipt).ok(), receipt_bytes);
        assert_eq!(ready.then(|| identity(&receipt)), receipt_id);
        assert_eq!(
            fs::read_dir(root.path().join(".si01-staging"))
                .unwrap()
                .count(),
            0
        );
        let cached = lease
            .prepare_existing(digest, Wait::Try, [73; 16])
            .unwrap_err();
        assert!(Arc::ptr_eq(&error, &cached));
        let other = lease
            .prepare_reader(&mut b"other digest".as_slice(), None, Wait::Try, [74; 16])
            .unwrap();
        assert_eq!(other.work(), PreparationWork::Created);
        drop(lease);
        let next = locks.shared(Wait::Try).unwrap();
        let proof = next.prepare_existing(digest, Wait::Try, [75; 16]).unwrap();
        assert_eq!(
            proof.work(),
            if ready {
                PreparationWork::Reused
            } else {
                PreparationWork::Requalified
            }
        );
    }
}

#[test]
fn inspection_preserves_real_io_error_when_cancellation_also_occurs() {
    let root = TempDir::new().unwrap();
    let bytes = b"retain actual I/O cause";
    let digest = legacy(root.path(), bytes);
    let locks = StoreLocks::open_existing(root.path()).unwrap();
    let lease = locks.shared(Wait::Try).unwrap();
    let cancellation = Cancellation::default();
    let error = lease
        .prepare_with(
            digest,
            None,
            Wait::Until {
                deadline: Instant::now() + Duration::from_secs(10),
                cancellation: &cancellation,
            },
            [76; 16],
            &|phase, _| {
                if phase == Phase::Verify {
                    cancellation.cancel();
                    return Err(io::Error::from_raw_os_error(5));
                }
                Ok(())
            },
        )
        .unwrap_err();
    assert_eq!(error.original_io().raw_os_error(), Some(5));
    assert_eq!(error.phase, Phase::Verify);
    assert_eq!(error.outcome, PublicationOutcome::RecoveryRequired);
    assert_eq!(error.operation_id, [76; 16]);
    assert_eq!(
        fs::read(path(root.path(), "objects", digest)).unwrap(),
        bytes
    );
    assert!(!path(root.path(), "objects-ready", digest).exists());
    assert_eq!(digest, Digest::of_bytes(bytes));
}

#[test]
fn inspection_checks_between_real_payload_blocks() {
    use super::publication_io::Layout;
    use std::cell::Cell;
    for ready in [false, true] {
        let root = TempDir::new().unwrap();
        let locks = StoreLocks::open_existing(root.path()).unwrap();
        let bytes = vec![41; 196_609];
        let digest = legacy(root.path(), &bytes);
        if ready {
            locks
                .shared(Wait::Try)
                .unwrap()
                .prepare_existing(digest, Wait::Try, [77; 16])
                .unwrap();
        }
        let lease = locks.shared(Wait::Try).unwrap();
        let layout = Layout::open(&lease, digest).unwrap();
        let payload = layout.payload_path();
        let receipt = layout.receipt_path();
        let payload_id = identity(&payload);
        let original_receipt = fs::read(&receipt).ok();
        for kind in [io::ErrorKind::Interrupted, io::ErrorKind::TimedOut] {
            let checks = Cell::new(0);
            let error = layout
                .inspect(digest, &|| {
                    let count = checks.get() + 1;
                    checks.set(count);
                    // Two metadata checks precede the hash. Check five is before
                    // the second block: removing the hash checkpoints cannot pass.
                    if count == 5 {
                        Err(io::Error::new(kind, "controlled inspection stop"))
                    } else {
                        Ok(())
                    }
                })
                .unwrap_err();
            assert_eq!(error.kind(), kind);
            assert_eq!(error.to_string(), "controlled inspection stop");
            assert_eq!(checks.get(), 5);
            assert_eq!(fs::read(&payload).unwrap(), bytes);
            assert_eq!(identity(&payload), payload_id);
            assert_eq!(fs::read(&receipt).ok(), original_receipt);
            assert_eq!(
                fs::read_dir(root.path().join(".si01-staging"))
                    .unwrap()
                    .count(),
                0
            );
        }
    }
}

#[test]
fn inspection_cancellation_at_empty_payload_eof_is_terminal() {
    use super::publication_io::Layout;
    use std::cell::Cell;
    let root = TempDir::new().unwrap();
    let digest = legacy(root.path(), b"");
    let locks = StoreLocks::open_existing(root.path()).unwrap();
    let lease = locks.shared(Wait::Try).unwrap();
    let layout = Layout::open(&lease, digest).unwrap();
    let checks = Cell::new(0);
    let error = layout
        .inspect(digest, &|| {
            checks.set(checks.get() + 1);
            // Metadata, metadata, before read, after EOF.
            if checks.get() == 4 {
                Err(io::ErrorKind::Interrupted.into())
            } else {
                Ok(())
            }
        })
        .unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::Interrupted);
    assert_eq!(checks.get(), 4);
    assert!(!layout.receipt_path().exists());
    assert_eq!(fs::metadata(layout.payload_path()).unwrap().len(), 0);
}

#[test]
fn inspection_precancel_and_expired_deadline_precede_receipt_decoding() {
    use super::publication_io::Layout;
    let root = TempDir::new().unwrap();
    let digest = legacy(root.path(), b"preserved");
    let locks = StoreLocks::open_existing(root.path()).unwrap();
    let lease = locks.shared(Wait::Try).unwrap();
    let layout = Layout::open(&lease, digest).unwrap();
    fs::write(
        layout.receipt_path(),
        b"invalid receipt, must not be decoded",
    )
    .unwrap();
    for cancel in [true, false] {
        let cancellation = Cancellation::default();
        if cancel {
            cancellation.cancel();
        }
        let deadline = if cancel {
            Instant::now() + Duration::from_secs(10)
        } else {
            Instant::now() - Duration::from_secs(1)
        };
        let wait = Wait::Until {
            deadline,
            cancellation: &cancellation,
        };
        let error = layout.inspect(digest, &|| wait.check()).unwrap_err();
        assert_eq!(
            error.kind(),
            if cancel {
                io::ErrorKind::Interrupted
            } else {
                io::ErrorKind::TimedOut
            }
        );
    }
    assert_eq!(
        fs::read(layout.receipt_path()).unwrap(),
        b"invalid receipt, must not be decoded"
    );
    assert_eq!(fs::read(layout.payload_path()).unwrap(), b"preserved");
}

#[test]
fn inspection_uncancelled_keeps_verification_and_returns_rewound_file() {
    use super::publication_io::Layout;
    use std::io::{Read, Seek};
    for length in [0, 1, 65_536, 65_537, 196_609] {
        let root = TempDir::new().unwrap();
        let bytes = vec![83; length];
        let digest = legacy(root.path(), &bytes);
        let locks = StoreLocks::open_existing(root.path()).unwrap();
        let lease = locks.shared(Wait::Try).unwrap();
        let layout = Layout::open(&lease, digest).unwrap();
        let (file, ready, actual) = layout.inspect(digest, &|| Ok(())).unwrap();
        assert!(!ready);
        assert_eq!(actual, length as u64);
        let mut file = file.unwrap();
        assert_eq!(file.stream_position().unwrap(), 0);
        let mut read = Vec::new();
        file.read_to_end(&mut read).unwrap();
        assert_eq!(read, bytes);
        fs::write(layout.payload_path(), b"different").unwrap();
        assert_eq!(
            layout.inspect(digest, &|| Ok(())).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
    }
}
