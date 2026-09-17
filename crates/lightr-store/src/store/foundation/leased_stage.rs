//! Staging whose lifetime borrows the actual Store lease. Still not CAS-ready.
use super::lease_io::Directory;
use super::{Phase, PublicationFailure, PublicationOutcome, StoreLease};
use crate::store::cas::preparation::StagedFile;
use lightr_core::Digest;
use std::io::{self, Read};

/// Unpublished staged bytes protected by a borrowed native Store lease.
/// There is no conversion from Readiness to this capability or PreparedObject.
/// The staged handle/cleanup is dropped before the parent directory and lease.
///
/// ```
/// use lightr_store::store::foundation::{StoreLocks, LeasedStagedFile, Wait};
/// use std::{io::Cursor, path::Path};
/// fn release_in_order(root: &Path) {
///     let locks = StoreLocks::open_existing(root).unwrap();
///     let lease = locks.shared(Wait::Try).unwrap();
///     let staged = LeasedStagedFile::copy(&lease, &mut Cursor::new(b"bytes"), None, [0;16]).unwrap();
///     assert!(!staged.is_empty());
///     drop(staged);
///     drop(lease);
/// }
/// ```
///
/// ```compile_fail,E0505
/// use lightr_store::store::foundation::{StoreLocks, LeasedStagedFile, Wait};
/// use std::{io::Cursor, path::Path};
/// fn cannot_release_early(root: &Path) {
///     let locks = StoreLocks::open_existing(root).unwrap();
///     let lease = locks.shared(Wait::Try).unwrap();
///     let staged = LeasedStagedFile::copy(&lease, &mut Cursor::new(b"bytes"), None, [0;16]).unwrap();
///     drop(lease);
///     assert!(!staged.is_empty());
/// }
/// ```
pub struct LeasedStagedFile<'a> {
    stage: StagedFile,
    _parent: Directory,
    _lease: &'a StoreLease,
}
impl<'a> LeasedStagedFile<'a> {
    /// The managed staging family is selected internally; no arbitrary parent
    /// path is accepted. No receipt, ref, payload install or full GC occurs.
    pub fn copy(
        lease: &'a StoreLease,
        reader: &mut impl Read,
        expected: Option<(Digest, u64)>,
        operation_id: [u8; 16],
    ) -> Result<Self, PublicationFailure> {
        let fail = |cause| {
            PublicationFailure::new(
                operation_id,
                PublicationOutcome::NotPublished,
                Phase::Stage,
                None,
                cause,
            )
        };
        let parent = lease.staging().map_err(fail)?;
        let stage = StagedFile::copy_from_reader(parent.path(), reader, expected, operation_id)?;
        parent.verify().map_err(fail)?;
        Ok(Self {
            stage,
            _parent: parent,
            _lease: lease,
        })
    }
    pub fn digest(&self) -> Digest {
        self.stage.digest()
    }
    pub fn len(&self) -> u64 {
        self.stage.len()
    }
    pub fn is_empty(&self) -> bool {
        self.stage.is_empty()
    }
    pub fn rewind(&mut self) -> io::Result<()> {
        self.stage.rewind()
    }
}
impl Read for LeasedStagedFile<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.stage.read(buffer)
    }
}
