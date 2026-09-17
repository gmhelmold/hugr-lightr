use super::{install_metadata, Assurance, InstallStage as Stage, InstallVisibility as Visible, MAX_METADATA_BYTES};
use super::publish::{install_with, InstallIo, NativeIo};
use std::{fs::{self, File}, io::{self, Seek, SeekFrom, Write}, path::Path};
use tempfile::TempDir;

#[derive(Clone, Copy)]
enum Fault { Flush, RenameBefore, RenameAfter, Directory, Corrupt, Truncate, ExtraChild }
struct FaultIo(Fault);
fn injected() -> io::Error { io::Error::from_raw_os_error(5) }
impl InstallIo for FaultIo {
    fn sync_file(&self, file: &File) -> io::Result<()> {
        match self.0 {
            Fault::Flush => Err(injected()),
            Fault::Corrupt => {
                let mut file = file;
                file.seek(SeekFrom::Start(0))?; file.write_all(b"X")?;
                NativeIo.sync_file(file)
            }
            Fault::Truncate => { file.set_len(0)?; NativeIo.sync_file(file) }
            _ => NativeIo.sync_file(file),
        }
    }
    fn rename(&self, source: &Path, destination: &Path) -> io::Result<()> {
        if matches!(self.0, Fault::RenameBefore) { return Err(injected()); }
        if matches!(self.0, Fault::ExtraChild) { fs::write(source.parent().unwrap().join("unknown"), b"retain")?; }
        NativeIo.rename(source, destination)?;
        if matches!(self.0, Fault::RenameAfter) { return Err(injected()); }
        Ok(())
    }
    fn sync_directory(&self, parent: &Path) -> io::Result<()> {
        if matches!(self.0, Fault::Directory) { return Err(injected()); }
        NativeIo.sync_directory(parent)
    }
}

#[test]
fn checked_metadata_replaces_bytes_and_cleans_only_own_staging() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("foreign"), b"untouched").unwrap();
    for bytes in [b"old".as_slice(), b"newer".as_slice(), b"".as_slice()] {
        let report = install_metadata(dir.path(), "entry", bytes, Assurance::native().unwrap()).unwrap();
        assert_eq!(report.length, bytes.len() as u64);
        assert_eq!(report.assurance, Assurance::native().unwrap());
        assert_eq!(fs::read(dir.path().join("entry")).unwrap(), bytes);
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 2);
        assert_eq!(fs::read(dir.path().join("foreign")).unwrap(), b"untouched");
    }
}

#[test]
fn checked_metadata_flush_failure_preserves_prior_destination() {
    let dir = TempDir::new().unwrap(); fs::write(dir.path().join("entry"), b"old").unwrap();
    let err = install_with(&FaultIo(Fault::Flush), dir.path(), "entry", b"new", Assurance::native().unwrap()).unwrap_err();
    assert_eq!(err.stage(), Stage::SyncFile);
    assert_eq!(err.visibility(), Visible::Unchanged);
    assert_eq!(err.io_error().raw_os_error(), Some(5));
    assert_eq!(fs::read(dir.path().join("entry")).unwrap(), b"old");
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[test]
fn checked_metadata_verifies_actual_staged_bytes() {
    for fault in [Fault::Corrupt, Fault::Truncate] {
        let dir = TempDir::new().unwrap(); fs::write(dir.path().join("entry"), b"old").unwrap();
        let err = install_with(&FaultIo(fault), dir.path(), "entry", b"new", Assurance::native().unwrap()).unwrap_err();
        assert_eq!(err.stage(), Stage::Verify);
        assert_eq!(err.visibility(), Visible::Unchanged);
        assert_eq!(fs::read(dir.path().join("entry")).unwrap(), b"old");
    }
}

#[test]
fn rename_errors_never_claim_destination_unchanged() {
    for (fault, expected) in [(Fault::RenameBefore, b"old"), (Fault::RenameAfter, b"new")] {
        let dir = TempDir::new().unwrap(); fs::write(dir.path().join("entry"), b"old").unwrap();
        let err = install_with(&FaultIo(fault), dir.path(), "entry", b"new", Assurance::native().unwrap()).unwrap_err();
        assert_eq!(err.stage(), Stage::Rename);
        assert_eq!(err.visibility(), Visible::MayBeInstalled);
        assert_eq!(fs::read(dir.path().join("entry")).unwrap(), expected);
    }
}

#[test]
fn directory_failure_keeps_installed_bytes_and_reports_uncertainty() {
    let dir = TempDir::new().unwrap(); fs::write(dir.path().join("entry"), b"old").unwrap();
    let err = install_with(&FaultIo(Fault::Directory), dir.path(), "entry", b"new", Assurance::native().unwrap()).unwrap_err();
    assert_eq!(err.stage(), Stage::SyncDirectory);
    assert_eq!(err.visibility(), Visible::InstalledUnconfirmed);
    assert_eq!(fs::read(dir.path().join("entry")).unwrap(), b"new");
    // A retry must perform the write/flush path; existence is not a fast path.
    let retry = install_with(&FaultIo(Fault::Flush), dir.path(), "entry", b"new", Assurance::native().unwrap()).unwrap_err();
    assert_eq!(retry.stage(), Stage::SyncFile);
}

#[test]
fn cleanup_error_retains_confirmation_and_unknown_child() {
    let dir = TempDir::new().unwrap();
    let err = install_with(&FaultIo(Fault::ExtraChild), dir.path(), "entry", b"new", Assurance::native().unwrap()).unwrap_err();
    assert_eq!(err.stage(), Stage::Cleanup);
    assert_eq!(err.visibility(), Visible::Confirmed);
    assert_eq!(fs::read(dir.path().join("entry")).unwrap(), b"new");
    let leftovers: Vec<_> = fs::read_dir(dir.path()).unwrap().filter_map(Result::ok).filter(|p| p.file_type().unwrap().is_dir()).collect();
    assert_eq!(leftovers.len(), 1);
    assert_eq!(fs::read(leftovers[0].path().join("unknown")).unwrap(), b"retain");
}

#[test]
fn preflight_rejects_names_sizes_and_profile_without_writes() {
    let dir = TempDir::new().unwrap();
    for name in ["", "..", "../entry", "a/b", "a\\b", "a:b", "/entry", "C:entry"] {
        assert_eq!(install_metadata(dir.path(), name, b"x", Assurance::native().unwrap()).unwrap_err().stage(), Stage::Preflight);
    }
    let other = if cfg!(unix) { Assurance::WindowsFile } else { Assurance::UnixFileDirectory };
    assert_eq!(install_metadata(dir.path(), "entry", b"x", other).unwrap_err().io_error().kind(), io::ErrorKind::Unsupported);
    assert!(install_metadata(dir.path(), "entry", &vec![0; MAX_METADATA_BYTES + 1], Assurance::native().unwrap()).is_err());
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
}

#[test]
fn missing_parent_and_directory_destination_are_not_repaired() {
    let dir = TempDir::new().unwrap();
    assert!(install_metadata(&dir.path().join("missing"), "entry", b"x", Assurance::native().unwrap()).is_err());
    assert!(!dir.path().join("missing").exists());
    fs::create_dir(dir.path().join("entry")).unwrap();
    assert!(install_metadata(dir.path(), "entry", b"x", Assurance::native().unwrap()).is_err());
    assert!(dir.path().join("entry").is_dir());
}

#[test]
fn bounded_large_metadata_roundtrips_without_payload_sized_readback() {
    let dir = TempDir::new().unwrap(); let bytes = vec![0x5a; MAX_METADATA_BYTES];
    install_metadata(dir.path(), "entry", &bytes, Assurance::native().unwrap()).unwrap();
    assert_eq!(fs::read(dir.path().join("entry")).unwrap(), bytes);
}

#[test]
fn parallel_metadata_installs_use_distinct_owned_staging() {
    let dir = TempDir::new().unwrap();
    std::thread::scope(|scope| {
        let mut jobs = Vec::new();
        for byte in 0..8u8 {
            let parent = dir.path();
            jobs.push(scope.spawn(move || install_metadata(parent, &format!("entry{byte}"), &[byte; 1024], Assurance::native().unwrap()).unwrap()));
        }
        for job in jobs { job.join().unwrap(); }
    });
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 8);
    for byte in 0..8u8 { assert_eq!(fs::read(dir.path().join(format!("entry{byte}"))).unwrap(), vec![byte; 1024]); }
}

#[cfg(unix)]
#[test]
fn metadata_rejects_symlink_destination_and_preserves_target() {
    let dir = TempDir::new().unwrap(); fs::write(dir.path().join("target"), b"keep").unwrap();
    std::os::unix::fs::symlink("target", dir.path().join("entry")).unwrap();
    assert!(install_metadata(dir.path(), "entry", b"new", Assurance::native().unwrap()).is_err());
    assert_eq!(fs::read(dir.path().join("target")).unwrap(), b"keep");
}
