//! Create-only temporary name markers in one retained public destination root.
//!
//! Deliberately separate from Store-private scratch: this ownership domain is the
//! user's already-validated empty output directory, not managed Store staging.
use super::{Node, ProbeStep, Wait};
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
            "expected one normal destination name component",
        ));
    }
    CString::new(name).map_err(Into::into)
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
        "destination probe name changed identity or type",
    )
}

struct OwnedName {
    parent: File,
    name: CString,
    identity: Identity,
    attempted: bool,
}

impl OwnedName {
    fn remove(&mut self) -> io::Result<()> {
        self.attempted = true;
        if identity(&self.parent, &self.name)? != self.identity {
            return Err(changed());
        }
        let flags = if self.identity.2 == 0o040000 {
            libc::AT_REMOVEDIR
        } else {
            0
        };
        // SAFETY: exact operation-owned name under its retained parent. Directory
        // removal is nonrecursive, so foreign children prevent destructive cleanup.
        if unsafe { libc::unlinkat(self.parent.as_raw_fd(), self.name.as_ptr(), flags) } != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
}

impl Drop for OwnedName {
    fn drop(&mut self) {
        if !self.attempted {
            let _ = self.remove();
        }
    }
}

pub(super) struct CreateFailure {
    pub(super) primary: io::Error,
    pub(super) cleanup: Vec<io::Error>,
    pub(super) cleanup_complete: bool,
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

    fn with_binding(primary: io::Error, mut binding: OwnedName) -> Self {
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

pub(super) struct Directory {
    file: File,
    owned: Option<OwnedName>,
}

impl Directory {
    pub(super) fn root(root: &File) -> io::Result<Self> {
        Ok(Self {
            file: root.try_clone()?,
            owned: None,
        })
    }

    fn create(parent: &File, name: &str) -> Result<Self, CreateFailure> {
        let name = component(name).map_err(CreateFailure::before)?;
        let parent = parent.try_clone().map_err(CreateFailure::before)?;

        // SAFETY: one exact component and retained parent. mkdirat is create-only;
        // EEXIST is terminal and never converted into adoption.
        if unsafe { libc::mkdirat(parent.as_raw_fd(), name.as_ptr(), 0o700) } != 0 {
            return Err(CreateFailure::before(io::Error::last_os_error()));
        }

        let id = match identity(&parent, &name) {
            Ok(id) if id.2 == 0o040000 => id,
            Ok(_) => return Err(CreateFailure::uncertain(changed())),
            Err(error) => return Err(CreateFailure::uncertain(error)),
        };

        let flags = libc::O_RDONLY
            | libc::O_DIRECTORY
            | libc::O_NOFOLLOW
            | libc::O_CLOEXEC
            | libc::O_NONBLOCK;
        // SAFETY: exact freshly created name. O_NOFOLLOW prevents link adoption.
        let fd = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags) };
        if fd < 0 {
            return Err(CreateFailure::with_binding(
                io::Error::last_os_error(),
                OwnedName {
                    parent,
                    name,
                    identity: id,
                    attempted: false,
                },
            ));
        }
        // SAFETY: successful openat returned one owned descriptor.
        let file = unsafe { File::from_raw_fd(fd) };
        let metadata = match file.metadata() {
            Ok(metadata) => metadata,
            Err(error) => {
                return Err(CreateFailure::with_binding(
                    error,
                    OwnedName {
                        parent,
                        name,
                        identity: id,
                        attempted: false,
                    },
                ))
            }
        };
        if (metadata.dev(), metadata.ino(), metadata.mode() & 0o170000) != id {
            return Err(CreateFailure::with_binding(
                changed(),
                OwnedName {
                    parent,
                    name,
                    identity: id,
                    attempted: false,
                },
            ));
        }

        Ok(Self {
            file,
            owned: Some(OwnedName {
                parent,
                name,
                identity: id,
                attempted: false,
            }),
        })
    }

    pub(super) fn directory(&self, name: &str) -> Result<Self, CreateFailure> {
        Self::create(&self.file, name)
    }

    pub(super) fn marker(&self, name: &str) -> Result<Marker, CreateFailure> {
        let name = component(name).map_err(CreateFailure::before)?;
        let parent = self.file.try_clone().map_err(CreateFailure::before)?;
        let flags =
            libc::O_RDWR | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC;
        // SAFETY: retained directory and exact component. O_EXCL forbids adoption.
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
        Ok(Marker {
            file,
            owned: OwnedName {
                parent,
                name,
                identity: id,
                attempted: false,
            },
        })
    }

    pub(super) fn handle(&self) -> &File {
        &self.file
    }

    pub(super) fn finish(mut self) -> io::Result<()> {
        match &mut self.owned {
            Some(owned) => owned.remove(),
            None => Ok(()),
        }
    }
}

pub(super) struct Marker {
    file: File,
    owned: OwnedName,
}

impl Marker {
    pub(super) fn handle(&self) -> &File {
        &self.file
    }

    pub(super) fn finish(mut self) -> io::Result<()> {
        self.owned.remove()
    }
}

enum Held {
    Directory(Directory),
    Marker(Marker),
}

impl Held {
    fn handle(&self) -> &File {
        match self {
            Self::Directory(directory) => directory.handle(),
            Self::Marker(marker) => marker.handle(),
        }
    }

    fn finish(self) -> io::Result<()> {
        match self {
            Self::Directory(directory) => directory.finish(),
            Self::Marker(marker) => marker.finish(),
        }
    }
}

pub(super) struct WalkContext<'a, O> {
    wait: Wait<'a>,
    observe: &'a mut O,
    cleanup: &'a mut Vec<io::Error>,
    cleanup_complete: &'a mut bool,
    created: &'a mut usize,
}

impl<'a, O> WalkContext<'a, O> {
    pub(super) fn new(
        wait: Wait<'a>,
        observe: &'a mut O,
        cleanup: &'a mut Vec<io::Error>,
        cleanup_complete: &'a mut bool,
        created: &'a mut usize,
    ) -> Self {
        Self {
            wait,
            observe,
            cleanup,
            cleanup_complete,
            created,
        }
    }
}

pub(super) fn walk<O>(
    directory: &Directory,
    parent: &str,
    nodes: &[Node<'_>],
    context: &mut WalkContext<'_, O>,
) -> io::Result<()>
where
    O: FnMut(ProbeStep, &str, Option<&File>) -> io::Result<()>,
{
    context.wait.check()?;
    let begin = nodes.partition_point(|node| node.parent < parent);
    let end = nodes.partition_point(|node| node.parent <= parent);
    let siblings = &nodes[begin..end];

    let mut held = Vec::new();
    held.try_reserve_exact(siblings.len())
        .map_err(|error| io::Error::new(io::ErrorKind::OutOfMemory, error))?;

    let result = (|| {
        for node in siblings {
            context.wait.check()?;
            let item = match if node.directory {
                directory.directory(node.name).map(Held::Directory)
            } else {
                directory.marker(node.name).map(Held::Marker)
            } {
                Ok(item) => item,
                Err(error) => {
                    context.cleanup.extend(error.cleanup);
                    *context.cleanup_complete &= error.cleanup_complete;
                    return Err(error.primary);
                }
            };
            held.push((node, item));
            *context.created += 1;
            let handle = held
                .last()
                .expect("just pushed destination probe name")
                .1
                .handle();
            (context.observe)(ProbeStep::Created, node.path, Some(handle))?;
            context.wait.check()?;
        }

        for (node, item) in &held {
            if let Held::Directory(child) = item {
                walk(child, node.path, nodes, context)?;
                if !context.cleanup.is_empty() || !*context.cleanup_complete {
                    return Ok(());
                }
            }
        }

        (context.observe)(ProbeStep::BeforeCleanup, parent, Some(directory.handle()))?;
        context.wait.check()
    })();

    while let Some((_node, item)) = held.pop() {
        if let Err(error) = item.finish() {
            context.cleanup.push(error);
            *context.cleanup_complete = false;
        }
    }
    result
}
