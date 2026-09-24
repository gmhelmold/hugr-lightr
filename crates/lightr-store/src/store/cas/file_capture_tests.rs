use super::*;
use std::cell::Cell;
use std::io::{Read, Write};
use tempfile::TempDir;

fn fixture(bytes: &[u8]) -> (TempDir, std::path::PathBuf, std::path::PathBuf) {
    let root = TempDir::new().unwrap();
    let source = root.path().join("source");
    let parent = root.path().join("staging");
    fs::write(&source, bytes).unwrap();
    fs::create_dir(&parent).unwrap();
    (root, source, parent)
}
fn fake_clone(
    source: &File,
    path: &Path,
    _: &dyn Fn() -> io::Result<()>,
) -> io::Result<(File, CowRung)> {
    let mut input = source.try_clone()?;
    input.rewind()?;
    let mut file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(path)?;
    io::copy(&mut input, &mut file)?;
    Ok((file, CowRung::Clone)) // Test seam, never presented as a native clone witness.
}
fn capture(
    parent: &Path,
    source: &Path,
    mode: CaptureMode,
    effects: CaptureEffects<'_>,
) -> Result<(StagedFile, CaptureMethod), PublicationFailure> {
    let bytes = fs::read(source).unwrap();
    StagedFile::capture_with(
        parent,
        File::open(source).unwrap(),
        mode,
        Some((Digest::of_bytes(&bytes), bytes.len() as u64)),
        [23; 16],
        effects,
    )
}
fn effects(checkpoint: &dyn Fn() -> io::Result<()>) -> CaptureEffects<'_> {
    CaptureEffects {
        clone_file: fake_clone,
        sync: &File::sync_all,
        checkpoint,
    }
}
fn assert_clean(parent: &Path) {
    assert_eq!(
        fs::read_dir(parent).unwrap().count(),
        0,
        "owned capture scratch leaked"
    );
}

#[test]
fn file_capture_copy_only_uses_full_bytes_from_offset_zero() {
    for length in [0, 1, 65536, 131079] {
        let bytes = vec![0x73; length];
        let (_root, source, parent) = fixture(&bytes);
        let mut input = File::open(&source).unwrap();
        input.seek(std::io::SeekFrom::End(0)).unwrap();
        let (mut stage, method) = StagedFile::capture_checked(
            &parent,
            input,
            CaptureMode::CopyOnly,
            Some((Digest::of_bytes(&bytes), length as u64)),
            [1; 16],
            &|| Ok(()),
        )
        .unwrap();
        assert!(matches!(method, CaptureMethod::Copy { clone_error: None }));
        let mut actual = Vec::new();
        stage.read_to_end(&mut actual).unwrap();
        assert_eq!(actual, bytes);
        assert_eq!(stage.len(), length as u64);
        drop(stage);
        assert_clean(&parent);
    }
}

#[test]
fn file_capture_partial_unsupported_clone_falls_back_without_tail() {
    let (_root, source, parent) = fixture(b"abc");
    let mut e = effects(&|| Ok(()));
    e.clone_file = |_, path, _| {
        fs::write(path, b"partial clone with a much longer tail")?;
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "controlled unsupported clone",
        ))
    };
    let (mut stage, method) = capture(&parent, &source, CaptureMode::CloneOrCopy, e).unwrap();
    match method {
        CaptureMethod::Copy {
            clone_error: Some(e),
        } => assert_eq!(e.kind(), io::ErrorKind::Unsupported),
        other => panic!("wrong actual method: {other:?}"),
    }
    let mut bytes = Vec::new();
    stage.read_to_end(&mut bytes).unwrap();
    assert_eq!(bytes, b"abc");
    assert_eq!(
        fs::read_dir(&parent).unwrap().count(),
        1,
        "failed clone reservation survived fallback"
    );
    drop(stage);
    assert_clean(&parent);
}

#[test]
fn file_capture_clone_resource_error_is_not_hidden_by_fallback() {
    let (_root, source, parent) = fixture(b"source");
    let mut e = effects(&|| Ok(()));
    e.clone_file = |_, path, _| {
        fs::write(path, b"partial")?;
        #[cfg(unix)]
        let code = libc::ENOSPC;
        #[cfg(windows)]
        let code = 112;
        Err(io::Error::from_raw_os_error(code))
    };
    let result = capture(&parent, &source, CaptureMode::CloneOrCopy, e);
    assert!(result.is_err(), "resource failure fell through to copy");
    let error = result.err().unwrap();
    assert_eq!(error.phase, Phase::Stage);
    #[cfg(unix)]
    assert_eq!(error.original_io().raw_os_error(), Some(libc::ENOSPC));
    #[cfg(windows)]
    assert_eq!(error.original_io().raw_os_error(), Some(112));
    assert_eq!(fs::read(&source).unwrap(), b"source");
    assert_clean(&parent);
}

#[test]
fn file_capture_claimed_clone_success_still_checks_digest_and_length() {
    let (_root, source, parent) = fixture(b"source");
    for wrong in [b"wrong!".as_slice(), b"short".as_slice()] {
        let mut e = effects(&|| Ok(()));
        e.clone_file = if wrong.len() == 6 {
            |_, path, _| {
                let mut f = fs::OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create_new(true)
                    .open(path)?;
                f.write_all(b"wrong!")?;
                Ok((f, CowRung::Clone))
            }
        } else {
            |_, path, _| {
                let mut f = fs::OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create_new(true)
                    .open(path)?;
                f.write_all(b"short")?;
                Ok((f, CowRung::Clone))
            }
        };
        let result = capture(&parent, &source, CaptureMode::CloneOrCopy, e);
        assert!(
            result.is_err(),
            "claimed clone skipped captured-byte verification"
        );
        let error = result.err().unwrap();
        assert_eq!(error.phase, Phase::Verify);
        assert_eq!(error.original_io().kind(), io::ErrorKind::InvalidData);
        assert_clean(&parent);
    }
}

#[test]
fn file_capture_precancel_does_not_allocate_or_invoke_clone() {
    let (_root, source, parent) = fixture(b"source");
    let mut e = effects(&|| Err(io::ErrorKind::Interrupted.into()));
    e.clone_file = |_, _, _| panic!("clone called after cancellation");
    let error = capture(&parent, &source, CaptureMode::CloneOrCopy, e)
        .err()
        .unwrap();
    assert_eq!(error.original_io().kind(), io::ErrorKind::Interrupted);
    assert_clean(&parent);
}

#[test]
fn file_capture_cancel_after_clone_and_during_hash_cleans_only_owned_file() {
    for stop in [3, 5] {
        let (_root, source, parent) = fixture(&vec![0x34; 65537]);
        let calls = Cell::new(0);
        let check = || {
            calls.set(calls.get() + 1);
            if calls.get() == stop {
                Err(io::ErrorKind::Interrupted.into())
            } else {
                Ok(())
            }
        };
        let error = capture(&parent, &source, CaptureMode::CloneOrCopy, effects(&check))
            .err()
            .unwrap();
        assert_eq!(
            error.phase,
            if stop == 3 {
                Phase::Stage
            } else {
                Phase::Verify
            }
        );
        assert_eq!(error.original_io().kind(), io::ErrorKind::Interrupted);
        assert_eq!(error.outcome, PublicationOutcome::NotPublished);
        assert_clean(&parent);
        assert_eq!(fs::read(&source).unwrap(), vec![0x34; 65537]);
    }
}

#[test]
fn file_capture_sync_error_remains_fatal_for_both_capture_modes() {
    for mode in [CaptureMode::CloneOrCopy, CaptureMode::CopyOnly] {
        let (_root, source, parent) = fixture(b"source");
        let mut e = effects(&|| Ok(()));
        e.sync = &|_| {
            Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "controlled flush error",
            ))
        };
        let result = capture(&parent, &source, mode, e);
        assert!(
            result.is_err(),
            "capture ignored file synchronization failure"
        );
        let error = result.err().unwrap();
        assert_eq!(error.phase, Phase::PayloadSync);
        assert_eq!(error.original_io().kind(), io::ErrorKind::BrokenPipe);
        assert_eq!(error.operation_id, [23; 16]);
        assert_clean(&parent);
    }
}

#[test]
fn file_capture_empty_source_never_claims_an_unexecuted_clone() {
    let (_root, source, parent) = fixture(b"");
    let mut e = effects(&|| Ok(()));
    e.clone_file = |_, _, _| panic!("zero-byte clone is not a native capability witness");
    let (stage, method) = capture(&parent, &source, CaptureMode::CloneOrCopy, e).unwrap();
    assert!(stage.is_empty());
    assert!(matches!(method, CaptureMethod::Copy { clone_error: None }));
    drop(stage);
    assert_clean(&parent);
}

#[test]
fn file_capture_native_or_reported_fallback_preserves_readonly_source() {
    let bytes = vec![0x5c; 131072];
    let (_root, source, parent) = fixture(&bytes);
    let permissions = fs::metadata(&source).unwrap().permissions();
    let mut readonly = permissions.clone();
    readonly.set_readonly(true);
    fs::set_permissions(&source, readonly).unwrap();
    let result = StagedFile::capture_checked(
        &parent,
        File::open(&source).unwrap(),
        CaptureMode::CloneOrCopy,
        Some((Digest::of_bytes(&bytes), bytes.len() as u64)),
        [1; 16],
        &|| Ok(()),
    );
    let still_readonly = fs::metadata(&source).unwrap().permissions().readonly();
    fs::set_permissions(&source, permissions).unwrap();
    assert!(still_readonly, "capture mutated source attributes");
    let (mut stage, method) = result.unwrap();
    eprintln!("actual file capture: {method:?}");
    #[cfg(target_os = "macos")]
    assert!(
        matches!(method, CaptureMethod::Native(CowRung::Clone)),
        "macOS native witness unavailable: {method:?}"
    );
    fs::write(&source, b"later source modification").unwrap();
    let mut actual = Vec::new();
    stage.read_to_end(&mut actual).unwrap();
    assert_eq!(actual, bytes, "capture must own independent bytes");
    drop(stage);
    assert_clean(&parent);
}

#[test]
fn file_capture_lease_composes_with_existing_publication_and_requalification() {
    use crate::store::foundation::{LeasedStagedFile, PreparationWork, StoreLocks, Wait};
    let root = TempDir::new().unwrap();
    let store = root.path().join("store");
    fs::create_dir(&store).unwrap();
    let input = root.path().join("source");
    fs::write(&input, b"live input").unwrap();
    let locks = StoreLocks::open_existing(&store).unwrap();
    let lease = locks.shared(Wait::Try).unwrap();
    let (stage, _method) = LeasedStagedFile::capture_file(
        &lease,
        File::open(&input).unwrap(),
        CaptureMode::CloneOrCopy,
        None,
        Wait::Try,
        [2; 16],
    )
    .unwrap();
    let proof = stage.publish(Wait::Try, [2; 16]).unwrap();
    assert_eq!(proof.digest(), Digest::of_bytes(b"live input"));
    assert_eq!(proof.work(), PreparationWork::Created);
    let digest = proof.digest();
    drop(lease);
    let hex = digest.to_hex();
    fs::remove_file(store.join("objects-ready").join(&hex[..2]).join(&hex[2..])).unwrap();
    let next = locks.shared(Wait::Try).unwrap();
    let proof = next.prepare_existing(digest, Wait::Try, [3; 16]).unwrap();
    assert_eq!(proof.work(), PreparationWork::Requalified);
    assert_eq!(proof.len(), 10);
}

#[test]
fn file_capture_fallback_classification_rejects_real_io_errors() {
    for kind in [
        io::ErrorKind::PermissionDenied,
        io::ErrorKind::BrokenPipe,
        io::ErrorKind::Interrupted,
        io::ErrorKind::NotFound,
        io::ErrorKind::InvalidData,
    ] {
        assert!(
            !native::can_fallback(&io::Error::from(kind)),
            "must not suppress {kind:?}"
        );
    }
    assert!(native::can_fallback(&io::ErrorKind::Unsupported.into()));
}
