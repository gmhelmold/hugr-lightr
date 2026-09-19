//! Native NAME observation in owned scratch, never destination qualification.
use super::anchored_scratch::{ScratchDirectory, ScratchFile};
use super::tree_plan::TreePlan;
use super::{StoreLease, Wait};
use lightr_core::Entry;
use std::{fmt, io};

/// Work bounds for this pass. Counts include implicit directories, not the
/// reservation root. The implementation also caps depth at 64 and nodes at 4096.
/// These are work limits, not measurements of available descriptors or storage.
#[derive(Clone, Copy, Debug)]
pub struct NameProbeLimits {
    pub max_nodes: usize,
    pub max_depth: usize,
}

/// Names created and removed successfully in ONE managed reservation.
/// Does not qualify another directory's collation, link semantics, file bytes,
/// permissions, capacity or durability. Not a writable capability or Store proof.
#[derive(Debug, PartialEq, Eq)]
pub struct ScratchNameObservation {
    pub directories: usize,
    pub leaf_names: usize,
}

/// Preserve the primary error and every explicit cleanup error independently.
/// Lower-level failed allocations retain ScratchDirectory's best-effort cleanup
/// contract; process death and descriptor-close errors are not made infallible.
#[derive(Debug)]
pub struct NameProbeFailure {
    pub primary: Option<io::Error>,
    pub cleanup: Vec<io::Error>,
}
impl From<io::Error> for NameProbeFailure {
    fn from(error: io::Error) -> Self {
        Self {
            primary: Some(error),
            cleanup: Vec::new(),
        }
    }
}
impl fmt::Display for NameProbeFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "scratch name probe failed")?;
        if let Some(error) = &self.primary {
            write!(f, ": {error}")?;
        }
        write!(f, "; {} explicit cleanup error(s)", self.cleanup.len())
    }
}
impl std::error::Error for NameProbeFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.primary
            .as_ref()
            .or_else(|| self.cleanup.first())
            .map(|error| error as &dyn std::error::Error)
    }
}

/// Probe every logical directory and leaf NAME under the SAME borrowed lease.
///
/// All sibling names coexist until their creation checks finish; removing an
/// earlier sibling first would hide case/Unicode collisions. Directories have
/// their exact hierarchy. Regular files and symlink names use empty REGULAR FILE
/// markers: no payload is read, link is created, or target text is interpreted.
/// A successful probe certifies neither links nor the actual output directory.
///
/// Failures stop new work and explicitly finish every acquired child in reverse
/// order before its parent. Unknown entries are not removed recursively. Native
/// errors/cancellation never become a successful observation. Checkpoints surround
/// bounded reservation/creation and cleanup; they do not preempt blocked syscalls.
/// The managed root must satisfy the existing stable-private-Store contract.
pub fn probe_scratch_names(
    plan: &TreePlan<'_>,
    lease: &StoreLease,
    limits: NameProbeLimits,
    wait: Wait<'_>,
) -> Result<ScratchNameObservation, NameProbeFailure> {
    probe_checked(plan, lease, limits, wait, |_, _| Ok(()))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Step {
    Created,
    BeforeCleanup,
    AfterCleanup,
}
struct Node<'a> {
    path: &'a str,
    parent: &'a str,
    name: &'a str,
    directory: bool,
}

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
                .filter(|e| !matches!(e, Entry::Dir { .. }))
                .count(),
        )
        .ok_or_else(|| budget("node count overflow"))?;
    if count > limits.max_nodes || count > 4096 {
        return Err(budget("name-probe node budget exceeded"));
    }
    // Check the complete plan BEFORE reserving scratch or allocating the index.
    for path in plan.directories().iter().copied().chain(
        plan.entries()
            .iter()
            .filter(|e| !matches!(e, Entry::Dir { .. }))
            .map(Entry::path),
    ) {
        wait.check()?;
        let depth = path.split('/').count();
        if depth > limits.max_depth || depth > 64 {
            return Err(budget("name-probe depth budget exceeded"));
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
    plan: &TreePlan<'_>,
    lease: &StoreLease,
    limits: NameProbeLimits,
    wait: Wait<'_>,
    mut observe: impl FnMut(Step, &str) -> io::Result<()>,
) -> Result<ScratchNameObservation, NameProbeFailure> {
    let nodes = nodes(plan, limits, wait)?;
    let mut cleanup = Vec::new();
    // No growth is needed while collecting cleanup failures, one per owned node.
    cleanup
        .try_reserve_exact(nodes.len() + 1)
        .map_err(allocation)?;
    wait.check()?;
    let root = ScratchDirectory::reserve(lease, Wait::Try)?;
    let mut created = 0usize;
    let mut primary = walk(
        &root,
        "",
        &nodes,
        wait,
        &mut observe,
        &mut cleanup,
        &mut created,
    )
    .err();
    if let Err(error) = root.finish() {
        cleanup.push(error);
    }
    let final_check = observe(Step::AfterCleanup, "").and_then(|()| wait.check());
    if primary.is_none() {
        primary = final_check.err();
    }
    if primary.is_none() && cleanup.is_empty() && created != nodes.len() {
        primary = Some(io::Error::new(
            io::ErrorKind::InvalidData,
            "incomplete native name traversal",
        ));
    }
    if primary.is_some() || !cleanup.is_empty() {
        return Err(NameProbeFailure { primary, cleanup });
    }
    let directories = plan.directories().len();
    Ok(ScratchNameObservation {
        directories,
        leaf_names: nodes.len() - directories,
    })
}

enum Held<'a> {
    Directory(ScratchDirectory<'a>),
    Leaf(ScratchFile<'a>),
}
impl Held<'_> {
    fn finish(self) -> io::Result<()> {
        match self {
            Self::Directory(dir) => dir.finish(),
            Self::Leaf(file) => file.finish(),
        }
    }
}

fn walk(
    directory: &ScratchDirectory<'_>,
    parent: &str,
    nodes: &[Node<'_>],
    wait: Wait<'_>,
    observe: &mut impl FnMut(Step, &str) -> io::Result<()>,
    cleanup: &mut Vec<io::Error>,
    created: &mut usize,
) -> io::Result<()> {
    wait.check()?;
    let begin = nodes.partition_point(|n| n.parent < parent);
    let end = nodes.partition_point(|n| n.parent <= parent);
    let siblings = &nodes[begin..end];
    let mut held = Vec::new();
    held.try_reserve_exact(siblings.len()).map_err(allocation)?;
    let result = (|| {
        for node in siblings {
            wait.check()?;
            // The owner is installed before the post-allocation checkpoint.
            let item = if node.directory {
                Held::Directory(directory.directory(node.name, Wait::Try)?)
            } else {
                Held::Leaf(directory.file(node.name, Wait::Try)?)
            };
            held.push((node, item));
            *created += 1;
            observe(Step::Created, node.path)?;
            wait.check()?;
        }
        for (node, item) in &held {
            if let Held::Directory(child) = item {
                walk(child, node.path, nodes, wait, observe, cleanup, created)?;
                if !cleanup.is_empty() {
                    return Ok(());
                }
            }
        }
        observe(Step::BeforeCleanup, parent)?;
        wait.check()
    })();
    while let Some((_node, item)) = held.pop() {
        if let Err(error) = item.finish() {
            cleanup.push(error);
        }
    }
    result
}

fn budget(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}
fn allocation(error: std::collections::TryReserveError) -> io::Error {
    io::Error::new(io::ErrorKind::OutOfMemory, error)
}

#[cfg(test)]
#[path = "scratch_name_probe_tests.rs"]
mod tests;
