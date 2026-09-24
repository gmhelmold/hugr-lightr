use super::*;
use std::cell::Cell;
use std::fs;
use tempfile::TempDir;

fn stop() -> io::Error {
    io::Error::new(io::ErrorKind::Interrupted, "cancelled capture")
}
fn untouched(root: &Path) {
    assert_eq!(fs::read_dir(root).unwrap().count(), 1);
    assert_eq!(fs::read(root.join("sentinel")).unwrap(), b"preserved");
}
fn parent() -> TempDir {
    let root = TempDir::new().unwrap();
    fs::write(root.path().join("sentinel"), b"preserved").unwrap();
    root
}

#[test]
fn checkpoint_precancel_allocates_nothing() {
    let root = parent();
    let error = StagedFile::copy_checked(
        root.path(),
        &mut b"input".as_slice(),
        None,
        [31; 16],
        &|| Err(stop()),
    )
    .err()
    .unwrap();
    assert_eq!(error.phase, Phase::Stage);
    assert_eq!(error.original_io().kind(), io::ErrorKind::Interrupted);
    untouched(root.path());
}

#[test]
fn checkpoint_hash_cancellation_cleans_only_owned_stage() {
    struct Source<'a>(&'a Cell<bool>);
    impl Read for Source<'_> {
        fn read(&mut self, b: &mut [u8]) -> io::Result<usize> {
            if self.0.replace(true) {
                Ok(0)
            } else {
                b[0] = 19;
                Ok(1)
            }
        }
    }
    // The source raises an independent EOF flag only on the second read.
    struct WithEof<'a> {
        input: Source<'a>,
        eof: &'a Cell<bool>,
    }
    impl Read for WithEof<'_> {
        fn read(&mut self, b: &mut [u8]) -> io::Result<usize> {
            let n = self.input.read(b)?;
            if n == 0 {
                self.eof.set(true);
            }
            Ok(n)
        }
    }
    let root = parent();
    let seen = Cell::new(false);
    let eof = Cell::new(false);
    let after_eof = Cell::new(false);
    let mut input = WithEof {
        input: Source(&seen),
        eof: &eof,
    };
    let error = StagedFile::copy_with_checkpoints(
        root.path(),
        &mut input,
        None,
        [32; 16],
        |_| panic!("sync after hash cancellation"),
        &|| {
            if eof.get() && after_eof.replace(true) {
                Err(stop())
            } else {
                Ok(())
            }
        },
    )
    .err()
    .unwrap();
    assert_eq!(error.phase, Phase::Verify);
    assert_eq!(error.original_io().kind(), io::ErrorKind::Interrupted);
    untouched(root.path());
}

#[test]
fn checkpoint_sync_completion_cannot_hide_cancellation() {
    let root = parent();
    let cancelled = Cell::new(false);
    let error = StagedFile::copy_with_checkpoints(
        root.path(),
        &mut b"input".as_slice(),
        None,
        [33; 16],
        |file| {
            file.sync_all()?;
            cancelled.set(true);
            Ok(())
        },
        &|| if cancelled.get() { Err(stop()) } else { Ok(()) },
    )
    .err()
    .unwrap();
    assert_eq!(error.phase, Phase::PayloadSync);
    assert_eq!(error.original_io().kind(), io::ErrorKind::Interrupted);
    assert_eq!(error.outcome, PublicationOutcome::NotPublished);
    untouched(root.path());
}

#[test]
fn checkpoint_write_interruption_is_not_blindly_retried() {
    struct Writer<'a>(&'a Cell<bool>, usize);
    impl Write for Writer<'_> {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            self.1 += 1;
            self.0.set(true);
            if self.1 == 1 {
                Err(io::ErrorKind::Interrupted.into())
            } else {
                Err(io::Error::from_raw_os_error(5))
            }
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let cancelled = Cell::new(false);
    let mut writer = Writer(&cancelled, 0);
    let error = write_checked(&mut writer, b"input", &|| {
        if cancelled.get() {
            Err(stop())
        } else {
            Ok(())
        }
    })
    .unwrap_err();
    assert_eq!(writer.1, 1);
    assert_eq!(error.kind(), io::ErrorKind::Interrupted);
}

#[test]
fn checkpoint_short_write_cancel_stops_remaining_bytes() {
    struct Writer<'a>(&'a Cell<bool>, Vec<u8>);
    impl Write for Writer<'_> {
        fn write(&mut self, b: &[u8]) -> io::Result<usize> {
            self.1.push(b[0]);
            self.0.set(true);
            Ok(1)
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let cancelled = Cell::new(false);
    let mut writer = Writer(&cancelled, Vec::new());
    let error = write_checked(&mut writer, b"input", &|| {
        if cancelled.get() {
            Err(stop())
        } else {
            Ok(())
        }
    })
    .unwrap_err();
    assert_eq!(writer.1, b"i");
    assert_eq!(error.kind(), io::ErrorKind::Interrupted);
}

#[test]
fn checkpoint_writer_errors_and_invalid_counts_remain_explicit() {
    struct Writer(u8);
    impl Write for Writer {
        fn write(&mut self, b: &[u8]) -> io::Result<usize> {
            match self.0 {
                0 => Ok(0),
                1 => Ok(b.len() + 1),
                _ => Err(io::Error::from_raw_os_error(5)),
            }
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    assert_eq!(
        write_checked(&mut Writer(0), b"input", &|| Ok(()))
            .unwrap_err()
            .kind(),
        io::ErrorKind::WriteZero
    );
    assert_eq!(
        write_checked(&mut Writer(1), b"input", &|| Ok(()))
            .unwrap_err()
            .kind(),
        io::ErrorKind::InvalidData
    );
    assert_eq!(
        write_checked(&mut Writer(2), b"input", &|| Ok(()))
            .unwrap_err()
            .raw_os_error(),
        Some(5)
    );
}

#[test]
fn checkpoint_short_writes_and_os_interrupts_still_complete() {
    struct Writer(usize, Vec<u8>);
    impl Write for Writer {
        fn write(&mut self, b: &[u8]) -> io::Result<usize> {
            self.0 += 1;
            if self.0 % 2 == 1 {
                Err(io::ErrorKind::Interrupted.into())
            } else {
                self.1.push(b[0]);
                Ok(1)
            }
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let mut writer = Writer(0, Vec::new());
    write_checked(&mut writer, b"input", &|| Ok(())).unwrap();
    assert_eq!(writer.1, b"input");
}
