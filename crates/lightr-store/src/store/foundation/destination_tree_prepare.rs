//! C07/C12 prepared output namespace rooted in one retained destination.
//!
//! This is the transition from representability-only probing to operation-owned
//! output entries. It prepares directories, empty regular files and exact POSIX
//! symlinks through retained handles. It does not publish a snapshot, activate
//! hydrate or qualify Windows topology.
use super::destination_anchor::DestinationAnchor;
use super::destination_name_probe::DestinationNameProbeFailure;
use super::scratch_name_probe::NameProbeLimits;
#[cfg(any(target_os = "linux", target_os = "macos"))]
use super::topology;
use super::tree_plan::TreePlan;
use super::Wait;
use crate::Store;
use std::{fmt, io};

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[path = "destination_tree_prepare_native.rs"]
mod native;

/// Failure while preparing or explicitly rolling back operation-owned output.
#[derive(Debug)]
pub struct DestinationTreePrepareFailure {
    pub primary: Option<io::Error>,
    pub cleanup: Vec<io::Error>,
    pub cleanup_complete: bool,
}

impl DestinationTreePrepareFailure {
    fn primary(error: io::Error) -> Self {
        Self {
            primary: Some(error),
            cleanup: Vec::new(),
            cleanup_complete: true,
        }
    }

    fn from_probe(error: DestinationNameProbeFailure) -> Self {
        Self {
            primary: error.primary,
            cleanup: error.cleanup,
            cleanup_complete: error.cleanup_complete,
        }
    }
}

impl fmt::Display for DestinationTreePrepareFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "destination tree preparation failed")?;
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

impl std::error::Error for DestinationTreePrepareFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.primary
            .as_ref()
            .or_else(|| self.cleanup.first())
            .map(|error| error as &dyn std::error::Error)
    }
}

/// A not-yet-published output tree whose names are operation-owned.
///
/// Files are created empty and retained open for a later payload writer. POSIX
/// links already contain their exact stored target text because link contents are
/// immutable after creation. Directories/files use private temporary modes.
/// Explicit rollback removes only entries whose native identity still matches.
/// Drop performs the same cleanup best-effort but cannot report cleanup errors.
pub struct PreparedDestinationTree<'anchor, 'inspection> {
    anchor: &'anchor DestinationAnchor<'inspection>,
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    inner: native::PreparedTree,
}

impl<'inspection> DestinationAnchor<'inspection> {
    /// Probe the complete tree in this exact destination, then prepare its actual
    /// namespace by anchored create-only operations.
    pub fn prepare_tree<'anchor>(
        &'anchor self,
        plan: &TreePlan<'_>,
        limits: NameProbeLimits,
        wait: Wait<'_>,
    ) -> Result<PreparedDestinationTree<'anchor, 'inspection>, DestinationTreePrepareFailure> {
        prepare_checked(self, plan, limits, wait, |_, _, _| Ok(()))
    }
}

impl PreparedDestinationTree<'_, '_> {
    pub fn directory_count(&self) -> usize {
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            self.inner.directory_count()
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            0
        }
    }

    pub fn file_count(&self) -> usize {
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            self.inner.file_count()
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            0
        }
    }

    pub fn link_count(&self) -> usize {
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            self.inner.link_count()
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            0
        }
    }

    /// Stream and verify one manifest file into its retained destination handle.
    /// Final mode is applied only after exact size and digest verification.
    #[allow(dead_code)]
    pub(crate) fn write_file_payload(
        &mut self,
        path: &str,
        source: &mut impl std::io::Read,
        wait: Wait<'_>,
    ) -> io::Result<()> {
        self.revalidate(wait)?;
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            self.inner.write_payload(path, source, wait)
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            let _ = (path, source, wait);
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "prepared destination tree requires qualified native topology",
            ))
        }
    }

    /// Fill every prepared regular file from verified CAS objects.
    /// Any failure rolls back all operation-owned output entries.
    #[allow(dead_code)]
    pub(crate) fn write_all_payloads_from_store(
        &mut self,
        store: &Store,
        wait: Wait<'_>,
    ) -> Result<(), DestinationTreePrepareFailure> {
        if let Err(error) = self.revalidate(wait) {
            return Err(self.fail_payload(error));
        }
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            match self.inner.write_all_payloads_from_store(store, wait) {
                Ok(()) => Ok(()),
                Err(error) => Err(self.fail_payload(error)),
            }
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            let _ = (store, wait);
            Err(self.fail_payload(io::Error::new(
                io::ErrorKind::Unsupported,
                "prepared destination tree requires qualified native topology",
            )))
        }
    }

    /// Complete a fully materialized tree without publishing a ref or snapshot.
    /// An incomplete or invalid tree is rolled back instead.
    #[allow(dead_code)]
    pub(crate) fn complete(self, wait: Wait<'_>) -> Result<(), DestinationTreePrepareFailure> {
        self.complete_checked(wait, || Ok(()), || Ok(()))
    }

    #[allow(dead_code)]
    fn complete_checked(
        self,
        wait: Wait<'_>,
        mut after_initial_root_check: impl FnMut() -> io::Result<()>,
        mut after_inner_complete: impl FnMut() -> io::Result<()>,
    ) -> Result<(), DestinationTreePrepareFailure> {
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            let mut this = self;
            if let Err(error) = this.anchor.revalidate(wait) {
                return Err(fail_and_cleanup(this.inner, error));
            }
            if let Err(error) = after_initial_root_check() {
                return Err(fail_and_cleanup(this.inner, error));
            }
            match this.inner.complete(wait) {
                Ok(()) => {
                    if let Err(error) = after_inner_complete() {
                        return Err(fail_and_cleanup(this.inner, error));
                    }
                    if let Err(error) = this.inner.revalidate_final_descendants(wait) {
                        return Err(fail_and_cleanup(this.inner, error));
                    }
                    if let Err(error) = this.inner.revalidate_root_namespace(wait) {
                        return Err(fail_and_cleanup(this.inner, error));
                    }
                    if let Err(error) = this.anchor.revalidate(wait) {
                        // Final root binding after descendants, before disarm.
                        return Err(fail_and_cleanup(this.inner, error));
                    }
                    if let Err(error) = wait.check() {
                        return Err(fail_and_cleanup(this.inner, error));
                    }
                    this.inner.disarm();
                    Ok(())
                }
                Err(error) => Err(fail_and_cleanup(this.inner, error)),
            }
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            let _ = (self, wait);
            Err(DestinationTreePrepareFailure::primary(io::Error::new(
                io::ErrorKind::Unsupported,
                "prepared destination tree requires qualified native topology",
            )))
        }
    }

    /// Check the public root binding and every operation-owned descendant.
    pub fn revalidate(&self, wait: Wait<'_>) -> io::Result<()> {
        self.anchor.revalidate(wait)?;
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            self.inner.revalidate(wait)
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "prepared destination tree requires qualified native topology",
            ))
        }
    }

    /// Explicitly remove operation-owned leaves then directories in reverse order.
    pub fn rollback(self) -> Result<(), DestinationTreePrepareFailure> {
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            let mut this = self;
            let cleanup = this.inner.rollback();
            if cleanup.errors.is_empty() && cleanup.complete {
                Ok(())
            } else {
                Err(DestinationTreePrepareFailure {
                    primary: None,
                    cleanup: cleanup.errors,
                    cleanup_complete: cleanup.complete,
                })
            }
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            Err(DestinationTreePrepareFailure::primary(io::Error::new(
                io::ErrorKind::Unsupported,
                "prepared destination tree requires qualified native topology",
            )))
        }
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[allow(dead_code)]
    fn fail_payload(&mut self, primary: io::Error) -> DestinationTreePrepareFailure {
        let cleanup = self.inner.rollback();
        DestinationTreePrepareFailure {
            primary: Some(primary),
            cleanup: cleanup.errors,
            cleanup_complete: cleanup.complete,
        }
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    #[allow(dead_code)]
    fn fail_payload(&mut self, primary: io::Error) -> DestinationTreePrepareFailure {
        let _ = self;
        DestinationTreePrepareFailure::primary(primary)
    }

    #[cfg(all(test, any(target_os = "linux", target_os = "macos")))]
    fn test_first_file_mut(&mut self) -> Option<&mut std::fs::File> {
        self.inner.test_first_file_mut()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum PrepareStep {
    AfterRepresentation,
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    BeforeCreate,
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    BeforeDirectoryOpen,
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    Created,
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    BeforeFinalValidation,
}

fn prepare_checked<'anchor, 'inspection>(
    anchor: &'anchor DestinationAnchor<'inspection>,
    plan: &TreePlan<'_>,
    limits: NameProbeLimits,
    wait: Wait<'_>,
    mut observe: impl FnMut(PrepareStep, &str, Option<&std::fs::File>) -> io::Result<()>,
) -> Result<PreparedDestinationTree<'anchor, 'inspection>, DestinationTreePrepareFailure> {
    wait.check()
        .map_err(DestinationTreePrepareFailure::primary)?;
    anchor
        .probe_tree_names(plan, limits, wait)
        .map_err(DestinationTreePrepareFailure::from_probe)?;
    observe(PrepareStep::AfterRepresentation, "", None)
        .map_err(DestinationTreePrepareFailure::primary)?;
    wait.check()
        .map_err(DestinationTreePrepareFailure::primary)?;
    anchor
        .revalidate(wait)
        .map_err(DestinationTreePrepareFailure::primary)?;

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        topology::require_empty_handle(anchor.retained_handle(), wait)
            .map_err(DestinationTreePrepareFailure::primary)?;
        observe(
            PrepareStep::BeforeCreate,
            "",
            Some(anchor.retained_handle()),
        )
        .map_err(DestinationTreePrepareFailure::primary)?;
        wait.check()
            .map_err(DestinationTreePrepareFailure::primary)?;
        let inner = match native::PreparedTree::create(
            anchor.retained_handle(),
            plan,
            wait,
            &mut observe,
        ) {
            Ok(tree) => tree,
            Err(error) => {
                return Err(DestinationTreePrepareFailure {
                    primary: Some(error.primary),
                    cleanup: error.cleanup,
                    cleanup_complete: error.cleanup_complete,
                })
            }
        };
        if let Err(error) = observe(PrepareStep::BeforeFinalValidation, "", None) {
            return Err(fail_and_cleanup(inner, error));
        }
        if let Err(error) = wait.check() {
            return Err(fail_and_cleanup(inner, error));
        }
        if let Err(error) = anchor.revalidate(wait) {
            return Err(fail_and_cleanup(inner, error));
        }
        if let Err(error) = inner.revalidate(wait) {
            return Err(fail_and_cleanup(inner, error));
        }
        Ok(PreparedDestinationTree { anchor, inner })
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = (anchor, plan, limits, observe);
        Err(DestinationTreePrepareFailure::primary(io::Error::new(
            io::ErrorKind::Unsupported,
            "prepared destination tree requires qualified native topology",
        )))
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn fail_and_cleanup(
    mut inner: native::PreparedTree,
    primary: io::Error,
) -> DestinationTreePrepareFailure {
    let cleanup = inner.rollback();
    DestinationTreePrepareFailure {
        primary: Some(primary),
        cleanup: cleanup.errors,
        cleanup_complete: cleanup.complete,
    }
}

#[cfg(all(test, any(target_os = "linux", target_os = "macos")))]
#[path = "destination_tree_prepare_tests.rs"]
mod tests;
