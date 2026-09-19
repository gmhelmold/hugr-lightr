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

/// Read-only destination inspection for materializing a stored snapshot.
///
/// No live source is accepted or inferred. This is NOT a writable destination
/// capability: initial inspection does not check emptiness; require_empty performs
/// a separate observation. Neither creates output, acquires a Store lease, or
/// excludes namespace/content changes. C12 write adapters remain separate.
/// The same conservative native profile limits as TopologyInspection apply.
///
/// ```no_run
/// use lightr_store::store::foundation::{topology::DestinationInspection, Wait};
/// use std::{io, path::Path};
/// fn inspect_output(protected: &[&Path], output: &Path) -> io::Result<()> {
///     let observation = DestinationInspection::inspect(protected, output, Wait::Try)?;
///     observation.revalidate(Wait::Try)?;
///     // A missing target exposes no handle to its existing ancestor.
///     assert_eq!(observation.is_missing(), observation.existing_directory().is_none());
///     Ok(())
/// }
/// ```
pub struct DestinationInspection {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    inner: topology_native::Inspection,
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    _private: (),
}

impl DestinationInspection {
    /// Check only the requested destination against all existing protected roots.
    /// Root completeness remains the caller's obligation. The nearest existing
    /// ancestor is retained internally for an absent suffix, never as the target.
    pub fn inspect(protected: &[&Path], destination: &Path, wait: Wait<'_>) -> io::Result<Self> {
        wait.check()?;
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            Ok(Self {
                inner: topology_native::Inspection::inspect_destination(
                    protected,
                    destination,
                    wait,
                )?,
            })
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            let _ = (protected, destination);
            Err(unsupported())
        }
    }

    /// Repeat the observation, not a namespace lock or permission for later writes.
    pub fn revalidate(&self, wait: Wait<'_>) -> io::Result<()> {
        wait.check()?;
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            let observed = &self.inner;
            observed.revalidate(wait)
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            Err(unsupported())
        }
    }

    /// Require the observed destination to be absent or currently empty.
    ///
    /// Resolve/revalidate before and after enumeration. Existing directories are
    /// enumerated through the retained handle, with a fresh stream offset; hidden
    /// names, dangling links and non-UTF-8 names all count as occupied entries.
    /// A nonempty destination returns the native ENOTEMPTY error on Linux/macOS.
    /// Nothing is created, removed, truncated or rewritten. Reading may update
    /// directory access time according to the filesystem's policy.
    ///
    /// This is a cancellable observation, NOT an exclusion guard or write token.
    /// Concurrent insertions after observation are possible. The eventual writer
    /// must retain its own namespace guards and use exclusive anchored creation.
    /// A missing destination stays missing; an old observation never adopts a
    /// newly created directory. Native reads cannot be asynchronously interrupted.
    pub fn require_empty(&self, wait: Wait<'_>) -> io::Result<()> {
        wait.check()?;
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            self.inner.revalidate(wait)?;
            if let Some(directory) = self.inner.destination_handle() {
                topology_empty::require_empty(directory, wait)?;
            }
            self.inner.revalidate(wait)
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            Err(unsupported())
        }
    }

    /// Borrow only the exact existing target. None means the destination was
    /// absent; its ancestor is NOT an output directory. O_RDONLY restricts data
    /// I/O, not all metadata operations on File. No emptiness/freshness is implied.
    pub fn existing_directory(&self) -> Option<&File> {
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            self.inner.destination_handle()
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            None
        }
    }

    /// True only for a captured missing suffix. Revalidation does not update
    /// an old observation into a newly existing directory: create a new inspection.
    pub fn is_missing(&self) -> bool {
        self.existing_directory().is_none()
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

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[path = "topology_empty.rs"]
mod topology_empty;
