//! Native prepared-tree ownership under one already-retained destination root.
use super::{TreePlan, Wait};
use lightr_core::Entry;
use std::ffi::CString;
use std::fs::File;
use std::io;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::MetadataExt;

type Identity = (u64, u64, u32);

fn component(name: &str) -> io::Result<CString> {
    if name.is_empty() || name == "." || name == ".." || name.contains('/') || name.contains('\0') {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "expected one normal prepared-output component",
        ));
    }
    CString::new(name).map_err(Into::into)
}

fn link_target(target: &str) -> io::Result<CString> {
    CString::new(target).map_err(Into::into)
}

fn identity(parent: &File, name: &CString) -> io::Result<Identity> {
    let mut info = std::mem::MaybeUninit::<libc::stat>::uninit();
    // SAFETY: retained directory, NUL-terminated component and valid output.
    if unsafe {
        libc::fstatat(
            parent.as_raw_fd(),
            name.as_ptr(),
            info.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    } != 0
    {
        return Err(io::Error::last_os_error());
    }
    let info = unsafe { info.assume_init() };
    #[cfg(target_os = "linux")]
    let (device, mode) = (info.st_dev, info.st_mode);
    #[cfg(target_os = "macos")]
    let (device, mode) = (info.st_dev as u64, u32::from(info.st_mode));
    Ok((device, info.st_ino, mode & 0o170000))
}

fn changed() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "prepared destination entry changed identity or type",
    )
}

struct OwnedName {
    parent: File,
    name: CString,
    identity: Identity,
    directory: bool,
}

impl OwnedName {
    fn revalidate(&self) -> io::Result<()> {
        if identity(&self.parent, &self.name)? != self.identity {
            return Err(changed());
        }
        Ok(())
    }

    fn remove(self) -> io::Result<()> {
        self.revalidate()?;
        let flags = if self.directory {
            libc::AT_REMOVEDIR
        } else {
            0
        };
        // SAFETY: exact operation-owned binding. Directory removal is
        // nonrecursive, so foreign children prevent destructive cleanup.
        if unsafe { libc::unlinkat(self.parent.as_raw_fd(), self.name.as_ptr(), flags) } != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
}

struct OwnedDirectory {
    file: File,
    name: OwnedName,
}

impl OwnedDirectory {
    fn create(parent: &File, raw: &str) -> Result<Self, CreateFailure> {
        let name = component(raw).map_err(CreateFailure::before)?;
        let parent = parent.try_clone().map_err(CreateFailure::before)?;
        // SAFETY: one exact component and retained parent; EEXIST is terminal.
        if unsafe { libc::mkdirat(parent.as_raw_fd(), name.as_ptr(), 0o700) } != 0 {
            return Err(CreateFailure::before(io::Error::last_os_error()));
        }
        let id = match identity(&parent, &name) {
            Ok(id) if id.2 == 0o040000 => id,
            Ok(_) => return Err(CreateFailure::uncertain(changed())),
            Err(error) => return Err(CreateFailure::uncertain(error)),
        };
        let binding = OwnedName {
            parent,
            name,
            identity: id,
            directory: true,
        };
        let flags = libc::O_RDONLY
            | libc::O_DIRECTORY
            | libc::O_NOFOLLOW
            | libc::O_CLOEXEC
            | libc::O_NONBLOCK;
        // SAFETY: exact freshly-created directory; O_NOFOLLOW forbids adoption.
        let fd = unsafe {
            libc::openat(
                binding.parent.as_raw_fd(),
                binding.name.as_ptr(),
                flags,
            )
        };
        if fd < 0 {
            return Err(CreateFailure::with_binding(
                io::Error::last_os_error(),
                binding,
            ));
        }
        // SAFETY: successful openat returned one owned descriptor.
        let file = unsafe { File::from_raw_fd(fd) };
        match file.metadata() {
            Ok(metadata)
                if (metadata.dev(), metadata.ino(), metadata.mode() & 0o170000) == id =>
            {
                Ok(Self { file, name: binding })
            }
            Ok(_) => Err(CreateFailure::with_binding(changed(), binding)),
            Err(error) => Err(CreateFailure::with_binding(error, binding)),
        }
    }

    fn revalidate(&self) -> io::Result<()> {
        self.name.revalidate()?;
        let metadata = self.file.metadata()?;
        if (metadata.dev(), metadata.ino(), metadata.mode() & 0o170000) != self.name.identity {
            return Err(changed());
        }
        Ok(())
    }
}

struct OwnedFile {
    entry_index: usize,
    file: File,
    name: OwnedName,
}

impl OwnedFile {
    fn create(parent: &File, raw: &str, entry_index: usize) -> Result<Self, CreateFailure> {
        let name = component(raw).map_err(CreateFailure::before)?;
        let parent = parent.try_clone().map_err(CreateFailure::before)?;
        let flags =
            libc::O_RDWR | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC;
        // SAFETY: retained parent and one exact component. O_EXCL forbids adoption.
        let fd = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags, 0o600) };
        if fd < 0 {
            return Err(CreateFailure::before(io::Error::last_os_error()));
        }
        // SAFETY: successful openat returned one owned descriptor.
        let file = unsafe { File::from_raw_fd(fd) };
        let metadata = match file.metadata() {
            Ok(metadata) => metadata,
            Err(error) => return Err(CreateFailure::uncertain(error)),
        };
        if !metadata.is_file() {
            return Err(CreateFailure::uncertain(changed()));
        }
        let id = (metadata.dev(), metadata.ino(), metadata.mode() & 0o170000);
        Ok(Self {
            entry_index,
            file,
            name: OwnedName {
                parent,
                name,
                identity: id,
                directory: false,
            },
        })
    }

    fn revalidate(&self) -> io::Result<()> {
        self.name.revalidate()?;
        let metadata = self.file.metadata()?;
        if (metadata.dev(), metadata.ino(), metadata.mode() & 0o170000) != self.name.identity {
            return Err(changed());
        }
        Ok(())
    }
}

struct OwnedLink {
    name: OwnedName,
}

impl OwnedLink {
    fn create(parent: &File, raw: &str, target: &str) -> Result<Self, CreateFailure> {
        let name = component(raw).map_err(CreateFailure::before)?;
        let target = link_target(target).map_err(CreateFailure::before)?;
        let parent = parent.try_clone().map_err(CreateFailure::before)?;
        // SAFETY: exact target text and one exact destination component.
        if unsafe { libc::symlinkat(target.as_ptr(), parent.as_raw_fd(), name.as_ptr()) } != 0 {
            return Err(CreateFailure::before(io::Error::last_os_error()));
        }
        let id = match identity(&parent, &name) {
            Ok(id) if id.2 == 0o120000 => id,
            Ok(_) => return Err(CreateFailure::uncertain(changed())),
            Err(error) => return Err(CreateFailure::uncertain(error)),
        };
        Ok(Self {
            name: OwnedName {
                parent,
                name,
                identity: id,
                directory: false,
            },
        })
    }
}

pub(super) struct PreparedTree {
    root: File,
    directories: Vec<OwnedDirectory>,
    files: Vec<OwnedFile>,
    links: Vec<OwnedLink>,
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
        mut observe: impl FnMut(&str, Option<&File>) -> io::Result<()>,
    ) -> Result<Self, CreateFailure> {
        let mut tree = Self {
            root: root.try_clone().map_err(CreateFailure::before)?,
            directories: Vec::new(),
            files: Vec::new(),
            links: Vec::new(),
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
            let directory = match OwnedDirectory::create(parent, name) {
                Ok(directory) => directory,
                Err(error) => return Err(tree.fail_create(error)),
            };
            tree.directories.push(directory);
            let handle = &tree.directories.last().expect("just pushed directory").file;
            if let Err(error) = observe(path, Some(handle)).and_then(|()| wait.check()) {
                return Err(tree.fail(error));
            }
        }

        for (entry_index, entry) in plan.entries().iter().enumerate() {
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
                    let file = match OwnedFile::create(parent, name, entry_index) {
                        Ok(file) => file,
                        Err(error) => return Err(tree.fail_create(error)),
                    };
                    tree.files.push(file);
                    let handle = &tree.files.last().expect("just pushed file").file;
                    if let Err(error) = observe(entry.path(), Some(handle)).and_then(|()| wait.check())
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
                    if let Err(error) = observe(entry.path(), None).and_then(|()| wait.check()) {
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
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "prepared parent not created"))
    }

    pub(super) fn revalidate(&self, wait: Wait<'_>) -> io::Result<()> {
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

    pub(super) fn file_mut(&mut self, entry_index: usize) -> Option<&mut File> {
        self.files
            .iter_mut()
            .find(|file| file.entry_index == entry_index)
            .map(|file| &mut file.file)
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

fn split(path: &str) -> (&str, &str) {
    path.rsplit_once('/').unwrap_or(("", path))
}

fn allocation(error: std::collections::TryReserveError) -> io::Error {
    io::Error::new(io::ErrorKind::OutOfMemory, error)
}
