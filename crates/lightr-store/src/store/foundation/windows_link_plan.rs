//! C07 Windows link-kind derivation from the captured tree, never live paths.
use super::{tree_plan::TreePlan, Wait};
use lightr_core::Entry;
use std::{fmt, io};

/// The Windows creation kind, derived from an exact in-tree target.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkKind {
    File,
    Directory,
}

/// Static representability failure, distinct from an OS capability failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RepresentationReason {
    ComponentSyntax,
    ReservedName,
    TargetSyntax,
    OutsideTree,
    MissingTarget,
    LinkTraversal,
    NonDirectoryTraversal,
}

#[derive(Debug)]
pub struct UnsupportedRepresentation {
    /// Index in the original, unsorted manifest entries.
    pub entry_index: usize,
    pub reason: RepresentationReason,
}
impl fmt::Display for UnsupportedRepresentation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Windows representation at entry {}: {:?}",
            self.entry_index, self.reason
        )
    }
}
impl std::error::Error for UnsupportedRepresentation {}

/// Original text is borrowed, never normalized or rewritten for publication.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlannedLink<'a> {
    pub path: &'a str,
    pub target: &'a str,
    pub kind: LinkKind,
}

/// A whole-tree static Windows link plan, NOT native write authorization.
///
/// All logical names are checked against the conservative Win32 syntax subset;
/// every link must resolve through represented directories to an exact file or
/// directory (including implied directories and the captured root). No link may
/// be traversed, even immediately before `..`. Stored target text is unchanged.
///
/// Actual case/Unicode/8.3 aliases, target-filesystem length limits, privileges,
/// source native link-kind agreement, topology, leases and anchored writing
/// still require separate checks. This type performs no filesystem operation.
/// No success here implies that the tree can be materialized on a given volume.
///
/// ```
/// use lightr_core::Manifest;
/// use lightr_store::store::foundation::{tree_plan::{TreePlan, TreeLimits}, windows_link_plan::WindowsLinkPlan, Wait};
/// let m = Manifest { version: 1, total_size: 0, entries: vec![] };
/// let tree = TreePlan::validate(&m, TreeLimits { max_entries: 0, max_components: 0, max_text_bytes: 0 }, Wait::Try)?;
/// let links = WindowsLinkPlan::validate(&tree, Wait::Try)?;
/// assert!(links.links().is_empty());
/// # Ok::<(), std::io::Error>(())
/// ```
///
/// ```compile_fail,E0506
/// use lightr_core::Manifest;
/// use lightr_store::store::foundation::{tree_plan::{TreePlan, TreeLimits}, windows_link_plan::WindowsLinkPlan, Wait};
/// let mut m = Manifest { version: 1, total_size: 0, entries: vec![] };
/// let tree = TreePlan::validate(&m, TreeLimits { max_entries: 0, max_components: 0, max_text_bytes: 0 }, Wait::Try).unwrap();
/// let links = WindowsLinkPlan::validate(&tree, Wait::Try).unwrap();
/// m.total_size = 1;
/// assert_eq!(links.tree().total_size(), 0);
/// ```
#[derive(Debug)]
pub struct WindowsLinkPlan<'p, 'a> {
    tree: &'p TreePlan<'a>,
    links: Vec<PlannedLink<'a>>,
}
impl<'p, 'a> WindowsLinkPlan<'p, 'a> {
    pub fn validate(tree: &'p TreePlan<'a>, wait: Wait<'_>) -> io::Result<Self> {
        Self::validate_checked(tree, || wait.check())
    }

    fn validate_checked(
        tree: &'p TreePlan<'a>,
        mut checkpoint: impl FnMut() -> io::Result<()>,
    ) -> io::Result<Self> {
        checkpoint()?;
        let mut link_count = 0usize;
        for (index, entry) in tree.entries().iter().enumerate() {
            for component in entry.path().split('/') {
                checkpoint()?;
                component_reason(component).map_or(Ok(()), |reason| Err(failure(index, reason)))?;
            }
            if matches!(entry, Entry::Symlink { .. }) {
                link_count += 1;
            }
        }
        // TreePlan already enforces caller entry/component/text budgets. These
        // two indices borrow entries; scratch is at most path+target UTF-8 bytes.
        let mut entries = Vec::new();
        entries
            .try_reserve_exact(tree.entries().len())
            .map_err(allocation)?;
        entries.extend(tree.entries().iter());
        entries.sort_unstable_by_key(|entry| entry.path());
        checkpoint()?;
        let mut links = Vec::new();
        links.try_reserve_exact(link_count).map_err(allocation)?;
        for (index, entry) in tree.entries().iter().enumerate() {
            checkpoint()?;
            if let Entry::Symlink { path, target } = entry {
                let kind = resolve(tree, &entries, path, target, index, &mut checkpoint)?;
                links.push(PlannedLink { path, target, kind });
            }
        }
        checkpoint()?;
        Ok(Self { tree, links })
    }

    pub fn tree(&self) -> &'p TreePlan<'a> {
        self.tree
    }
    /// Original manifest order, not a publish order. Targets precede links only
    /// when the future materializer explicitly schedules them that way.
    pub fn links(&self) -> &[PlannedLink<'a>] {
        &self.links
    }
}

fn resolve(
    tree: &TreePlan<'_>,
    entries: &[&Entry],
    path: &str,
    target: &str,
    index: usize,
    checkpoint: &mut impl FnMut() -> io::Result<()>,
) -> io::Result<LinkKind> {
    use RepresentationReason as R;
    // No OS Path::components: C07 selects forward-slash relative target syntax
    // on EVERY host. Backslashes, drives, devices and empty components are not
    // reinterpreted. Dot components affect lookup only, never the stored text.
    if target.split('/').any(str::is_empty) || target.contains(['\\', ':']) {
        return Err(failure(index, R::TargetSyntax));
    }
    let capacity = path
        .len()
        .checked_add(target.len())
        .and_then(|n| n.checked_add(1))
        .ok_or_else(|| io::Error::from(io::ErrorKind::OutOfMemory))?;
    let mut selected = String::new();
    selected.try_reserve_exact(capacity).map_err(allocation)?;
    if let Some((parent, _)) = path.rsplit_once('/') {
        selected.push_str(parent);
    }
    let mut kind = LinkKind::Directory;
    for component in target.split('/') {
        checkpoint()?;
        if kind != LinkKind::Directory {
            return Err(failure(index, R::NonDirectoryTraversal));
        }
        match component {
            "." => (),
            ".." => {
                if selected.is_empty() {
                    return Err(failure(index, R::OutsideTree));
                }
                selected.truncate(selected.rfind('/').unwrap_or(0));
            }
            _ => {
                if let Some(reason) = component_reason(component) {
                    return Err(failure(index, reason));
                }
                if !selected.is_empty() {
                    selected.push('/');
                }
                selected.push_str(component);
                kind = match entries.binary_search_by_key(&selected.as_str(), |entry| entry.path())
                {
                    Ok(n) => match entries[n] {
                        Entry::File { .. } => LinkKind::File,
                        Entry::Dir { .. } => LinkKind::Directory,
                        Entry::Symlink { .. } => return Err(failure(index, R::LinkTraversal)),
                    },
                    Err(_) if tree.directories().binary_search(&selected.as_str()).is_ok() => {
                        LinkKind::Directory
                    }
                    Err(_) => return Err(failure(index, R::MissingTarget)),
                };
            }
        }
    }
    checkpoint()?;
    Ok(kind)
}

fn component_reason(name: &str) -> Option<RepresentationReason> {
    use RepresentationReason as R;
    if name.is_empty()
        || name.ends_with([' ', '.'])
        || name
            .chars()
            .any(|c| c <= '\u{1f}' || "<>:\"/\\|?*".contains(c))
    {
        return Some(R::ComponentSyntax);
    }
    // Microsoft's reserved device basenames also apply before extensions.
    // Trim spaces in the stem conservatively; never rewrite the original name.
    let stem = name.split('.').next().unwrap_or("").trim_end_matches(' ');
    if ["CON", "PRN", "AUX", "NUL", "CONIN$", "CONOUT$"]
        .iter()
        .any(|word| stem.eq_ignore_ascii_case(word))
    {
        return Some(R::ReservedName);
    }
    for prefix in ["COM", "LPT"] {
        if let Some(first) = stem.get(..3) {
            if first.eq_ignore_ascii_case(prefix)
                && matches!(
                    &stem[3..],
                    "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
                )
            {
                return Some(R::ReservedName);
            }
        }
    }
    None
}
fn failure(entry_index: usize, reason: RepresentationReason) -> io::Error {
    io::Error::new(
        io::ErrorKind::Unsupported,
        UnsupportedRepresentation {
            entry_index,
            reason,
        },
    )
}
fn allocation(error: std::collections::TryReserveError) -> io::Error {
    io::Error::new(io::ErrorKind::OutOfMemory, error)
}

#[cfg(test)]
#[path = "windows_link_plan_tests.rs"]
mod tests;
