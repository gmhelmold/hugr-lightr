//! Pure C07 structural validation. No decoding, filesystem access or write authority.
use super::Wait;
use lightr_core::{Entry, Manifest};
use std::io;

/// Caller-selected work limits, not measured filesystem capacity or wire changes.
/// Zero is meaningful: an empty tree fits zero limits. Limits cover this pass,
/// not the memory already used to decode the caller's Manifest.
#[derive(Clone, Copy, Debug)]
pub struct TreeLimits {
    pub max_entries: usize,
    /// Sum of component occurrences in all entry paths, including repeats.
    pub max_components: usize,
    /// Sum of UTF-8 bytes in entry paths and symbolic-link targets.
    pub max_text_bytes: usize,
}

/// A checked logical tree, borrowed immutably for its entire lifetime.
///
/// This is NOT native name/case/Unicode representability, payload verification,
/// topology, destination emptiness, a lease, or permission to write. Link text
/// is retained without interpretation; Windows link-kind validation is separate.
/// A Unix backslash/colon is not rewritten or classified as a native separator.
///
/// ```
/// use lightr_core::Manifest;
/// use lightr_store::store::foundation::{tree_plan::{TreeLimits, TreePlan}, Wait};
/// let manifest = Manifest { version: 1, total_size: 0, entries: vec![] };
/// let limits = TreeLimits { max_entries: 0, max_components: 0, max_text_bytes: 0 };
/// let plan = TreePlan::validate(&manifest, limits, Wait::Try)?;
/// assert!(plan.directories().is_empty());
/// # Ok::<(), std::io::Error>(())
/// ```
///
/// ```compile_fail,E0506
/// use lightr_core::Manifest;
/// use lightr_store::store::foundation::{tree_plan::{TreeLimits, TreePlan}, Wait};
/// let mut manifest = Manifest { version: 1, total_size: 0, entries: vec![] };
/// let limits = TreeLimits { max_entries: 0, max_components: 0, max_text_bytes: 0 };
/// let plan = TreePlan::validate(&manifest, limits, Wait::Try).unwrap();
/// manifest.total_size = 1;
/// assert_eq!(plan.total_size(), 0);
/// ```
#[derive(Debug)]
pub struct TreePlan<'a> {
    manifest: &'a Manifest,
    directories: Vec<&'a str>,
    files: usize,
}

impl<'a> TreePlan<'a> {
    /// Checks the whole logical manifest before returning a plan. Does not call
    /// Manifest::encode/decode or inspect any live path. Checkpoints occur around
    /// bounded passes and between entries/components, not inside standard sorting.
    pub fn validate(
        manifest: &'a Manifest,
        limits: TreeLimits,
        wait: Wait<'_>,
    ) -> io::Result<Self> {
        Self::validate_checked(manifest, limits, || wait.check())
    }

    fn validate_checked(
        manifest: &'a Manifest,
        limits: TreeLimits,
        mut checkpoint: impl FnMut() -> io::Result<()>,
    ) -> io::Result<Self> {
        checkpoint()?;
        if manifest.version != 1 || u32::try_from(manifest.entries.len()).is_err() {
            return Err(invalid("unsupported manifest version or wire entry count"));
        }
        if manifest.entries.len() > limits.max_entries {
            return Err(budget("entry budget exceeded"));
        }
        let mut text = 0usize;
        let mut components = 0usize;
        let mut directory_count = 0usize;
        let mut total = 0u64;
        let mut files = 0usize;
        // Validate scalar values and budget BEFORE allocating either index.
        for entry in &manifest.entries {
            checkpoint()?;
            let path = entry.path();
            if path.is_empty() || path.len() > u16::MAX as usize || path.contains('\0') {
                return Err(invalid("empty, NUL-containing or overlong entry path"));
            }
            text = consume(
                text,
                path.len(),
                limits.max_text_bytes,
                "text budget exceeded",
            )?;
            let mut depth = 0usize;
            for component in path.split('/') {
                checkpoint()?;
                if component.is_empty() || component == "." || component == ".." {
                    return Err(invalid(
                        "entry path must have exact relative normal components",
                    ));
                }
                components = consume(
                    components,
                    1,
                    limits.max_components,
                    "component budget exceeded",
                )?;
                depth += 1;
            }
            directory_count = directory_count
                .checked_add(depth - 1)
                .ok_or_else(|| budget("directory count overflow"))?;
            match entry {
                Entry::File { size, mode, .. } => {
                    if mode & !0o7777 != 0 {
                        return Err(invalid("file mode contains non-permission bits"));
                    }
                    total = total
                        .checked_add(*size)
                        .ok_or_else(|| invalid("declared file length sum overflow"))?;
                    files += 1;
                }
                Entry::Symlink { target, .. } => {
                    if target.is_empty()
                        || target.len() > u16::MAX as usize
                        || target.contains('\0')
                    {
                        return Err(invalid("empty, NUL-containing or overlong link target"));
                    }
                    text = consume(
                        text,
                        target.len(),
                        limits.max_text_bytes,
                        "text budget exceeded",
                    )?;
                }
                Entry::Dir { .. } => {
                    directory_count = directory_count
                        .checked_add(1)
                        .ok_or_else(|| budget("directory count overflow"))?;
                }
            }
        }
        if total != manifest.total_size {
            return Err(invalid("declared total differs from file length sum"));
        }
        checkpoint()?;
        let mut entries = Vec::new();
        entries
            .try_reserve_exact(manifest.entries.len())
            .map_err(allocation)?;
        entries.extend(manifest.entries.iter());
        entries.sort_unstable_by_key(|entry| entry.path());
        checkpoint()?;
        for pair in entries.windows(2) {
            checkpoint()?;
            if pair[0].path() == pair[1].path() {
                return Err(invalid("duplicate explicit entry path"));
            }
        }
        let mut directories = Vec::new();
        directories
            .try_reserve_exact(directory_count)
            .map_err(allocation)?;
        for entry in &entries {
            checkpoint()?;
            let path = entry.path();
            for (end, _) in path.match_indices('/') {
                checkpoint()?;
                directories.push(&path[..end]);
            }
            if matches!(entry, Entry::Dir { .. }) {
                directories.push(path);
            }
        }
        directories.sort_unstable();
        directories.dedup();
        checkpoint()?;
        for directory in &directories {
            checkpoint()?;
            if let Ok(index) = entries.binary_search_by_key(directory, |entry| entry.path()) {
                if !matches!(entries[index], Entry::Dir { .. }) {
                    return Err(invalid("file or link occupies a required directory"));
                }
            }
        }
        checkpoint()?;
        Ok(Self {
            manifest,
            directories,
            files,
        })
    }

    /// Original entries and order; no copied/rewritten path or target text.
    pub fn entries(&self) -> &'a [Entry] {
        &self.manifest.entries
    }

    /// Exact explicit and implied directories, deduplicated in byte order.
    /// Every parent precedes its descendants. This is not a native output plan.
    pub fn directories(&self) -> &[&'a str] {
        &self.directories
    }

    pub fn file_count(&self) -> usize {
        self.files
    }

    pub fn total_size(&self) -> u64 {
        self.manifest.total_size
    }
}

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
fn budget(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}
fn consume(
    current: usize,
    amount: usize,
    maximum: usize,
    message: &'static str,
) -> io::Result<usize> {
    current
        .checked_add(amount)
        .filter(|total| *total <= maximum)
        .ok_or_else(|| budget(message))
}
fn allocation(error: std::collections::TryReserveError) -> io::Error {
    io::Error::new(io::ErrorKind::OutOfMemory, error)
}

#[cfg(test)]
#[path = "tree_plan_tests.rs"]
mod tests;
