//! Identity-checked native entries owned by one prepared output tree.
use super::CreateFailure;
use std::ffi::{CStr, CString};
use std::fs::File;
use std::io;
use std::os::fd::{AsRawFd, FromRawFd, IntoRawFd};
use std::os::unix::fs::MetadataExt;
use std::ptr::NonNull;

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

pub(super) struct OwnedName {
    parent: File,
    name: CString,
    identity: Identity,
    directory: bool,
}

impl OwnedName {
    pub(super) fn revalidate(&self) -> io::Result<()> {
        if identity(&self.parent, &self.name)? != self.identity {
            return Err(changed());
        }
        Ok(())
    }

    pub(super) fn remove(self) -> io::Result<()> {
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

pub(super) struct OwnedDirectory {
    pub(super) file: File,
    pub(super) name: OwnedName,
    expected_children: usize,
}

impl OwnedDirectory {
    pub(super) fn create(
        parent: &File,
        raw: &str,
        expected_children: usize,
    ) -> Result<Self, CreateFailure> {
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
        let fd = unsafe { libc::openat(binding.parent.as_raw_fd(), binding.name.as_ptr(), flags) };
        if fd < 0 {
            return Err(CreateFailure::with_binding(
                io::Error::last_os_error(),
                binding,
            ));
        }
        // SAFETY: successful openat returned one owned descriptor.
        let file = unsafe { File::from_raw_fd(fd) };
        match file.metadata() {
            Ok(metadata) if (metadata.dev(), metadata.ino(), metadata.mode() & 0o170000) == id => {
                Ok(Self {
                    file,
                    name: binding,
                    expected_children,
                })
            }
            Ok(_) => Err(CreateFailure::with_binding(changed(), binding)),
            Err(error) => Err(CreateFailure::with_binding(error, binding)),
        }
    }

    pub(super) fn revalidate(&self) -> io::Result<()> {
        self.name.revalidate()?;
        let metadata = self.file.metadata()?;
        if (metadata.dev(), metadata.ino(), metadata.mode() & 0o170000) != self.name.identity {
            return Err(changed());
        }
        require_child_count(&self.file, self.expected_children)
    }
}

pub(super) fn require_child_count(directory: &File, expected: usize) -> io::Result<()> {
    let dot = CString::new(".").expect("static directory component");
    let flags = libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NONBLOCK;
    // SAFETY: retained directory and static normal component. This creates a
    // fresh open-file description so enumeration never inherits another offset.
    let fd = unsafe { libc::openat(directory.as_raw_fd(), dot.as_ptr(), flags) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: successful openat returned one owned descriptor.
    let file = unsafe { File::from_raw_fd(fd) };
    // SAFETY: fdopendir acquires descriptor ownership on success only.
    let raw = unsafe { libc::fdopendir(file.as_raw_fd()) };
    let raw = NonNull::new(raw).ok_or_else(io::Error::last_os_error)?;
    let _ = file.into_raw_fd();

    let result = (|| {
        let mut count = 0usize;
        loop {
            // SAFETY: locally-owned DIR stream; d_name is borrowed only until
            // the next call. Thread-local errno distinguishes EOF from error.
            unsafe {
                *errno_address() = 0;
                let entry = libc::readdir(raw.as_ptr());
                if entry.is_null() {
                    let code = *errno_address();
                    if code == 0 {
                        break;
                    }
                    return Err(io::Error::from_raw_os_error(code));
                }
                let name = CStr::from_ptr((*entry).d_name.as_ptr()).to_bytes();
                if name != b"." && name != b".." {
                    count = count.checked_add(1).ok_or_else(|| {
                        io::Error::new(io::ErrorKind::InvalidData, "directory entry count overflow")
                    })?;
                    if count > expected {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "prepared directory contains an unowned entry",
                        ));
                    }
                }
            }
        }
        if count != expected {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "prepared directory child count changed",
            ));
        }
        Ok(())
    })();

    // SAFETY: sole stream owner, consumed exactly once. Never retry close.
    let closed = if unsafe { libc::closedir(raw.as_ptr()) } == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    };
    result.and(closed)
}

#[cfg(target_os = "linux")]
unsafe fn errno_address() -> *mut libc::c_int {
    unsafe { libc::__errno_location() }
}
#[cfg(target_os = "macos")]
unsafe fn errno_address() -> *mut libc::c_int {
    unsafe { libc::__error() }
}

pub(super) struct OwnedFile {
    pub(super) file: File,
    pub(super) name: OwnedName,
}

impl OwnedFile {
    pub(super) fn create(parent: &File, raw: &str) -> Result<Self, CreateFailure> {
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
            file,
            name: OwnedName {
                parent,
                name,
                identity: id,
                directory: false,
            },
        })
    }

    pub(super) fn revalidate(&self) -> io::Result<()> {
        self.name.revalidate()?;
        let metadata = self.file.metadata()?;
        if (metadata.dev(), metadata.ino(), metadata.mode() & 0o170000) != self.name.identity {
            return Err(changed());
        }
        Ok(())
    }
}

pub(super) struct OwnedLink {
    pub(super) name: OwnedName,
}

impl OwnedLink {
    pub(super) fn create(parent: &File, raw: &str, target: &str) -> Result<Self, CreateFailure> {
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
