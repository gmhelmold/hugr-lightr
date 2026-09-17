//! Cooperative cancellation on the real reader-to-staging path, not sleeps.
use super::{Cancellation, Phase, PublicationOutcome, StoreLocks, Wait};
use std::fs;
use std::io::{self, Read};
use std::time::{Duration, Instant};
use tempfile::TempDir;

struct CancellingReader<'a> {
    cancellation: &'a Cancellation,
    calls: usize,
    interrupted: bool,
    eof: bool,
}
impl Read for CancellingReader<'_> {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        self.calls += 1;
        if self.calls == 1 {
            self.cancellation.cancel();
            if self.interrupted {
                return Err(io::ErrorKind::Interrupted.into());
            }
            if self.eof {
                return Ok(0);
            }
            bytes[0] = 17;
            return Ok(1);
        }
        // Bounded negative oracle: the old implementation must fail, not hang.
        Err(io::Error::from_raw_os_error(5))
    }
}

fn run_cancelled_read(interrupted: bool, eof: bool) {
    let root = TempDir::new().unwrap();
    let locks = StoreLocks::open_existing(root.path()).unwrap();
    let lease = locks.shared(Wait::Try).unwrap();
    let staging = root.path().join(".si01-staging");
    fs::create_dir(&staging).unwrap();
    fs::write(staging.join("existing"), b"retained").unwrap();
    let cancellation = Cancellation::default();
    let mut reader = CancellingReader {
        cancellation: &cancellation,
        calls: 0,
        interrupted,
        eof,
    };
    let error = lease
        .prepare_reader(
            &mut reader,
            None,
            Wait::Until {
                deadline: Instant::now() + Duration::from_secs(5),
                cancellation: &cancellation,
            },
            [23; 16],
        )
        .unwrap_err();
    assert_eq!(
        reader.calls, 1,
        "cancelled capture must not call the source again"
    );
    assert_eq!(error.original_io().kind(), io::ErrorKind::Interrupted);
    assert_eq!(error.phase, Phase::Stage);
    assert_eq!(error.outcome, PublicationOutcome::NotPublished);
    assert_eq!(error.operation_id, [23; 16]);
    assert_eq!(fs::read(staging.join("existing")).unwrap(), b"retained");
    assert_eq!(fs::read_dir(staging).unwrap().count(), 1);
    assert!(!root.path().join("objects").exists());
    assert!(!root.path().join("objects-ready").exists());
}

#[test]
fn capture_cancellation_after_read_stops_before_a_second_read() {
    run_cancelled_read(false, false);
}
#[test]
fn capture_cancellation_is_not_retried_as_os_interrupted() {
    run_cancelled_read(true, false);
}
#[test]
fn capture_cancellation_at_eof_cannot_finish_staging() {
    run_cancelled_read(false, true);
}

#[test]
fn capture_cancellation_precancel_does_not_create_staging() {
    struct Never;
    impl Read for Never {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            panic!("unexpected source access");
        }
    }
    let root = TempDir::new().unwrap();
    let locks = StoreLocks::open_existing(root.path()).unwrap();
    let lease = locks.shared(Wait::Try).unwrap();
    let cancellation = Cancellation::default();
    cancellation.cancel();
    let error = super::LeasedStagedFile::copy_with_wait(
        &lease,
        &mut Never,
        None,
        Wait::Until {
            deadline: Instant::now() + Duration::from_secs(5),
            cancellation: &cancellation,
        },
        [24; 16],
    )
    .err()
    .unwrap();
    assert_eq!(error.original_io().kind(), io::ErrorKind::Interrupted);
    assert!(!root.path().join(".si01-staging").exists());
}

#[test]
fn capture_cancellation_during_capture_does_not_poison_the_lease() {
    let root = TempDir::new().unwrap();
    let locks = StoreLocks::open_existing(root.path()).unwrap();
    let lease = locks.shared(Wait::Try).unwrap();
    let cancellation = Cancellation::default();
    let mut reader = CancellingReader {
        cancellation: &cancellation,
        calls: 0,
        interrupted: false,
        eof: false,
    };
    let error = lease
        .prepare_reader(
            &mut reader,
            None,
            Wait::Until {
                deadline: Instant::now() + Duration::from_secs(5),
                cancellation: &cancellation,
            },
            [25; 16],
        )
        .unwrap_err();
    assert_eq!(error.original_io().kind(), io::ErrorKind::Interrupted);
    assert_eq!(reader.calls, 1);
    let result = lease
        .prepare_reader(&mut b"fresh".as_slice(), None, Wait::Try, [26; 16])
        .unwrap();
    assert_eq!(result.work(), super::PreparationWork::Created);
    assert_eq!(result.digest(), lightr_core::Digest::of_bytes(b"fresh"));
    assert_eq!(result.len(), 5);
}

#[test]
fn capture_cancellation_keeps_real_read_error_when_both_occur() {
    struct Broken<'a>(&'a Cancellation);
    impl Read for Broken<'_> {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            self.0.cancel();
            Err(io::Error::from_raw_os_error(5))
        }
    }
    let root = TempDir::new().unwrap();
    let locks = StoreLocks::open_existing(root.path()).unwrap();
    let lease = locks.shared(Wait::Try).unwrap();
    let cancellation = Cancellation::default();
    let error = lease
        .prepare_reader(
            &mut Broken(&cancellation),
            None,
            Wait::Until {
                deadline: Instant::now() + Duration::from_secs(5),
                cancellation: &cancellation,
            },
            [27; 16],
        )
        .unwrap_err();
    assert_eq!(error.original_io().raw_os_error(), Some(5));
    assert_eq!(error.operation_id, [27; 16]);
    assert_eq!(error.outcome, PublicationOutcome::NotPublished);
    assert_eq!(
        fs::read_dir(root.path().join(".si01-staging"))
            .unwrap()
            .count(),
        0
    );
    assert!(!root.path().join("objects").exists());
}

#[test]
fn capture_cancellation_uncancelled_read_interruptions_preserve_bytes() {
    struct Partial(usize);
    impl Read for Partial {
        fn read(&mut self, b: &mut [u8]) -> io::Result<usize> {
            self.0 += 1;
            match self.0 {
                1 | 3 => Err(io::ErrorKind::Interrupted.into()),
                2 => {
                    b[0] = 42;
                    Ok(1)
                }
                _ => Ok(0),
            }
        }
    }
    let root = TempDir::new().unwrap();
    let locks = StoreLocks::open_existing(root.path()).unwrap();
    let lease = locks.shared(Wait::Try).unwrap();
    let cancellation = Cancellation::default();
    let mut reader = Partial(0);
    let proof = lease
        .prepare_reader(
            &mut reader,
            None,
            Wait::Until {
                deadline: Instant::now() + Duration::from_secs(5),
                cancellation: &cancellation,
            },
            [28; 16],
        )
        .unwrap();
    assert_eq!(reader.0, 4);
    assert_eq!(proof.len(), 1);
    assert_eq!(proof.digest(), lightr_core::Digest::of_bytes(&[42]));
}

#[test]
fn capture_cancellation_expired_deadline_does_not_read_or_allocate() {
    struct Never;
    impl Read for Never {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            panic!("read after deadline");
        }
    }
    let root = TempDir::new().unwrap();
    let locks = StoreLocks::open_existing(root.path()).unwrap();
    let lease = locks.shared(Wait::Try).unwrap();
    let cancellation = Cancellation::default();
    let error = super::LeasedStagedFile::copy_with_wait(
        &lease,
        &mut Never,
        None,
        Wait::Until {
            deadline: Instant::now() - Duration::from_secs(1),
            cancellation: &cancellation,
        },
        [29; 16],
    )
    .err()
    .unwrap();
    assert_eq!(error.original_io().kind(), io::ErrorKind::TimedOut);
    assert!(!root.path().join(".si01-staging").exists());
}
