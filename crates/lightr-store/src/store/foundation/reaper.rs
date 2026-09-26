//! Reap only scratch carrying a descriptor-bound ownership stamp.
#![allow(dead_code)] // Internal recovery seam; no public route is active.
use crate::store::foundation::{ExclusiveStoreLease, StoreLocks};
#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::fs;
use std::io;

fn mismatch() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        "scratch reaper guard belongs to another Store",
    )
}

/// Remove stale scratch owned by this Store while its exclusive guard is held.
/// Legacy and unknown entries are never adopted. This is not a hostile-writer
/// confinement boundary; stable managed roots remain a StoreLocks precondition.
pub(super) fn reap_owned_scratch(
    domain: &StoreLocks,
    guard: &ExclusiveStoreLease,
) -> io::Result<usize> {
    if !guard.belongs_to(domain) {
        return Err(mismatch());
    }
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        let staging = guard.staging()?;
        let mut removed = 0;
        for entry in fs::read_dir(staging.path())? {
            let entry = entry?;
            if super::native::reap_owned_scratch(
                staging.anchored_handle(),
                &entry.file_name(),
                || {},
            )? {
                removed += 1;
            }
        }
        Ok(removed)
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = guard;
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "scratch reaper requires qualified native support",
        ))
    }
}
