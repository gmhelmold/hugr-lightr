//! Read-only C12 namespace inspection; NOT permission to mutate a path.
//!
//! Linux/macOS resolve through directory handles and compare native ancestry.
//! Kept handles identify observed objects; they do not freeze directory moves.
//! Callers must still establish write/traversal guards before public activation.
//! Windows and unqualified filesystem/mount combinations fail Unsupported.
use super::Wait;
use std::{fs::File, io, path::Path};

/// The requested object type is explicit; a file with a trailing slash is not
/// silently converted into a file request without that slash.
#[derive(Clone, Copy)]
pub enum SourcePath<'a> {
    Directory(&'a Path),
    File(&'a Path),
}

/// One retained, read-only observation. This is not ValidatedTopology, a Store
/// lease, a writable destination handle, or a PreparedObject constructor.
pub struct TopologyInspection {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    inner: topology_native::Inspection,
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    _private: (),
}

impl TopologyInspection {
    /// All configured protected roots must already be directories. A proposed
    /// destination may be absent; inspection never creates it. Supply the whole
    /// configured inventory, not just the Store most convenient to this call.
    /// Bounds: 32 protected roots, 128 components/depth, 512 handles per observation (two during revalidation).
    pub fn inspect(
        protected: &[&Path],
        source: SourcePath<'_>,
        destination: Option<&Path>,
        wait: Wait<'_>,
    ) -> io::Result<Self> {
        wait.check()?;
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            Ok(Self {
                inner: topology_native::Inspection::inspect(protected, source, destination, wait)?,
            })
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            let _ = (protected, source, destination);
            Err(unsupported())
        }
    }

    /// Re-resolve from the original pinned working directory and compare the
    /// retained identities, ancestry and absent suffix. Changes fail explicitly;
    /// success is another observation, NOT exclusion of future namespace changes.
    pub fn revalidate(&self, wait: Wait<'_>) -> io::Result<()> {
        wait.check()?;
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            self.inner.revalidate(wait)
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            Err(unsupported())
        }
    }

    /// Borrow the exact observed read-only source; never reopen its pathname.
    /// The file/directory type was fixed by SourcePath. Content is not frozen.
    pub fn source_handle(&self) -> &File {
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            self.inner.source_handle()
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            unreachable!("unsupported platforms cannot construct an inspection")
        }
    }

    pub fn destination_is_missing(&self) -> bool {
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            self.inner.destination_is_missing()
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            false
        }
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn unsupported() -> io::Error {
    io::Error::new(
        io::ErrorKind::Unsupported,
        "native topology inspection is not qualified on this platform",
    )
}

#[cfg(test)]
#[path = "topology_tests.rs"]
mod tests;
#[cfg(any(target_os = "linux", target_os = "macos"))]
#[path = "topology_io.rs"]
mod topology_io;
#[cfg(any(target_os = "linux", target_os = "macos"))]
#[path = "topology_native.rs"]
mod topology_native;
