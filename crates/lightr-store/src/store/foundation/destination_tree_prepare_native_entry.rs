//! Identity-checked native entries owned by one prepared output tree.
use super::CreateFailure;
use lightr_core::Digest;
use std::ffi::{CStr, CString};
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::os::fd::{AsRawFd, FromRawFd, IntoRawFd};
use std::os::unix::fs::MetadataExt;
use std::os::unix::fs::PermissionsExt;
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

fn invalid_payload(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
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
        mut after_mkdir_before_open: impl FnMut() -> io::Result<()>,
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
        if let Err(error) = after_mkdir_before_open() {
            return Err(CreateFailure::with_binding(error, binding));
        }
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
#[allow(dead_code)]
pub(super) struct OwnedFile {
    pub(super) file: File,
    pub(super) name: OwnedName,
    path: String,
    digest: Digest,
    size: u64,
    mode: u32,
    written: bool,
}
#[allow(dead_code)]
impl OwnedFile {
    pub(super) fn create(
        parent: &File,
        raw: &str,
        path: &str,
        digest: Digest,
        size: u64,
        mode: u32,
    ) -> Result<Self, CreateFailure> {
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
            path: path.to_owned(),
            digest,
            size,
            mode,
            written: false,
        })
    }
    pub(super) fn path(&self) -> &str {
        &self.path
    }
    pub(super) fn digest(&self) -> Digest {
        self.digest
    }
    pub(super) fn is_written(&self) -> bool {
        self.written
    }
    pub(super) fn revalidate_final_mode(&self) -> io::Result<()> {
        self.revalidate()?;
        (self.file.metadata()?.mode() & 0o7777 == self.mode)
            .then_some(())
            .ok_or_else(|| invalid_payload("prepared file mode changed before completion"))
    }
    pub(super) fn write_payload(
        &mut self,
        source: &mut impl Read,
        wait: super::super::Wait<'_>,
    ) -> io::Result<()> {
        if self.written {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "prepared file payload already written",
            ));
        }
        self.revalidate()?;
        wait.check()?;
        self.file.set_len(0)?;
        self.file.seek(SeekFrom::Start(0))?;
        self.file
            .set_permissions(std::fs::Permissions::from_mode(0o600))?;
        let mut length = 0u64;
        let mut buffer = [0u8; 64 * 1024];
        loop {
            wait.check()?;
            let count = loop {
                match source.read(&mut buffer) {
                    Ok(count) => break count,
                    Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                    Err(error) => return Err(error),
                }
            };
            wait.check()?;
            if count == 0 {
                break;
            }
            if count > buffer.len() {
                return Err(invalid_payload("payload reader exceeded supplied buffer"));
            }
            let next = length
                .checked_add(count as u64)
                .ok_or_else(|| invalid_payload("payload length overflow"))?;
            if next > self.size {
                return Err(invalid_payload("payload exceeds declared file length"));
            }
            self.file.write_all(&buffer[..count])?;
            length = next;
        }
        if length != self.size {
            return Err(invalid_payload(
                "payload length differs from declared file length",
            ));
        }
        self.file.seek(SeekFrom::Start(0))?;
        let (digest, hashed_length) = Digest::of_reader_checked(&mut self.file, || wait.check())?;
        if hashed_length != self.size || digest != self.digest {
            return Err(invalid_payload(
                "written payload does not match declared file",
            ));
        }
        wait.check()?;
        self.revalidate()?;
        self.file
            .set_permissions(std::fs::Permissions::from_mode(self.mode))?;
        self.file.sync_data()?;
        wait.check()?;
        self.revalidate()?;
        self.written = true;
        Ok(())
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
