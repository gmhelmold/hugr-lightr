//! C12 transition from read-only destination observation to one anchored root.
//!
//! This creates or retains only the requested output directory. It does not create
//! tree descendants, write payloads, create links, certify name representation or
//! activate hydrate. Missing suffixes are created relative to the retained native
//! ancestor; existing empty destinations are never operation-owned.
use super::topology::DestinationInspection;
use super::Wait;
use std::{fmt, fs::File, io};

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[path = "destination_anchor_native.rs"]
mod native;

/// Failure while acquiring or explicitly rolling back an anchored destination.
///
/// primary is the operation/revalidation error. cleanup contains independent
/// errors from removing directories that this operation had already created.
/// cleanup_complete=false means at least one created name could not be proved
/// removed; callers must report partial output rather than inventing no-effect.
#[derive(Debug)]
pub struct DestinationAnchorFailure {
    pub primary: Option<io::Error>,
    pub cleanup: Vec<io::Error>,
    pub cleanup_complete: bool,
}

impl DestinationAnchorFailure {
    fn primary(error: io::Error) -> Self {
        Self {
            primary: Some(error),
            cleanup: Vec::new(),
            cleanup_complete: true,
        }
    }
}

impl fmt::Display for DestinationAnchorFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "destination anchor failed")?;
        if let Some(error) = &self.primary {
            write!(f, ": {error}")?;
        }
        write!(
            f,
            "; {} cleanup error(s); cleanup_complete={}",
            self.cleanup.len(),
            self.cleanup_complete
        )
    }
}

impl std::error::Error for DestinationAnchorFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.primary
            .as_ref()
            .or_else(|| self.cleanup.first())
            .map(|error| error as &dyn std::error::Error)
    }
}

/// Exact existing-or-created destination directory tied to its inspection.
///
/// The raw descriptor and path do not escape. Future writer/probe adapters may
/// compose internally through the retained directory; this object alone creates
/// no children and grants no public materialization success.
pub struct DestinationAnchor<'a> {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    inner: native::Anchor,
    _inspection: &'a DestinationInspection,
}

impl DestinationInspection {
    /// Require the observed destination to be empty, then retain the exact
    /// existing directory or create the previously missing suffix by anchored,
    /// create-only directory operations.
    ///
    /// Missing components begin mode 0700 subject to umask. They are operation-
    /// owned only for explicit rollback. Existing destinations are never removed
    /// by rollback. Final validation proves the requested path still resolves to
    /// the retained root and protected-root observations did not change.
    pub fn anchor_empty(
        &self,
        wait: Wait<'_>,
    ) -> Result<DestinationAnchor<'_>, DestinationAnchorFailure> {
        anchor_checked(self, wait, |_, _| Ok(()))
    }
}

impl DestinationAnchor<'_> {
    /// Re-check protected roots plus the binding of the requested destination to
    /// this retained directory. This is observation, not namespace exclusion.
    pub fn revalidate(&self, wait: Wait<'_>) -> io::Result<()> {
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            self._inspection
                .validate_anchored_directory(self.inner.handle(), wait)
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            let _ = wait;
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "anchored destination requires qualified native topology",
            ))
        }
    }

    pub fn was_created(&self) -> bool {
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            self.inner.created_components() != 0
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            false
        }
    }

    pub fn created_components(&self) -> usize {
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            self.inner.created_components()
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            0
        }
    }

    /// Remove only the still-empty directory suffix created by this operation.
    /// Existing destinations are a no-op. Identity changes or unknown children
    /// are preserved and reported; removal is nonrecursive and never retried.
    pub fn rollback_empty(self) -> Result<(), DestinationAnchorFailure> {
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            let cleanup = self.inner.rollback();
            if cleanup.errors.is_empty() && cleanup.complete {
                Ok(())
            } else {
                Err(DestinationAnchorFailure {
                    primary: None,
                    cleanup: cleanup.errors,
                    cleanup_complete: cleanup.complete,
                })
            }
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            Err(DestinationAnchorFailure::primary(io::Error::new(
                io::ErrorKind::Unsupported,
                "anchored destination requires qualified native topology",
            )))
        }
    }

    #[cfg(all(test, any(target_os = "linux", target_os = "macos")))]
    fn anchored_handle(&self) -> &File {
        self.inner.handle()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AnchorStep {
    AfterPreflight,
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    AfterCreate(usize),
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    BeforeFinalValidation,
}

fn anchor_checked<'a>(
    inspection: &'a DestinationInspection,
    wait: Wait<'_>,
    mut observe: impl FnMut(AnchorStep, Option<&File>) -> io::Result<()>,
) -> Result<DestinationAnchor<'a>, DestinationAnchorFailure> {
    wait.check().map_err(DestinationAnchorFailure::primary)?;
    inspection
        .require_empty(wait)
        .map_err(DestinationAnchorFailure::primary)?;
    observe(AnchorStep::AfterPreflight, inspection.existing_directory())
        .map_err(DestinationAnchorFailure::primary)?;
    wait.check().map_err(DestinationAnchorFailure::primary)?;

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        let (base, missing) = inspection
            .anchor_parts()
            .map_err(DestinationAnchorFailure::primary)?;
        let inner = if missing.is_empty() {
            native::Anchor::existing(base).map_err(DestinationAnchorFailure::primary)?
        } else {
            match native::Anchor::create(base, missing, wait, |index, handle| {
                observe(AnchorStep::AfterCreate(index), Some(handle))?;
                wait.check()
            }) {
                Ok(anchor) => anchor,
                Err(error) => {
                    return Err(DestinationAnchorFailure {
                        primary: Some(error.primary),
                        cleanup: error.cleanup,
                        cleanup_complete: error.cleanup_complete,
                    });
                }
            }
        };
        if let Err(error) = observe(AnchorStep::BeforeFinalValidation, Some(inner.handle())) {
            return Err(fail_and_cleanup(inner, error));
        }
        if let Err(error) = wait.check() {
            return Err(fail_and_cleanup(inner, error));
        }
        if let Err(error) = inspection.validate_anchored_directory(inner.handle(), wait) {
            return Err(fail_and_cleanup(inner, error));
        }
        Ok(DestinationAnchor {
            inner,
            _inspection: inspection,
        })
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = inspection;
        Err(DestinationAnchorFailure::primary(io::Error::new(
            io::ErrorKind::Unsupported,
            "anchored destination requires qualified native topology",
        )))
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn fail_and_cleanup(inner: native::Anchor, primary: io::Error) -> DestinationAnchorFailure {
    let cleanup = inner.rollback();
    DestinationAnchorFailure {
        primary: Some(primary),
        cleanup: cleanup.errors,
        cleanup_complete: cleanup.complete,
    }
}

#[cfg(all(test, any(target_os = "linux", target_os = "macos")))]
#[path = "destination_anchor_tests.rs"]
mod tests;
