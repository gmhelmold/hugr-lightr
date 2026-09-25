//! Owned creation of a previously missing destination suffix.
//!
//! Every mutation is relative to a retained parent. No existing name is adopted,
//! no symlink is followed, and cleanup removes only a still-bound empty directory.
use super::Wait;
use std::ffi::CString;
use std::fs::File;
use std::io;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::MetadataExt;

type Identity = (u64, u64, u32);

fn component(name: &[u8]) -> io::Result<CString> {
    if name.is_empty() || name == b"." || name == b".." || name.contains(&b'/') || name.contains(&0)
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "expected one normal destination component",
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
        "owned destination component changed identity or type",
    )
}

struct OwnedDirectory {
    _file: File,
    parent: File,
    name: CString,
    identity: Identity,
}

impl OwnedDirectory {
    fn remove(self) -> io::Result<()> {
        if identity(&self.parent, &self.name)? != self.identity {
            return Err(changed());
        }
        // SAFETY: exact operation-owned component; directory removal is
        // nonrecursive, so unknown children preserve the directory via ENOTEMPTY.
        if unsafe {
            libc::unlinkat(
                self.parent.as_raw_fd(),
                self.name.as_ptr(),
                libc::AT_REMOVEDIR,
            )
        } != 0
        {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
}

pub(super) struct Anchor {
    root: File,
    created: Vec<OwnedDirectory>,
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

impl Anchor {
    pub(super) fn existing(directory: &File) -> io::Result<Self> {
        Ok(Self {
            root: directory.try_clone()?,
            created: Vec::new(),
        })
    }

    pub(super) fn create(
        base: &File,
        missing: &[Vec<u8>],
        wait: Wait<'_>,
        mut after_create: impl FnMut(usize, &File) -> io::Result<()>,
    ) -> Result<Self, CreateFailure> {
        let mut parent = match base.try_clone() {
            Ok(parent) => parent,
            Err(error) => return Err(CreateFailure::before(error)),
        };
        let mut created = Vec::new();
        if let Err(error) = created.try_reserve_exact(missing.len()) {
            return Err(CreateFailure::before(io::Error::new(
                io::ErrorKind::OutOfMemory,
                error,
            )));
        }

        for (index, raw) in missing.iter().enumerate() {
            if let Err(error) = wait.check() {
                return Err(cleanup_failure(error, created, true));
            }
            let name = match component(raw) {
                Ok(name) => name,
                Err(error) => return Err(cleanup_failure(error, created, true)),
            };
            let owned_parent = match parent.try_clone() {
                Ok(parent) => parent,
                Err(error) => return Err(cleanup_failure(error, created, true)),
            };

            // SAFETY: retained parent and one exact component. mkdirat is
            // create-only: EEXIST is an error and is never converted to adoption.
            let result = unsafe { libc::mkdirat(owned_parent.as_raw_fd(), name.as_ptr(), 0o700) };
            if result != 0 {
                return Err(cleanup_failure(io::Error::last_os_error(), created, true));
            }

            let id = match identity(&owned_parent, &name) {
                Ok(id) if id.2 == 0o040000 => id,
                Ok(_) => return Err(cleanup_failure(changed(), created, false)),
                Err(error) => return Err(cleanup_failure(error, created, false)),
            };
            let flags = libc::O_RDONLY
                | libc::O_DIRECTORY
                | libc::O_NOFOLLOW
                | libc::O_CLOEXEC
                | libc::O_NONBLOCK;
            // SAFETY: exact created name and retained parent. O_NOFOLLOW forbids
            // substituting a link between creation and acquisition.
            let fd = unsafe { libc::openat(owned_parent.as_raw_fd(), name.as_ptr(), flags) };
            if fd < 0 {
                let primary = io::Error::last_os_error();
                let binding = OwnedBinding {
                    parent: owned_parent,
                    name,
                    identity: id,
                };
                let mut cleanup = Vec::new();
                let mut complete = true;
                if let Err(error) = binding.remove() {
                    cleanup.push(error);
                    complete = false;
                }
                let prior = cleanup_owned(created);
                cleanup.extend(prior.errors);
                complete &= prior.complete;
                return Err(CreateFailure {
                    primary,
                    cleanup,
                    cleanup_complete: complete,
                });
            }

            // SAFETY: successful openat returned one owned descriptor.
            let file = unsafe { File::from_raw_fd(fd) };
            let metadata = match file.metadata() {
                Ok(metadata) => metadata,
                Err(error) => {
                    let binding = OwnedBinding {
                        parent: owned_parent,
                        name,
                        identity: id,
                    };
                    let mut cleanup = Vec::new();
                    let mut complete = true;
                    if let Err(cleanup_error) = binding.remove() {
                        cleanup.push(cleanup_error);
                        complete = false;
                    }
                    let prior = cleanup_owned(created);
                    cleanup.extend(prior.errors);
                    complete &= prior.complete;
                    return Err(CreateFailure {
                        primary: error,
                        cleanup,
                        cleanup_complete: complete,
                    });
                }
            };

            if (metadata.dev(), metadata.ino(), metadata.mode() & 0o170000) != id {
                let binding = OwnedBinding {
                    parent: owned_parent,
                    name,
                    identity: id,
                };
                let mut cleanup = Vec::new();
                let mut complete = true;
                if let Err(error) = binding.remove() {
                    cleanup.push(error);
                    complete = false;
                }
                let prior = cleanup_owned(created);
                cleanup.extend(prior.errors);
                complete &= prior.complete;
                return Err(CreateFailure {
                    primary: changed(),
                    cleanup,
                    cleanup_complete: complete,
                });
            }

            let next_parent = match file.try_clone() {
                Ok(next) => next,
                Err(error) => {
                    created.push(OwnedDirectory {
                        _file: file,
                        parent: owned_parent,
                        name,
                        identity: id,
                    });
                    return Err(cleanup_failure(error, created, true));
                }
            };
            created.push(OwnedDirectory {
                _file: file,
                parent: owned_parent,
                name,
                identity: id,
            });
            parent = next_parent;
            if let Err(error) = after_create(index, &parent) {
                return Err(cleanup_failure(error, created, true));
            }
        }

        let root = match parent.try_clone() {
            Ok(root) => root,
            Err(error) => return Err(cleanup_failure(error, created, true)),
        };
        Ok(Self { root, created })
    }

    pub(super) fn handle(&self) -> &File {
        &self.root
    }

    pub(super) fn created_components(&self) -> usize {
        self.created.len()
    }

    pub(super) fn rollback(self) -> Cleanup {
        drop(self.root);
        cleanup_owned(self.created)
    }
}

struct OwnedBinding {
    parent: File,
    name: CString,
    identity: Identity,
}

impl OwnedBinding {
    fn remove(self) -> io::Result<()> {
        if identity(&self.parent, &self.name)? != self.identity {
            return Err(changed());
        }
        if unsafe {
            libc::unlinkat(
                self.parent.as_raw_fd(),
                self.name.as_ptr(),
                libc::AT_REMOVEDIR,
            )
        } != 0
        {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
}

impl CreateFailure {
    fn before(primary: io::Error) -> Self {
        Self {
            primary,
            cleanup: Vec::new(),
            cleanup_complete: true,
        }
    }
}

fn cleanup_failure(
    primary: io::Error,
    created: Vec<OwnedDirectory>,
    complete: bool,
) -> CreateFailure {
    let cleanup = cleanup_owned(created);
    CreateFailure {
        primary,
        cleanup: cleanup.errors,
        cleanup_complete: complete && cleanup.complete,
    }
}

fn cleanup_owned(mut created: Vec<OwnedDirectory>) -> Cleanup {
    let mut errors = Vec::new();
    let mut complete = true;
    while let Some(directory) = created.pop() {
        if let Err(error) = directory.remove() {
            errors.push(error);
            complete = false;
        }
    }
    Cleanup { errors, complete }
}
