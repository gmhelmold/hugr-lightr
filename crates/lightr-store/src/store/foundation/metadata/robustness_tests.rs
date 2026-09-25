//! Regression witnesses confined to temporary directories and scoped I/O seams.
use super::publish::{install_with, InstallIo, NativeIo};
use super::{Assurance, InstallStage as Stage, InstallVisibility as Visible, ReadinessFrame};
use std::{
    cell::Cell,
    fs::{self, File},
    io::{self, Read},
    path::Path,
};
use tempfile::TempDir;

struct ReplaceStage<'a>(&'a Path);
impl InstallIo for ReplaceStage<'_> {
    fn sync_file(&self, file: &File) -> io::Result<()> {
        NativeIo.sync_file(file)?;
        let mut stages = fs::read_dir(self.0)?
            .collect::<io::Result<Vec<_>>>()?
            .into_iter()
            .filter(|entry| entry.file_name().to_string_lossy().starts_with(".tmp-"));
        let stage = stages.next().expect("one operation owns staging");
        assert!(stages.next().is_none());
        let payload = stage.path().join("payload");
        // The open handle still contains the verified bytes, but its name now
        // denotes a different object. No path outside this fixture is touched.
        fs::rename(&payload, self.0.join("retained-original"))?;
        fs::write(&payload, b"not-the-verified-object")
    }
    fn rename(&self, source: &Path, destination: &Path) -> io::Result<()> {
        NativeIo.rename(source, destination)
    }
    fn sync_directory(&self, parent: &Path) -> io::Result<()> {
        NativeIo.sync_directory(parent)
    }
}

#[test]
fn metadata_replaced_staging_name_cannot_install_other_bytes() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("entry"), b"previous").unwrap();
    let error = install_with(
        &ReplaceStage(dir.path()),
        dir.path(),
        "entry",
        b"verified",
        Assurance::native(),
    )
    .unwrap_err();
    assert_eq!(error.stage(), Stage::Verify);
    assert_eq!(error.visibility(), Visible::Unchanged);
    assert_eq!(error.io_error().kind(), io::ErrorKind::InvalidData);
    assert_eq!(fs::read(dir.path().join("entry")).unwrap(), b"previous");
    assert_eq!(
        fs::read(dir.path().join("retained-original")).unwrap(),
        b"verified"
    );
}

struct InvalidCount;
impl Read for InvalidCount {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        Ok(output.len() + 1)
    }
}

#[test]
fn readiness_capture_rejects_invalid_reader_count_without_panicking() {
    let outcome = std::panic::catch_unwind(|| ReadinessFrame::read_from(InvalidCount));
    assert!(
        outcome.is_ok(),
        "invalid reader count must produce an error, not panic"
    );
    assert_eq!(
        outcome.unwrap().unwrap_err().kind(),
        io::ErrorKind::InvalidData
    );
}

struct CleanupSync(Cell<usize>);
impl InstallIo for CleanupSync {
    fn sync_file(&self, file: &File) -> io::Result<()> {
        NativeIo.sync_file(file)
    }
    fn rename(&self, source: &Path, destination: &Path) -> io::Result<()> {
        NativeIo.rename(source, destination)
    }
    fn sync_directory(&self, parent: &Path) -> io::Result<()> {
        self.0.set(self.0.get() + 1);
        if self.0.get() == 2 {
            Err(io::Error::from_raw_os_error(5))
        } else {
            NativeIo.sync_directory(parent)
        }
    }
}

#[test]
fn metadata_final_cleanup_barrier_failure_preserves_confirmation() {
    let dir = TempDir::new().unwrap();
    let ops = CleanupSync(Cell::new(0));
    let error =
        install_with(&ops, dir.path(), "entry", b"confirmed", Assurance::native()).unwrap_err();
    assert_eq!(error.stage(), Stage::Cleanup);
    assert_eq!(error.visibility(), Visible::Confirmed);
    assert_eq!(error.io_error().raw_os_error(), Some(5));
    assert_eq!(ops.0.get(), 2);
    assert_eq!(fs::read(dir.path().join("entry")).unwrap(), b"confirmed");
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
}
