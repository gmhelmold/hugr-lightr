//! Physical metadata-install outcomes. These are deliberately NOT the logical
//! reference transaction's commit outcome: the SI-03 caller owns that mapping.

use lightr_core::LightrError;
use std::{error::Error, fmt, io, path::{Path, PathBuf}};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InstallStage {
    Preflight,
    Allocate,
    Write,
    SyncFile,
    Verify,
    Rename,
    SyncDirectory,
    Cleanup,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InstallVisibility {
    /// This invocation has not attempted destination replacement.
    Unchanged,
    /// Replacement was attempted; do not turn an error into a no-effect claim.
    MayBeInstalled,
    /// Rename returned success, but the required namespace barrier failed.
    InstalledUnconfirmed,
    /// Required barriers succeeded; only staging cleanup failed.
    Confirmed,
}

#[derive(Debug)]
pub struct InstallFailure {
    stage: InstallStage,
    visibility: InstallVisibility,
    path: PathBuf,
    cause: io::Error,
}

impl InstallFailure {
    pub(super) fn new(stage: InstallStage, visibility: InstallVisibility, path: &Path, cause: io::Error) -> Self {
        Self { stage, visibility, path: path.to_owned(), cause }
    }

    pub fn stage(&self) -> InstallStage { self.stage }
    pub fn visibility(&self) -> InstallVisibility { self.visibility }
    pub fn path(&self) -> &Path { &self.path }
    pub fn io_error(&self) -> &io::Error { &self.cause }

    /// Preserve the frozen outer error variant without flattening the OS cause.
    /// The outer io::Error's raw_os_error may be None; downcast its payload to
    /// InstallFailure and inspect io_error(), or follow this error's source().
    pub fn into_legacy(self) -> LightrError {
        LightrError::Io(io::Error::new(self.cause.kind(), self))
    }
}

impl fmt::Display for InstallFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "metadata install {:?} ({:?}) at {}: {}", self.stage, self.visibility, self.path.display(), self.cause)
    }
}

impl Error for InstallFailure {
    fn source(&self) -> Option<&(dyn Error + 'static)> { Some(&self.cause) }
}
