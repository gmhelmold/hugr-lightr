//! C07/C12 whole-tree NAME representability in the actual anchored destination.
//!
//! This pass creates only temporary directories and regular-file name markers
//! directly below the retained destination root. It never writes payload bytes,
//! creates symlinks or activates materialization.
use super::destination_anchor::DestinationAnchor;
use super::scratch_name_probe::NameProbeLimits;
#[cfg(any(target_os = "linux", target_os = "macos"))]
use super::topology;
use super::tree_plan::TreePlan;
use super::Wait;
#[cfg(any(target_os = "linux", target_os = "macos"))]
use lightr_core::Entry;
use std::{fmt, io};

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[path = "destination_name_probe_native.rs"]
mod native;

/// Names that were representable together in this exact anchored destination.
/// This is not a payload/link/metadata proof or permission to publish output.
#[derive(Debug, PartialEq, Eq)]
pub struct DestinationNameObservation {
    pub directories: usize,
    pub leaf_names: usize,
}

/// A representation/preflight failure plus explicit cleanup evidence.
#[derive(Debug)]
pub struct DestinationNameProbeFailure {
    pub primary: Option<io::Error>,
    pub cleanup: Vec<io::Error>,
    pub cleanup_complete: bool,
}

impl DestinationNameProbeFailure {
    fn primary(error: io::Error) -> Self {
        Self {
            primary: Some(error),
            cleanup: Vec::new(),
            cleanup_complete: true,
        }
    }
}

impl fmt::Display for DestinationNameProbeFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "destination name probe failed")?;
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

impl std::error::Error for DestinationNameProbeFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.primary
            .as_ref()
            .or_else(|| self.cleanup.first())
            .map(|error| error as &dyn std::error::Error)
    }
}

/// A bounded representability preflight using the actual retained destination.
///
/// The complete logical plan and caller work limits are checked before mutation.
/// The anchored root must still be empty. Planned directories and leaf-name
/// markers are then created directly under that root using create-only native
/// operations; all siblings coexist before recursion/cleanup so native
/// case/Unicode collisions cannot be hidden by early deletion.
///
/// Regular-file and symlink entries both use empty regular-file markers: this
/// proves only NAME coexistence. Payload bytes, modes, link target text/kind and
/// capability are separate checks. Every temporary entry is explicitly removed
/// before success, then the root is checked empty and rebound to the same
/// DestinationAnchor. A later writer must still use exclusive anchored creation;
/// this preflight is not a namespace lock against concurrent foreign writers.
impl DestinationAnchor<'_> {
    pub fn probe_tree_names(
        &self,
        plan: &TreePlan<'_>,
        limits: NameProbeLimits,
        wait: Wait<'_>,
    ) -> Result<DestinationNameObservation, DestinationNameProbeFailure> {
        probe_checked(self, plan, limits, wait, |_, _, _| Ok(()))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ProbeStep {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    AfterPreflight,
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    Created,
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    BeforeCleanup,
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    AfterCleanup,
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
struct Node<'a> {
    path: &'a str,
    parent: &'a str,
    name: &'a str,
    directory: bool,
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn nodes<'a>(
    plan: &TreePlan<'a>,
    limits: NameProbeLimits,
    wait: Wait<'_>,
) -> io::Result<Vec<Node<'a>>> {
    wait.check()?;
    let count = plan
        .directories()
        .len()
        .checked_add(
            plan.entries()
                .iter()
                .filter(|entry| !matches!(entry, Entry::Dir { .. }))
                .count(),
        )
        .ok_or_else(|| budget("destination name-probe node count overflow"))?;
    if count > limits.max_nodes || count > 4096 {
        return Err(budget("destination name-probe node budget exceeded"));
    }
    for path in plan.directories().iter().copied().chain(
        plan.entries()
            .iter()
            .filter(|entry| !matches!(entry, Entry::Dir { .. }))
            .map(Entry::path),
    ) {
        wait.check()?;
        let depth = path.split('/').count();
        if depth > limits.max_depth || depth > 64 {
            return Err(budget("destination name-probe depth budget exceeded"));
        }
    }

    let mut result = Vec::new();
    result.try_reserve_exact(count).map_err(allocation)?;
    let mut add = |path: &'a str, directory| {
        let (parent, name) = path.rsplit_once('/').unwrap_or(("", path));
        result.push(Node {
            path,
            parent,
            name,
            directory,
        });
    };
    for path in plan.directories() {
        add(path, true);
    }
    for entry in plan.entries() {
        if !matches!(entry, Entry::Dir { .. }) {
            add(entry.path(), false);
        }
    }
    result.sort_unstable_by_key(|node| (node.parent, node.name));
    wait.check()?;
    Ok(result)
}

fn probe_checked(
    anchor: &DestinationAnchor<'_>,
    plan: &TreePlan<'_>,
    limits: NameProbeLimits,
    wait: Wait<'_>,
    observe: impl FnMut(ProbeStep, &str, Option<&std::fs::File>) -> io::Result<()>,
) -> Result<DestinationNameObservation, DestinationNameProbeFailure> {
    wait.check().map_err(DestinationNameProbeFailure::primary)?;

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        let mut observe = observe;
        let nodes = nodes(plan, limits, wait).map_err(DestinationNameProbeFailure::primary)?;
        let mut cleanup = Vec::new();
        cleanup
            .try_reserve_exact(nodes.len())
            .map_err(allocation)
            .map_err(DestinationNameProbeFailure::primary)?;

        anchor
            .revalidate(wait)
            .map_err(DestinationNameProbeFailure::primary)?;
        topology::require_empty_handle(anchor.retained_handle(), wait)
            .map_err(DestinationNameProbeFailure::primary)?;
        observe(
            ProbeStep::AfterPreflight,
            "",
            Some(anchor.retained_handle()),
        )
        .map_err(DestinationNameProbeFailure::primary)?;
        wait.check().map_err(DestinationNameProbeFailure::primary)?;
        let root = native::Directory::root(anchor.retained_handle())
            .map_err(DestinationNameProbeFailure::primary)?;
        let mut created = 0usize;
        let mut cleanup_complete = true;
        let mut context = native::WalkContext::new(
            wait,
            &mut observe,
            &mut cleanup,
            &mut cleanup_complete,
            &mut created,
        );
        let mut primary = native::walk(&root, "", &nodes, &mut context).err();

        let final_observation =
            observe(ProbeStep::AfterCleanup, "", Some(anchor.retained_handle()));
        if primary.is_none() {
            primary = final_observation.err();
        }

        if primary.is_none() && cleanup.is_empty() {
            if let Err(error) = wait.check() {
                primary = Some(error);
            } else if let Err(error) =
                topology::require_empty_handle(anchor.retained_handle(), wait)
            {
                primary = Some(error);
            } else if let Err(error) = anchor.revalidate(wait) {
                primary = Some(error);
            }
        }

        if primary.is_none() && cleanup.is_empty() && created != nodes.len() {
            primary = Some(io::Error::new(
                io::ErrorKind::InvalidData,
                "incomplete destination name traversal",
            ));
        }

        if primary.is_some() || !cleanup.is_empty() || !cleanup_complete {
            return Err(DestinationNameProbeFailure {
                primary,
                cleanup,
                cleanup_complete,
            });
        }

        let directories = plan.directories().len();
        Ok(DestinationNameObservation {
            directories,
            leaf_names: nodes.len() - directories,
        })
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = (anchor, plan, limits, observe);
        Err(DestinationNameProbeFailure::primary(io::Error::new(
            io::ErrorKind::Unsupported,
            "destination name probing requires qualified native topology",
        )))
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn budget(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn allocation(error: std::collections::TryReserveError) -> io::Error {
    io::Error::new(io::ErrorKind::OutOfMemory, error)
}

#[cfg(all(test, any(target_os = "linux", target_os = "macos")))]
#[path = "destination_name_probe_tests.rs"]
mod tests;
