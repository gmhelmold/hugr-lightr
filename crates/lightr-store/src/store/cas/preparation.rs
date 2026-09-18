//! Explicit private-file preparation with a crate-private installation seam.
//! Only the foundation publisher uses that seam; existing Store routing is unchanged.
//!
//! The caller supplies an already resolved, managed staging parent and retains
//! its operation lease. This helper does not create that parent, resolve public
//! source paths, or provide GC protection. It is not wired into Store routing.

use super::OwnedTemp;
use crate::store::foundation::{Phase, PublicationFailure, PublicationOutcome};
use lightr_core::Digest;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, Write};
use std::path::{Path, PathBuf};

/// Verified unpublished bytes. This is NOT a lease-bound PreparedObject.
///
/// The writable handle remains private. Reading and rewinding cannot change the
/// staged bytes through this API. Dropping closes the handle before the existing
/// OwnedTemp cleanup runs, including on Windows. No destination is overwritten.
pub struct StagedFile {
    file: File,
    _owned: OwnedTemp,
    digest: Digest,
    length: u64,
}

impl StagedFile {
    /// Copy into an exclusively allocated temporary, verify its actual bytes,
    /// and check file synchronization on the retained read/write handle.
    ///
    /// `expected`, when supplied, binds both digest and length. No directory
    /// durability, persistent readiness, final permissions or lease is inferred.
    pub fn copy_from_reader(
        parent: &Path,
        reader: &mut impl Read,
        expected: Option<(Digest, u64)>,
        operation_id: [u8; 16],
    ) -> Result<Self, PublicationFailure> {
        Self::copy_with_sync(parent, reader, expected, operation_id, File::sync_all)
    }

    fn copy_with_sync(
        parent: &Path,
        reader: &mut impl Read,
        expected: Option<(Digest, u64)>,
        operation_id: [u8; 16],
        sync: impl FnOnce(&File) -> io::Result<()>,
    ) -> Result<Self, PublicationFailure> {
        Self::copy_with_checkpoints(parent, reader, expected, operation_id, sync, &|| Ok(()))
    }

    pub(crate) fn copy_checked(
        parent: &Path,
        reader: &mut impl Read,
        expected: Option<(Digest, u64)>,
        operation_id: [u8; 16],
        checkpoint: &dyn Fn() -> io::Result<()>,
    ) -> Result<Self, PublicationFailure> {
        Self::copy_with_checkpoints(
            parent,
            reader,
            expected,
            operation_id,
            File::sync_all,
            checkpoint,
        )
    }

    fn copy_with_checkpoints(
        parent: &Path,
        reader: &mut impl Read,
        expected: Option<(Digest, u64)>,
        operation_id: [u8; 16],
        sync: impl FnOnce(&File) -> io::Result<()>,
        checkpoint: &dyn Fn() -> io::Result<()>,
    ) -> Result<Self, PublicationFailure> {
        let failure = |phase, cause| {
            PublicationFailure::new(
                operation_id,
                PublicationOutcome::NotPublished,
                phase,
                Some(PathBuf::from("payload")),
                cause,
            )
        };
        checkpoint().map_err(|error| failure(Phase::Stage, error))?;
        let owned =
            OwnedTemp::new(parent, "stage").map_err(|error| failure(Phase::Stage, error))?;
        let mut options = OpenOptions::new();
        options.read(true).write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options
            .open(owned.payload())
            .map_err(|error| failure(Phase::Stage, error))?;
        let copied = copy_bounded_checked(reader, &mut file, checkpoint)
            .map_err(|error| failure(Phase::Stage, error))?;
        Self::finish_capture(
            owned,
            file,
            copied,
            expected,
            operation_id,
            sync,
            checkpoint,
        )
    }

    fn finish_capture(
        owned: OwnedTemp,
        mut file: File,
        copied: u64,
        expected: Option<(Digest, u64)>,
        operation_id: [u8; 16],
        sync: impl FnOnce(&File) -> io::Result<()>,
        checkpoint: &dyn Fn() -> io::Result<()>,
    ) -> Result<Self, PublicationFailure> {
        let failure = |phase, cause| {
            PublicationFailure::new(
                operation_id,
                PublicationOutcome::NotPublished,
                phase,
                Some(PathBuf::from("payload")),
                cause,
            )
        };
        file.rewind()
            .map_err(|error| failure(Phase::Verify, error))?;
        let (digest, length) = Digest::of_reader_checked(&mut file, checkpoint)
            .map_err(|error| failure(Phase::Verify, error))?;
        if length != copied || expected.is_some_and(|value| value != (digest, length)) {
            return Err(failure(
                Phase::Verify,
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "staged digest or length mismatch",
                ),
            ));
        }
        checkpoint().map_err(|error| failure(Phase::PayloadSync, error))?;
        sync(&file).map_err(|error| failure(Phase::PayloadSync, error))?;
        checkpoint().map_err(|error| failure(Phase::PayloadSync, error))?;
        file.rewind()
            .map_err(|error| failure(Phase::Verify, error))?;
        Ok(Self {
            file,
            _owned: owned,
            digest,
            length,
        })
    }

    pub(crate) fn staged_path(&self) -> PathBuf {
        self._owned.payload()
    }

    /// Internal install seam only; public readers cannot alter staged bytes.
    pub(crate) fn verify_again(
        &mut self,
        checkpoint: &dyn Fn() -> io::Result<()>,
    ) -> io::Result<()> {
        crate::store::foundation::verify_file_path(&self.file, &self._owned.payload())?;
        self.file.rewind()?;
        let actual = Digest::of_reader_checked(&mut self.file, checkpoint)?;
        if actual != (self.digest, self.length) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "staged bytes changed",
            ));
        }
        Ok(())
    }

    pub(crate) fn sync_for_install(
        &self,
        payload: bool,
        sync: impl FnOnce(&File) -> io::Result<()>,
    ) -> io::Result<()> {
        #[cfg(unix)]
        if payload {
            use std::os::unix::fs::PermissionsExt;
            self.file
                .set_permissions(std::fs::Permissions::from_mode(0o444))?;
        }
        // Windows uses the retained writable handle and the private file's own
        // attributes; source read-only attributes are never inherited or edited.
        #[cfg(windows)]
        let _ = payload;
        sync(&self.file)
    }

    pub(crate) fn install(&mut self, destination: &Path) -> io::Result<()> {
        crate::store::foundation::verify_file_path(&self.file, &self._owned.payload())?;
        std::fs::rename(self._owned.payload(), destination)
    }

    pub fn digest(&self) -> Digest {
        self.digest
    }

    pub fn len(&self) -> u64 {
        self.length
    }

    pub fn is_empty(&self) -> bool {
        self.length == 0
    }

    pub fn rewind(&mut self) -> io::Result<()> {
        self.file.rewind()
    }
}

impl Read for StagedFile {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.file.read(buffer)
    }
}

#[cfg(test)]
fn copy_bounded(reader: &mut impl Read, writer: &mut impl Write) -> io::Result<u64> {
    copy_bounded_checked(reader, writer, &|| Ok(()))
}

fn copy_bounded_checked(
    reader: &mut impl Read,
    writer: &mut impl Write,
    checkpoint: &dyn Fn() -> io::Result<()>,
) -> io::Result<u64> {
    let mut buffer = [0u8; 64 * 1024];
    let mut total = 0u64;
    loop {
        checkpoint()?;
        let size = match reader.read(&mut buffer) {
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            result => result?,
        };
        checkpoint()?;
        if size == 0 {
            return Ok(total);
        }
        let bytes = buffer.get(..size).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "reader exceeded supplied buffer",
            )
        })?;
        total = total
            .checked_add(size as u64)
            .ok_or_else(|| io::Error::other("staged length overflow"))?;
        write_checked(writer, bytes, checkpoint)?;
    }
}

fn write_checked(
    writer: &mut impl Write,
    mut bytes: &[u8],
    checkpoint: &dyn Fn() -> io::Result<()>,
) -> io::Result<()> {
    while !bytes.is_empty() {
        checkpoint()?;
        let size = match writer.write(bytes) {
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            result => result?,
        };
        checkpoint()?;
        if size == 0 {
            return Err(io::ErrorKind::WriteZero.into());
        }
        bytes = bytes.get(size..).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "writer exceeded supplied buffer",
            )
        })?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "preparation_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "preparation_cancellation_tests.rs"]
mod cancellation_tests;

#[path = "file_capture.rs"]
mod file_capture;
pub use file_capture::{CaptureMethod, CaptureMode};
