use super::*;
use std::fs;
use tempfile::tempdir;

fn assert_only_sentinel(parent: &Path) {
    let entries: Vec<_> = fs::read_dir(parent)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(entries, [std::ffi::OsString::from("existing")]);
    assert_eq!(fs::read(parent.join("existing")).unwrap(), b"keep");
}

#[test]
fn stage_roundtrips_empty_and_multiple_buffers() {
    let parent = tempdir().unwrap();
    for length in [0, 1, 65535, 65536, 65537, 200000] {
        let bytes: Vec<u8> = (0..length).map(|n| (n % 251) as u8).collect();
        let mut stage = StagedFile::copy_from_reader(
            parent.path(),
            &mut bytes.as_slice(),
            Some((Digest::of_bytes(&bytes), length as u64)),
            [1; 16],
        )
        .unwrap();
        assert_eq!(stage.digest(), Digest::of_bytes(&bytes));
        assert_eq!(stage.len(), length as u64);
        assert_eq!(stage.is_empty(), length == 0);
        let mut restored = Vec::new();
        stage.read_to_end(&mut restored).unwrap();
        assert_eq!(restored, bytes);
        stage.rewind().unwrap();
        assert_eq!(Digest::of_reader(&mut stage).unwrap().0, stage.digest());
        let payload = stage._owned.payload();
        assert!(payload.is_file());
        drop(stage);
        assert!(!payload.exists());
        assert_eq!(fs::read_dir(parent.path()).unwrap().count(), 0);
    }
}

#[test]
fn stage_rejects_digest_and_length_mismatch_before_sync() {
    let parent = tempdir().unwrap();
    fs::write(parent.path().join("existing"), b"keep").unwrap();
    for expected in [
        (Digest::of_bytes(b"wrong"), 4),
        (Digest::of_bytes(b"data"), 5),
    ] {
        let result = StagedFile::copy_with_sync(
            parent.path(),
            &mut b"data".as_slice(),
            Some(expected),
            [2; 16],
            |_| panic!("synchronized unverified bytes"),
        );
        let error = result.err().expect("accepted identity mismatch");
        assert_eq!(error.phase, Phase::Verify);
        assert_eq!(error.original_io().kind(), io::ErrorKind::InvalidData);
        assert_only_sentinel(parent.path());
    }
}

#[test]
fn stage_retains_sync_failure_and_preserves_existing_files() {
    let parent = tempdir().unwrap();
    fs::write(parent.path().join("existing"), b"keep").unwrap();
    let result = StagedFile::copy_with_sync(
        parent.path(),
        &mut b"data".as_slice(),
        None,
        [3; 16],
        |file| {
            assert_eq!(file.metadata()?.len(), 4);
            Err(io::Error::from_raw_os_error(5))
        },
    );
    let error = result.err().expect("ignored synchronization failure");
    assert_eq!(error.phase, Phase::PayloadSync);
    assert_eq!(error.outcome, PublicationOutcome::NotPublished);
    assert_eq!(error.original_io().raw_os_error(), Some(5));
    assert_eq!(error.operation_id, [3; 16]);
    assert_only_sentinel(parent.path());
}

#[test]
fn stage_read_failure_removes_only_its_own_partial_payload() {
    struct Partial(bool);
    impl Read for Partial {
        fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
            if self.0 {
                self.0 = false;
                bytes[0] = 9;
                Ok(1)
            } else {
                Err(io::Error::from_raw_os_error(5))
            }
        }
    }
    let parent = tempdir().unwrap();
    fs::write(parent.path().join("existing"), b"keep").unwrap();
    let result = StagedFile::copy_from_reader(parent.path(), &mut Partial(true), None, [4; 16]);
    let error = result.err().expect("accepted failed source read");
    assert_eq!(error.phase, Phase::Stage);
    assert_eq!(error.original_io().raw_os_error(), Some(5));
    assert_only_sentinel(parent.path());
}

#[test]
fn stage_does_not_create_missing_parent() {
    let parent = tempdir().unwrap();
    let missing = parent.path().join("missing");
    let result = StagedFile::copy_from_reader(&missing, &mut b"data".as_slice(), None, [5; 16]);
    assert_eq!(result.err().unwrap().phase, Phase::Stage);
    assert!(!missing.exists());
}

#[test]
fn stage_handles_are_independent_and_source_is_unchanged() {
    let parent = tempdir().unwrap();
    let source = parent.path().join("source");
    fs::write(&source, b"original").unwrap();
    let first = StagedFile::copy_from_reader(
        parent.path(),
        &mut File::open(&source).unwrap(),
        None,
        [6; 16],
    )
    .unwrap();
    let mut second = StagedFile::copy_from_reader(
        parent.path(),
        &mut File::open(&source).unwrap(),
        None,
        [7; 16],
    )
    .unwrap();
    assert_ne!(first._owned.payload(), second._owned.payload());
    assert_eq!(fs::read(&source).unwrap(), b"original");
    fs::write(&source, b"changed").unwrap();
    drop(first);
    let mut bytes = Vec::new();
    second.read_to_end(&mut bytes).unwrap();
    assert_eq!(bytes, b"original");
    assert_eq!(fs::read(&source).unwrap(), b"changed");
}

#[test]
fn bounded_copy_handles_interrupt_short_write_and_write_error() {
    struct Interrupted(bool);
    impl Read for Interrupted {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            if self.0 {
                self.0 = false;
                Err(io::ErrorKind::Interrupted.into())
            } else {
                Ok(0)
            }
        }
    }
    struct Short(Vec<u8>);
    impl Write for Short {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.0.push(bytes[0]);
            Ok(1)
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let mut output = Short(Vec::new());
    assert_eq!(
        copy_bounded(&mut Interrupted(true), &mut output).unwrap(),
        0
    );
    assert_eq!(
        copy_bounded(&mut b"data".as_slice(), &mut output).unwrap(),
        4
    );
    assert_eq!(output.0, b"data");
    struct Broken;
    impl Write for Broken {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::from_raw_os_error(5))
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    assert_eq!(
        copy_bounded(&mut b"data".as_slice(), &mut Broken)
            .unwrap_err()
            .raw_os_error(),
        Some(5)
    );
}
