//! Lease-borrowed, create-only scratch. Never an arbitrary public destination.
use super::{StoreLease, Wait};
use std::io::{self, Read, Write};

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[path = "anchored_scratch_native.rs"]
mod native;

/// An operation-owned directory in the Store's managed `.si01-staging` family.
///
/// Children borrow this directory and the SAME Store lease. No path or raw
/// handle escapes. Every name is one exact component; creation never adopts an
/// existing entry. Drop is best-effort, nonrecursive cleanup. Call `finish` to
/// observe cleanup errors. Unknown children are never recursively removed.
///
/// This supplies a reservation for subsequent whole-tree representation probes;
/// it does NOT qualify another directory's case/Unicode policy, verify payloads,
/// prove durability, exclude foreign filesystem changes, or authorize output.
/// Managed Store roots must remain stable, as required by StoreLocks. Abrupt
/// death may leave owned scratch for the future domain-guarded reaper.
///
/// ```compile_fail,E0505
/// use lightr_store::store::foundation::{anchored_scratch::ScratchDirectory, StoreLocks, Wait};
/// fn example(root: &std::path::Path) {
///     let locks = StoreLocks::open_existing(root).unwrap();
///     let lease = locks.shared(Wait::Try).unwrap();
///     let scratch = ScratchDirectory::reserve(&lease, Wait::Try).unwrap();
///     drop(lease);
///     scratch.finish().unwrap();
/// }
/// ```
pub struct ScratchDirectory<'a> {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    inner: native::Directory,
    _lease: &'a StoreLease,
}

impl<'a> ScratchDirectory<'a> {
    /// Reserve at most 32 names atomically. No caller-selected staging parent.
    /// Cancellation is cooperative, including after successful allocation.
    pub fn reserve(lease: &'a StoreLease, wait: Wait<'_>) -> io::Result<Self> {
        wait.check()?;
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            let parent = lease.staging()?;
            parent.verify()?;
            let inner = native::Directory::reserve(parent.anchored_handle(), wait)?;
            parent.verify()?;
            wait.check()?;
            Ok(Self {
                inner,
                _lease: lease,
            })
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            let _ = lease;
            Err(unsupported())
        }
    }

    /// Create a fresh child directory, initially mode 0700 subject to umask.
    /// Its lifetime prevents the parent or its lease from being released first.
    pub fn directory(&self, name: &str, wait: Wait<'_>) -> io::Result<ScratchDirectory<'_>> {
        wait.check()?;
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            let inner = self.inner.directory(name)?;
            wait.check()?;
            Ok(ScratchDirectory {
                inner,
                _lease: self._lease,
            })
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            let _ = name;
            Err(unsupported())
        }
    }

    /// Create a fresh regular file, initially mode 0600 subject to umask.
    /// A file, directory or dangling link already at this name is an error.
    pub fn file(&self, name: &str, wait: Wait<'_>) -> io::Result<ScratchFile<'_>> {
        wait.check()?;
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            let inner = self.inner.file(name)?;
            wait.check()?;
            Ok(ScratchFile {
                inner,
                _lease: self._lease,
            })
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            let _ = name;
            Err(unsupported())
        }
    }

    /// Remove only this owned empty directory. No recursive removal or retry.
    /// On an error, the name may remain; this is not a scratch-reaper guarantee.
    pub fn finish(self) -> io::Result<()> {
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            self.inner.finish()
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            Err(unsupported())
        }
    }
}

/// Unpublished temporary bytes, not a PreparedObject or readiness proof.
/// Borrowing the parent prevents cleanup while this file is live. Read/Write
/// use its retained descriptor, not a pathname; no descriptor clone is exposed.
///
/// ```compile_fail,E0505
/// use lightr_store::store::foundation::{anchored_scratch::ScratchDirectory, StoreLocks, Wait};
/// fn example(root: &std::path::Path) {
///     let locks = StoreLocks::open_existing(root).unwrap();
///     let lease = locks.shared(Wait::Try).unwrap();
///     let scratch = ScratchDirectory::reserve(&lease, Wait::Try).unwrap();
///     let file = scratch.file("probe", Wait::Try).unwrap();
///     scratch.finish().unwrap();
///     file.finish().unwrap();
/// }
/// ```
pub struct ScratchFile<'a> {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    inner: native::RegularFile,
    _lease: &'a StoreLease,
}
impl ScratchFile<'_> {
    /// Rewind the same file. Does not finalize, verify or publish its contents.
    pub fn rewind(&mut self) -> io::Result<()> {
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            self.inner.rewind()
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            Err(unsupported())
        }
    }

    /// Remove this owned name. Cleanup errors are returned, never retried.
    pub fn finish(self) -> io::Result<()> {
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            self.inner.finish()
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            Err(unsupported())
        }
    }
}
impl Read for ScratchFile<'_> {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            self.inner.read(bytes)
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            let _ = bytes;
            Err(unsupported())
        }
    }
}
impl Write for ScratchFile<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            self.inner.write(bytes)
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            let _ = bytes;
            Err(unsupported())
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            self.inner.flush()
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            Err(unsupported())
        }
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn unsupported() -> io::Error {
    io::Error::new(
        io::ErrorKind::Unsupported,
        "anchored scratch requires qualified native support",
    )
}

#[cfg(test)]
#[path = "anchored_scratch_tests.rs"]
mod tests;
