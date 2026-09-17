//! Exclusive, operation-owned staging for the existing temp/rename protocol.
//!
//! Names are only candidates. Atomic mkdir, not the PID or counter, proves
//! ownership. The future readiness/journal protocol is deliberately not here.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);
const ATTEMPTS: usize = 32;

pub(crate) struct OwnedTemp {
    dir: PathBuf,
}

impl OwnedTemp {
    pub(crate) fn new(parent: &Path, hint: &str) -> io::Result<Self> {
        Self::reserve(parent, || {
            format!(
                ".tmp-{}-{hint}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            )
        })
    }

    fn reserve(parent: &Path, mut candidate: impl FnMut() -> String) -> io::Result<Self> {
        for _ in 0..ATTEMPTS {
            let dir = parent.join(candidate());
            let mut builder = fs::DirBuilder::new();
            builder.recursive(false);
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt;
                builder.mode(0o700);
            }
            match builder.create(&dir) {
                Ok(()) => return Ok(Self { dir }),
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(e),
            }
        }
        Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "exclusive staging allocation exhausted 32 candidates",
        ))
    }

    /// Complete cleanup explicitly after a new checked writer installed payload.
    /// Legacy callers keep their existing best-effort Drop behavior.
    pub(crate) fn finish(self) -> io::Result<()> {
        fs::remove_dir(&self.dir)
    }

    pub(crate) fn payload(&self) -> PathBuf {
        self.dir.join("payload")
    }
}

impl Drop for OwnedTemp {
    fn drop(&mut self) {
        let payload = self.payload();
        // A copied Windows read-only attribute belongs to our unpublished file,
        // not the source. Clear it only when necessary to clean up that file.
        #[cfg(windows)]
        if let Ok(meta) = fs::symlink_metadata(&payload) {
            if meta.is_file() && !meta.file_type().is_symlink() && meta.permissions().readonly() {
                let mut permissions = meta.permissions();
                permissions.set_readonly(false);
                let _ = fs::set_permissions(&payload, permissions);
            }
        }
        let _ = fs::remove_file(payload);
        // Never recursively delete unexpected children, even in our directory.
        let _ = fs::remove_dir(&self.dir);
    }
}

#[cfg(test)]
#[path = "owned_temp_tests.rs"]
mod tests;
