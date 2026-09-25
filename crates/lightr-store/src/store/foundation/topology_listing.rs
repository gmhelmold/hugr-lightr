//! Bounded directory-name observation, not a coherent recursive snapshot.
use super::super::Wait;
use super::TopologyInspection;
use std::{io, path::Path};

/// Work limits for one directory, not a filesystem capacity or recursion budget.
/// Zero entries and zero name bytes permit only an empty directory.
#[derive(Clone, Copy, Debug)]
pub struct DirectoryLimits {
    pub max_entries: usize,
    pub max_name_bytes: usize,
}

impl TopologyInspection {
    /// Enumerate exact UTF-8 names from the source root (`None`) or a normal
    /// relative descendant directory (`Some`). Never follows a descendant link.
    ///
    /// Entries are returned in byte order, without recoding, deduplication or
    /// type inference from dirent hints. A non-UTF-8 name, duplicate observation,
    /// exceeded budget, native error or cancellation rejects the entire result.
    /// Hidden entries and dangling links are included; only `.` and `..` are not.
    ///
    /// This is a bounded observation, NOT a coherent directory snapshot, a lease,
    /// or permission to write. Concurrent additions/removals can be unobserved;
    /// callers must validate captured objects before acknowledging a snapshot.
    /// A directory read may update atime. Native reads are not preemptible.
    pub fn list_source_directory(
        &self,
        relative: Option<&Path>,
        limits: DirectoryLimits,
        wait: Wait<'_>,
    ) -> io::Result<Vec<String>> {
        wait.check()?;
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            native::list_checked(self, relative, limits, wait, || Ok(()))
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            let _ = (relative, limits);
            Err(super::unsupported())
        }
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[path = "topology_listing_native.rs"]
mod native;
