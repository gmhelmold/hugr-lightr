use super::Digest;
use std::cell::Cell;
use std::io::{self, Read};

#[test]
fn checked_hash_matches_stream_and_buffer_boundaries() {
    for length in [0, 1, 65535, 65536, 65537, 200001] {
        let bytes: Vec<_> = (0..length).map(|n| (n % 251) as u8).collect();
        let mut checkpoints = 0;
        let actual = Digest::of_reader_checked(&mut bytes.as_slice(), || {
            checkpoints += 1;
            Ok(())
        })
        .unwrap();
        assert_eq!(actual, (Digest::of_bytes(&bytes), length as u64));
        assert_eq!(actual, Digest::of_reader(&mut bytes.as_slice()).unwrap());
        assert!(checkpoints >= 2);
    }
}

#[test]
fn checked_hash_checkpoint_interruption_is_terminal() {
    struct Never;
    impl Read for Never {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            panic!("read after cancellation");
        }
    }
    let mut checks = 0;
    let error = Digest::of_reader_checked(&mut Never, || {
        checks += 1;
        Err(io::Error::new(
            io::ErrorKind::Interrupted,
            "requested cancellation",
        ))
    })
    .unwrap_err();
    assert_eq!(checks, 1);
    assert_eq!(error.kind(), io::ErrorKind::Interrupted);
    assert_eq!(error.to_string(), "requested cancellation");
}

#[test]
fn checked_hash_checks_cancellation_before_retrying_interrupted_read() {
    struct Interrupt<'a>(&'a Cell<bool>, usize);
    impl Read for Interrupt<'_> {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            self.1 += 1;
            self.0.set(true);
            if self.1 == 1 {
                Err(io::ErrorKind::Interrupted.into())
            } else {
                Err(io::Error::from_raw_os_error(5))
            }
        }
    }
    let cancelled = Cell::new(false);
    let mut source = Interrupt(&cancelled, 0);
    let error = Digest::of_reader_checked(&mut source, || {
        if cancelled.get() {
            Err(io::ErrorKind::Interrupted.into())
        } else {
            Ok(())
        }
    })
    .unwrap_err();
    assert_eq!(source.1, 1);
    assert_eq!(error.kind(), io::ErrorKind::Interrupted);
}

#[test]
fn checked_hash_checks_after_eof() {
    struct End<'a>(&'a Cell<bool>);
    impl Read for End<'_> {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            self.0.set(true);
            Ok(0)
        }
    }
    let ended = Cell::new(false);
    let error = Digest::of_reader_checked(&mut End(&ended), || {
        if ended.get() {
            Err(io::Error::new(io::ErrorKind::TimedOut, "expired"))
        } else {
            Ok(())
        }
    })
    .unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::TimedOut);
    assert_eq!(error.to_string(), "expired");
}

#[test]
fn checked_hash_keeps_reader_os_error() {
    struct Broken;
    impl Read for Broken {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::from_raw_os_error(5))
        }
    }
    assert_eq!(
        Digest::of_reader_checked(&mut Broken, || Ok(()))
            .unwrap_err()
            .raw_os_error(),
        Some(5)
    );
}

#[test]
fn checked_hash_rejects_invalid_reader_count() {
    struct Invalid;
    impl Read for Invalid {
        fn read(&mut self, b: &mut [u8]) -> io::Result<usize> {
            Ok(b.len() + 1)
        }
    }
    assert_eq!(
        Digest::of_reader_checked(&mut Invalid, || Ok(()))
            .unwrap_err()
            .kind(),
        io::ErrorKind::InvalidData
    );
}

#[test]
fn checked_hash_retries_uncancelled_interrupted_reads() {
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
    assert_eq!(
        Digest::of_reader_checked(&mut Partial(0), || Ok(())).unwrap(),
        (Digest::of_bytes(&[42]), 1)
    );
}
