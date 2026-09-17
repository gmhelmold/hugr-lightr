use super::publication_tests::{identity, path};
use super::{
    Cancellation, LeasedStagedFile, Phase, PreparationWork, PublicationOutcome, StoreLocks, Wait,
};
use lightr_core::Digest;
use std::fs::{self, File};
use std::io::{self, Cursor};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tempfile::TempDir;

#[test]
fn publication_phase_failures_leave_safe_states_and_shared_errors() {
    for (phase, payload_exists, receipt_exists) in [
        (Phase::PayloadSync, false, false),
        (Phase::PayloadInstall, false, false),
        (Phase::PayloadDirectory, true, false),
        (Phase::ReceiptSync, true, false),
        (Phase::ReceiptInstall, true, false),
        (Phase::ReceiptDirectory, true, true),
    ] {
        let root = TempDir::new().unwrap();
        let sentinel = root.path().join("sentinel");
        fs::write(&sentinel, b"keep").unwrap();
        let locks = StoreLocks::open_existing(root.path()).unwrap();
        let digest = Digest::of_bytes(b"transaction");
        let payload = path(root.path(), "objects", digest);
        let receipt = path(root.path(), "objects-ready", digest);
        {
            let lease = locks.shared(Wait::Try).unwrap();
            let stage =
                LeasedStagedFile::copy(&lease, &mut Cursor::new(b"transaction"), None, [20; 16])
                    .unwrap();
            let err = lease
                .prepare_with(digest, Some(stage), Wait::Try, [20; 16], &|step, _| {
                    if step == phase {
                        Err(io::Error::from_raw_os_error(5))
                    } else {
                        Ok(())
                    }
                })
                .unwrap_err();
            assert_eq!(err.phase, phase);
            assert_eq!(err.original_io().raw_os_error(), Some(5));
            assert_eq!(
                err.outcome,
                if phase == Phase::PayloadSync {
                    PublicationOutcome::NotPublished
                } else {
                    PublicationOutcome::CommitUncertain
                }
            );
            assert_eq!(payload.exists(), payload_exists, "{phase:?}");
            assert_eq!(receipt.exists(), receipt_exists, "{phase:?}");
            let again = lease
                .prepare_existing(digest, Wait::Try, [21; 16])
                .unwrap_err();
            assert!(
                Arc::ptr_eq(&err, &again),
                "same operation must retain the original failure"
            );
            assert_eq!(fs::read(&sentinel).unwrap(), b"keep");
        }
        let previous = if payload_exists {
            Some(File::open(&payload).unwrap())
        } else {
            None
        };
        let lease = locks.shared(Wait::Try).unwrap();
        let proof = lease
            .prepare_reader(&mut Cursor::new(b"transaction"), None, Wait::Try, [22; 16])
            .unwrap();
        let expected = if receipt_exists {
            PreparationWork::Reused
        } else if payload_exists {
            PreparationWork::Requalified
        } else {
            PreparationWork::Created
        };
        assert_eq!(proof.work(), expected);
        if let Some(old) = previous {
            assert_eq!(
                super::lease_io::identity(&old).unwrap() == identity(&payload),
                receipt_exists,
                "only receipt-confirmed payload is reusable in-place"
            );
        }
        assert_eq!(fs::read(&payload).unwrap(), b"transaction");
        assert!(receipt.exists());
        assert_eq!(fs::read(&sentinel).unwrap(), b"keep");
        assert_eq!(
            fs::read_dir(root.path().join(".si01-staging"))
                .unwrap()
                .count(),
            0
        );
    }
}

#[test]
fn publication_cancellation_after_install_does_not_issue_a_proof() {
    let root = TempDir::new().unwrap();
    let locks = StoreLocks::open_existing(root.path()).unwrap();
    let cancel = Cancellation::default();
    let digest = Digest::of_bytes(b"cancel");
    let lease = locks.shared(Wait::Try).unwrap();
    let stage =
        LeasedStagedFile::copy(&lease, &mut Cursor::new(b"cancel"), None, [23; 16]).unwrap();
    let err = lease
        .prepare_with(
            digest,
            Some(stage),
            Wait::Until {
                deadline: Instant::now() + Duration::from_secs(5),
                cancellation: &cancel,
            },
            [23; 16],
            &|phase, _| {
                if phase == Phase::PayloadDirectory {
                    cancel.cancel();
                }
                Ok(())
            },
        )
        .unwrap_err();
    assert_eq!(err.original_io().kind(), io::ErrorKind::Interrupted);
    assert_eq!(err.outcome, PublicationOutcome::CommitUncertain);
    assert!(path(root.path(), "objects", digest).exists());
    assert!(!path(root.path(), "objects-ready", digest).exists());
}

#[test]
fn publication_precancel_creates_no_staging_and_releases_no_foreign_state() {
    let root = TempDir::new().unwrap();
    let locks = StoreLocks::open_existing(root.path()).unwrap();
    let lease = locks.shared(Wait::Try).unwrap();
    let cancel = Cancellation::default();
    cancel.cancel();
    let result = lease.prepare_reader(
        &mut Cursor::new(b"cancel"),
        None,
        Wait::Until {
            deadline: Instant::now() + Duration::from_secs(5),
            cancellation: &cancel,
        },
        [24; 16],
    );
    assert_eq!(
        result.unwrap_err().original_io().kind(),
        io::ErrorKind::Interrupted
    );
    assert!(!root.path().join(".si01-staging").exists());
    assert!(!root.path().join("objects-ready").exists());
}

#[test]
fn publication_stage_from_another_lease_is_rejected_before_namespace_writes() {
    let root = TempDir::new().unwrap();
    let other = TempDir::new().unwrap();
    let a = StoreLocks::open_existing(root.path()).unwrap();
    let b = StoreLocks::open_existing(other.path()).unwrap();
    let a = a.shared(Wait::Try).unwrap();
    let b = b.shared(Wait::Try).unwrap();
    let staged = LeasedStagedFile::copy(&a, &mut Cursor::new(b"domain"), None, [25; 16]).unwrap();
    let error = b
        .prepare_with(
            staged.digest(),
            Some(staged),
            Wait::Try,
            [25; 16],
            &|_, _| Ok(()),
        )
        .unwrap_err();
    assert_eq!(error.phase, Phase::Validate);
    assert!(!other.path().join("objects").exists());
    assert!(!root.path().join("objects").exists());
}

#[test]
fn publication_wrong_namespace_type_is_rejected_without_cleanup_of_foreign_data() {
    let root = TempDir::new().unwrap();
    let sentinel = root.path().join("objects-ready");
    fs::write(&sentinel, b"not a directory").unwrap();
    let locks = StoreLocks::open_existing(root.path()).unwrap();
    let lease = locks.shared(Wait::Try).unwrap();
    assert!(lease
        .prepare_reader(&mut Cursor::new(b"bytes"), None, Wait::Try, [26; 16])
        .is_err());
    assert_eq!(fs::read(&sentinel).unwrap(), b"not a directory");
}

#[test]
fn publication_retains_shared_error_through_legacy_boundary() {
    let root = TempDir::new().unwrap();
    let locks = StoreLocks::open_existing(root.path()).unwrap();
    let lease = locks.shared(Wait::Try).unwrap();
    let stage =
        LeasedStagedFile::copy(&lease, &mut Cursor::new(b"shared"), None, [27; 16]).unwrap();
    let error = lease
        .prepare_with(
            stage.digest(),
            Some(stage),
            Wait::Try,
            [27; 16],
            &|phase, _| {
                if phase == Phase::ReceiptSync {
                    Err(io::Error::from_raw_os_error(5))
                } else {
                    Ok(())
                }
            },
        )
        .unwrap_err();
    let legacy = error.clone().into_shared_legacy();
    let decoded = super::PublicationFailure::from_legacy(&legacy).unwrap();
    assert!(std::ptr::eq(decoded, error.as_ref()));
    assert_eq!(decoded.original_io().raw_os_error(), Some(5));
    assert_eq!(decoded.phase, Phase::ReceiptSync);
    assert_eq!(decoded.operation_id, [27; 16]);
}

#[test]
fn publication_replaced_stage_name_cannot_install_unverified_bytes() {
    let root = TempDir::new().unwrap();
    let locks = StoreLocks::open_existing(root.path()).unwrap();
    let lease = locks.shared(Wait::Try).unwrap();
    let stage = LeasedStagedFile::copy(&lease, &mut Cursor::new(b"owned"), None, [28; 16]).unwrap();
    let source = stage.stage.staged_path();
    let digest = stage.digest();
    let displaced = root.path().join("retained-owned");
    let error = lease
        .prepare_with(digest, Some(stage), Wait::Try, [28; 16], &|phase, _| {
            if phase == Phase::PayloadInstall {
                fs::rename(&source, &displaced)?;
                fs::write(&source, b"other")?;
            }
            Ok(())
        })
        .unwrap_err();
    assert_eq!(error.phase, Phase::PayloadInstall);
    assert_eq!(error.original_io().kind(), io::ErrorKind::InvalidData);
    assert!(!path(root.path(), "objects", digest).exists());
    assert_eq!(fs::read(displaced).unwrap(), b"owned");
}

#[test]
fn publication_native_file_barrier_failure_is_not_ignored() {
    let root = TempDir::new().unwrap();
    let locks = StoreLocks::open_existing(root.path()).unwrap();
    let lease = locks.shared(Wait::Try).unwrap();
    let stage = LeasedStagedFile::copy(&lease, &mut Cursor::new(b"sync"), None, [29; 16]).unwrap();
    let digest = stage.digest();
    let mut effects = super::publication::Effects::new(&|_, _| Ok(()));
    effects.file_sync = &|_| Err(io::Error::from_raw_os_error(5));
    let error = lease
        .prepare_with_effects(digest, Some(stage), Wait::Try, [29; 16], effects)
        .unwrap_err();
    assert_eq!(error.phase, Phase::PayloadSync);
    assert_eq!(error.original_io().raw_os_error(), Some(5));
    assert!(!path(root.path(), "objects", digest).exists());
}

#[cfg(unix)]
#[test]
fn publication_native_directory_barrier_failure_is_not_ignored() {
    let root = TempDir::new().unwrap();
    let locks = StoreLocks::open_existing(root.path()).unwrap();
    let lease = locks.shared(Wait::Try).unwrap();
    let stage =
        LeasedStagedFile::copy(&lease, &mut Cursor::new(b"directory"), None, [30; 16]).unwrap();
    let digest = stage.digest();
    let mut effects = super::publication::Effects::new(&|_, _| Ok(()));
    effects.dir_sync = &|_| Err(io::Error::from_raw_os_error(5));
    let error = lease
        .prepare_with_effects(digest, Some(stage), Wait::Try, [30; 16], effects)
        .unwrap_err();
    assert_eq!(error.phase, Phase::PayloadDirectory);
    assert!(path(root.path(), "objects", digest).exists());
    assert!(!path(root.path(), "objects-ready", digest).exists());
}
