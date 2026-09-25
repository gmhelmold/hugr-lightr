//! Native prepared-tree orchestration under one retained destination root.
use super::{PrepareStep, TreePlan, Wait};
use entry::{require_child_count, OwnedDirectory, OwnedFile, OwnedLink, OwnedName};
use lightr_core::Entry;
use std::fs::File;
use std::io;

#[path = "destination_tree_prepare_native_entry.rs"]
mod entry;

pub(super) struct PreparedTree {
    root: File,
    directories: Vec<OwnedDirectory>,
    files: Vec<OwnedFile>,
    links: Vec<OwnedLink>,
    root_expected_children: usize,
    armed: bool,
}

pub(super) struct CreateFailure {
    pub(super) primary: io::Error,
    pub(super) cleanup: Vec<io::Error>,
    pub(super) cleanup_complete: bool,
}

pub(super) struct Cleanup {
    pub(super) errors: Vec<io::Error>,
    pub(super) complete: bool,
}

impl CreateFailure {
    fn before(primary: io::Error) -> Self {
        Self {
            primary,
            cleanup: Vec::new(),
            cleanup_complete: true,
        }
    }

    fn uncertain(primary: io::Error) -> Self {
        Self {
            primary,
            cleanup: Vec::new(),
            cleanup_complete: false,
        }
    }

    fn with_binding(primary: io::Error, binding: OwnedName) -> Self {
        match binding.remove() {
            Ok(()) => Self::before(primary),
            Err(error) => Self {
                primary,
                cleanup: vec![error],
                cleanup_complete: false,
            },
        }
    }
}

impl PreparedTree {
    pub(super) fn create(
        root: &File,
        plan: &TreePlan<'_>,
        wait: Wait<'_>,
        observe: &mut impl FnMut(PrepareStep, &str, Option<&File>) -> io::Result<()>,
    ) -> Result<Self, CreateFailure> {
        let mut tree = Self {
            root: root.try_clone().map_err(CreateFailure::before)?,
            directories: Vec::new(),
            files: Vec::new(),
            links: Vec::new(),
            root_expected_children: expected_children(plan, ""),
            armed: true,
        };
        let link_count = plan
            .entries()
            .iter()
            .filter(|entry| matches!(entry, Entry::Symlink { .. }))
            .count();
        tree.directories
            .try_reserve_exact(plan.directories().len())
            .map_err(|e| CreateFailure::before(allocation(e)))?;
        tree.files
            .try_reserve_exact(plan.file_count())
            .map_err(|e| CreateFailure::before(allocation(e)))?;
        tree.links
            .try_reserve_exact(link_count)
            .map_err(|e| CreateFailure::before(allocation(e)))?;

        for path in plan.directories() {
            if let Err(error) = wait.check() {
                return Err(tree.fail(error));
            }
            let (parent, name) = split(path);
            let parent = match tree.parent(plan, parent) {
                Ok(parent) => parent,
                Err(error) => return Err(tree.fail(error)),
            };
            let directory =
                match OwnedDirectory::create(parent, name, expected_children(plan, path), || {
                    observe(PrepareStep::BeforeDirectoryOpen, path, None)
                }) {
                    Ok(directory) => directory,
                    Err(error) => return Err(tree.fail_create(error)),
                };
            tree.directories.push(directory);
            let handle = &tree.directories.last().expect("just pushed directory").file;
            if let Err(error) =
                observe(PrepareStep::Created, path, Some(handle)).and_then(|()| wait.check())
            {
                return Err(tree.fail(error));
            }
        }

        for entry in plan.entries() {
            if matches!(entry, Entry::Dir { .. }) {
                continue;
            }
            if let Err(error) = wait.check() {
                return Err(tree.fail(error));
            }
            let (parent, name) = split(entry.path());
            let parent = match tree.parent(plan, parent) {
                Ok(parent) => parent,
                Err(error) => return Err(tree.fail(error)),
            };
            match entry {
                Entry::File { .. } => {
                    let file = match OwnedFile::create(parent, name) {
                        Ok(file) => file,
                        Err(error) => return Err(tree.fail_create(error)),
                    };
                    tree.files.push(file);
                    let handle = &tree.files.last().expect("just pushed file").file;
                    if let Err(error) = observe(PrepareStep::Created, entry.path(), Some(handle))
                        .and_then(|()| wait.check())
                    {
                        return Err(tree.fail(error));
                    }
                }
                Entry::Symlink { target, .. } => {
                    let link = match OwnedLink::create(parent, name, target) {
                        Ok(link) => link,
                        Err(error) => return Err(tree.fail_create(error)),
                    };
                    tree.links.push(link);
                    if let Err(error) = observe(PrepareStep::Created, entry.path(), None)
                        .and_then(|()| wait.check())
                    {
                        return Err(tree.fail(error));
                    }
                }
                Entry::Dir { .. } => unreachable!(),
            }
        }

        Ok(tree)
    }

    fn parent<'a>(&'a self, plan: &TreePlan<'_>, parent: &str) -> io::Result<&'a File> {
        if parent.is_empty() {
            return Ok(&self.root);
        }
        let index = plan
            .directories()
            .binary_search(&parent)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "missing prepared parent"))?;
        self.directories
            .get(index)
            .map(|directory| &directory.file)
            .ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "prepared parent not created")
            })
    }

    pub(super) fn revalidate(&self, wait: Wait<'_>) -> io::Result<()> {
        wait.check()?;
        require_child_count(&self.root, self.root_expected_children)?;
        wait.check()?;
        for directory in &self.directories {
            directory.revalidate()?;
            wait.check()?;
        }
        for file in &self.files {
            file.revalidate()?;
            wait.check()?;
        }
        for link in &self.links {
            link.name.revalidate()?;
            wait.check()?;
        }
        Ok(())
    }

    pub(super) fn directory_count(&self) -> usize {
        self.directories.len()
    }

    pub(super) fn file_count(&self) -> usize {
        self.files.len()
    }

    pub(super) fn link_count(&self) -> usize {
        self.links.len()
    }

    #[cfg(test)]
    pub(super) fn test_first_file_mut(&mut self) -> Option<&mut File> {
        self.files.first_mut().map(|file| &mut file.file)
    }

    pub(super) fn rollback(&mut self) -> Cleanup {
        self.armed = false;
        self.cleanup()
    }

    fn fail(&mut self, primary: io::Error) -> CreateFailure {
        let cleanup = self.cleanup();
        self.armed = false;
        CreateFailure {
            primary,
            cleanup: cleanup.errors,
            cleanup_complete: cleanup.complete,
        }
    }

    fn fail_create(&mut self, error: CreateFailure) -> CreateFailure {
        let cleanup = self.cleanup();
        self.armed = false;
        let mut errors = error.cleanup;
        errors.extend(cleanup.errors);
        CreateFailure {
            primary: error.primary,
            cleanup: errors,
            cleanup_complete: error.cleanup_complete && cleanup.complete,
        }
    }

    fn cleanup(&mut self) -> Cleanup {
        let mut errors = Vec::new();
        let mut complete = true;
        while let Some(link) = self.links.pop() {
            if let Err(error) = link.name.remove() {
                errors.push(error);
                complete = false;
            }
        }
        while let Some(file) = self.files.pop() {
            if let Err(error) = file.name.remove() {
                errors.push(error);
                complete = false;
            }
        }
        while let Some(directory) = self.directories.pop() {
            if let Err(error) = directory.name.remove() {
                errors.push(error);
                complete = false;
            }
        }
        Cleanup { errors, complete }
    }
}

impl Drop for PreparedTree {
    fn drop(&mut self) {
        if self.armed {
            let _ = self.cleanup();
            self.armed = false;
        }
    }
}

fn expected_children(plan: &TreePlan<'_>, parent: &str) -> usize {
    let directories = plan
        .directories()
        .iter()
        .filter(|path| split(path).0 == parent)
        .count();
    let leaves = plan
        .entries()
        .iter()
        .filter(|entry| !matches!(entry, Entry::Dir { .. }) && split(entry.path()).0 == parent)
        .count();
    directories + leaves
}

fn split(path: &str) -> (&str, &str) {
    path.rsplit_once('/').unwrap_or(("", path))
}

fn allocation(error: std::collections::TryReserveError) -> io::Error {
    io::Error::new(io::ErrorKind::OutOfMemory, error)
}
