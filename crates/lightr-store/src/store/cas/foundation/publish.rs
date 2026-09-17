//! Checked, phase-aware metadata replacement; not a multi-file transaction.

use super::{Assurance, InstallFailure, InstallStage as Stage, InstallVisibility as Visible};
use crate::store::cas::OwnedTemp;
use std::{fs::{self, File, OpenOptions}, io::{self, Read, Seek, SeekFrom, Write}, path::Path};

/// Accepted maximum pending-record body plus framing. Never used for payloads.
pub const MAX_METADATA_BYTES: usize = 1_048_576 + 44;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MetadataInstall {
    pub length: u64,
    pub assurance: Assurance,
}

/// Install bounded metadata in an already existing private directory.
///
/// The caller MUST retain its validated topology, live Store/resource lease
/// and relevant key lock for this call and all surrounding transaction steps.
/// This low-level primitive neither creates a lease nor issues PreparedObject.
/// No existing Store/CLI method calls it before coordinated SI-03 activation.
///
/// A successful return confirms this one replacement at the stated profile;
/// WindowsFile is NOT directory-entry power-loss durability. Failures preserve
/// the original OS error, physical visibility and stage. No destination unlink,
/// fsync-only recovery, automatic retry or publication rollback is performed.
pub fn install_metadata(parent: &Path, name: &str, bytes: &[u8], required: Assurance) -> Result<MetadataInstall, InstallFailure> {
    install_with(&NativeIo, parent, name, bytes, required)
}

pub(super) trait InstallIo {
    fn sync_file(&self, file: &File) -> io::Result<()>;
    fn rename(&self, source: &Path, destination: &Path) -> io::Result<()>;
    fn sync_directory(&self, parent: &Path) -> io::Result<()>;
}

pub(super) struct NativeIo;

impl InstallIo for NativeIo {
    fn sync_file(&self, file: &File) -> io::Result<()> {
        // File is opened read/write. On Windows this uses a flush-capable handle;
        // unlike the legacy FFI call, its failure cannot be silently discarded.
        file.sync_all()
    }
    fn rename(&self, source: &Path, destination: &Path) -> io::Result<()> {
        fs::rename(source, destination)
    }
    fn sync_directory(&self, parent: &Path) -> io::Result<()> {
        #[cfg(unix)]
        { File::open(parent)?.sync_all() }
        #[cfg(windows)]
        { let _ = parent; Ok(()) }
        #[cfg(not(any(unix, windows)))]
        { let _ = parent; Err(io::Error::new(io::ErrorKind::Unsupported, "no directory sync profile")) }
    }
}

pub(super) fn install_with(ops: &impl InstallIo, parent: &Path, name: &str, bytes: &[u8], required: Assurance) -> Result<MetadataInstall, InstallFailure> {
    let failure = |stage, visibility, error| InstallFailure::new(stage, visibility, parent, error);
    if name.is_empty() || !name.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_') || name.len() > 128 {
        return Err(failure(Stage::Preflight, Visible::Unchanged, io::Error::new(io::ErrorKind::InvalidInput, "metadata name must be one bounded ASCII component")));
    }
    let destination = parent.join(name);
    let fail = |stage, visibility, error| InstallFailure::new(stage, visibility, &destination, error);
    let assurance = Assurance::native().map_err(|e| fail(Stage::Preflight, Visible::Unchanged, e))?;
    if assurance != required {
        return Err(fail(Stage::Preflight, Visible::Unchanged, io::Error::new(io::ErrorKind::Unsupported, "requested synchronization profile is unavailable")));
    }
    if bytes.len() > MAX_METADATA_BYTES {
        return Err(fail(Stage::Preflight, Visible::Unchanged, io::Error::new(io::ErrorKind::InvalidInput, "metadata exceeds bounded protocol limit")));
    }
    let metadata = fs::symlink_metadata(parent).map_err(|e| fail(Stage::Preflight, Visible::Unchanged, e))?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(fail(Stage::Preflight, Visible::Unchanged, io::Error::new(io::ErrorKind::InvalidInput, "metadata parent must be a private real directory")));
    }
    match fs::symlink_metadata(&destination) {
        Ok(meta) if !meta.is_file() || meta.file_type().is_symlink() => {
            return Err(fail(Stage::Preflight, Visible::Unchanged, io::Error::new(io::ErrorKind::InvalidInput, "metadata destination is not a regular file")));
        }
        Ok(_) => (),
        Err(e) if e.kind() == io::ErrorKind::NotFound => (),
        Err(e) => return Err(fail(Stage::Preflight, Visible::Unchanged, e)),
    }
    let staging = OwnedTemp::new(parent, "si01-metadata").map_err(|e| fail(Stage::Allocate, Visible::Unchanged, e))?;
    let temporary = staging.payload();
    let mut options = OpenOptions::new();
    options.read(true).write(true).create_new(true);
    #[cfg(unix)]
    { use std::os::unix::fs::OpenOptionsExt; options.mode(0o600); }
    let mut file = options.open(&temporary).map_err(|e| fail(Stage::Allocate, Visible::Unchanged, e))?;
    file.write_all(bytes).map_err(|e| fail(Stage::Write, Visible::Unchanged, e))?;
    ops.sync_file(&file).map_err(|e| fail(Stage::SyncFile, Visible::Unchanged, e))?;
    file.seek(SeekFrom::Start(0)).map_err(|e| fail(Stage::Verify, Visible::Unchanged, e))?;
    // Bounded comparison reads back the actual staged bytes, including length.
    let mut offset = 0usize;
    let mut buffer = [0u8; 8192];
    loop {
        let count = file.read(&mut buffer).map_err(|e| fail(Stage::Verify, Visible::Unchanged, e))?;
        if count == 0 { break; }
        if offset.checked_add(count).is_none_or(|end| end > bytes.len()) || bytes[offset..offset + count] != buffer[..count] {
            return Err(fail(Stage::Verify, Visible::Unchanged, io::Error::new(io::ErrorKind::InvalidData, "staged metadata differs from supplied bytes")));
        }
        offset += count;
    }
    if offset != bytes.len() {
        return Err(fail(Stage::Verify, Visible::Unchanged, io::Error::new(io::ErrorKind::InvalidData, "staged metadata was truncated")));
    }
    drop(file);
    ops.rename(&temporary, &destination).map_err(|e| fail(Stage::Rename, Visible::MayBeInstalled, e))?;
    ops.sync_directory(parent).map_err(|e| fail(Stage::SyncDirectory, Visible::InstalledUnconfirmed, e))?;
    staging.finish().map_err(|e| fail(Stage::Cleanup, Visible::Confirmed, e))?;
    ops.sync_directory(parent).map_err(|e| fail(Stage::Cleanup, Visible::Confirmed, e))?;
    Ok(MetadataInstall { length: bytes.len() as u64, assurance })
}
