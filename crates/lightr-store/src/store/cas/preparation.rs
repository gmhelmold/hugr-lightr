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
        let failure = |phase, cause| {
            PublicationFailure::new(
                operation_id,
                PublicationOutcome::NotPublished,
                phase,
                Some(PathBuf::from("payload")),
                cause,
            )
        };
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
        let copied =
            copy_bounded(reader, &mut file).map_err(|error| failure(Phase::Stage, error))?;
        file.rewind()
            .map_err(|error| failure(Phase::Verify, error))?;
        let (digest, length) =
            Digest::of_reader(&mut file).map_err(|error| failure(Phase::Verify, error))?;
        if length != copied || expected.is_some_and(|value| value != (digest, length)) {
            return Err(failure(
                Phase::Verify,
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "staged digest or length mismatch",
                ),
            ));
        }
        sync(&file).map_err(|error| failure(Phase::PayloadSync, error))?;
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
    pub(crate) fn verify_again(&mut self) -> io::Result<()> {
        crate::store::foundation::verify_file_path(&self.file, &self._owned.payload())?;
        self.file.rewind()?;
        let actual = Digest::of_reader(&mut self.file)?;
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

fn copy_bounded(reader: &mut impl Read, writer: &mut impl Write) -> io::Result<u64> {
    let mut buffer = [0u8; 64 * 1024];
    let mut total = 0u64;
    loop {
        let size = match reader.read(&mut buffer) {
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            result => result?,
        };
        if size == 0 {
            return Ok(total);
        }
        // Defend the copy boundary even against an invalid custom Read impl.
        let bytes = buffer.get(..size).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "reader exceeded supplied buffer",
            )
        })?;
        total = total
            .checked_add(size as u64)
            .ok_or_else(|| io::Error::other("staged length overflow"))?;
        writer.write_all(bytes)?;
    }
}

#[cfg(test)]
#[path = "preparation_tests.rs"]
mod tests;
