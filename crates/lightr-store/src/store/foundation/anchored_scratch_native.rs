//! Owned, single-component mutations inside a private, lease-protected reserve.
use super::Wait;
use std::ffi::{CStr, CString, OsStr};
use std::fs::File;
use std::io::{self, Read, Seek, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

#[cfg(target_os = "linux")]
const OWNERSHIP_XATTR: &[u8] = b"user.lightr.si01.owned-scratch\0";
#[cfg(target_os = "macos")]
const OWNERSHIP_XATTR: &[u8] = b"com.hugr.lightr.si01.owned-scratch\0";
const OWNERSHIP_STAMP: &[u8] = b"lightr-si01-owned-scratch-v1";

fn ownership_name() -> *const libc::c_char {
    OWNERSHIP_XATTR.as_ptr().cast()
}

fn set_ownership(file: &File) -> io::Result<()> {
    #[cfg(target_os = "linux")]
    let result = unsafe {
        libc::fsetxattr(
            file.as_raw_fd(),
            ownership_name(),
            OWNERSHIP_STAMP.as_ptr().cast(),
            OWNERSHIP_STAMP.len(),
            0,
        )
    };
    #[cfg(target_os = "macos")]
    let result = unsafe {
        libc::fsetxattr(
            file.as_raw_fd(),
            ownership_name(),
            OWNERSHIP_STAMP.as_ptr().cast(),
            OWNERSHIP_STAMP.len(),
            0,
            0,
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[allow(dead_code)] // Internal recovery seam; no public route is active.
fn has_ownership(file: &File) -> io::Result<bool> {
    let mut stamp = [0; OWNERSHIP_STAMP.len()];
    #[cfg(target_os = "linux")]
    let result = unsafe {
        libc::fgetxattr(
            file.as_raw_fd(),
            ownership_name(),
            stamp.as_mut_ptr().cast(),
            stamp.len(),
        )
    };
    #[cfg(target_os = "macos")]
    let result = unsafe {
        libc::fgetxattr(
            file.as_raw_fd(),
            ownership_name(),
            stamp.as_mut_ptr().cast(),
            stamp.len(),
            0,
            0,
        )
    };
    if result >= 0 {
        return Ok(result as usize == OWNERSHIP_STAMP.len() && stamp == OWNERSHIP_STAMP);
    }
    let error = io::Error::last_os_error();
    #[cfg(target_os = "linux")]
    if error.raw_os_error() == Some(libc::ENODATA) {
        return Ok(false);
    }
    #[cfg(target_os = "macos")]
    if error.raw_os_error() == Some(libc::ENOATTR) {
        return Ok(false);
    }
    Err(error)
}

#[allow(dead_code)] // Internal recovery seam; no public route is active.
fn component_os(name: &OsStr) -> io::Result<CString> {
    let name = name.as_bytes();
    if name.is_empty() || name == b"." || name == b".." || name.contains(&b'/') {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "expected one normal component",
        ));
    }
    CString::new(name).map_err(Into::into)
}

#[allow(dead_code)] // Internal recovery seam; no public route is active.
fn open_directory(parent: &File, name: &CStr) -> io::Result<File> {
    let flags = libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC;
    // SAFETY: retained parent descriptor and one NUL-terminated component.
    let fd = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags) };
    if fd < 0 {
        Err(io::Error::last_os_error())
    } else {
        // SAFETY: successful openat transfers unique descriptor ownership.
        Ok(unsafe { File::from_raw_fd(fd) })
    }
}

#[allow(dead_code)] // Internal recovery seam; no public route is active.
fn remove_entry(parent: &File, name: &CStr) -> io::Result<()> {
    let expected = identity(parent, &name.to_owned())?;
    if expected.2 == 0o040000 {
        let directory = open_directory(parent, name)?;
        clear_directory(&directory)?;
    }
    if identity(parent, &name.to_owned())? != expected {
        return Err(changed());
    }
    let flags = if expected.2 == 0o040000 {
        libc::AT_REMOVEDIR
    } else {
        0
    };
    // SAFETY: exact retained parent and identity revalidated immediately before removal.
    if unsafe { libc::unlinkat(parent.as_raw_fd(), name.as_ptr(), flags) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[allow(dead_code)] // Internal recovery seam; no public route is active.
fn errno_ptr() -> *mut libc::c_int {
    #[cfg(target_os = "linux")]
    unsafe {
        libc::__errno_location()
    }
    #[cfg(target_os = "macos")]
    unsafe {
        libc::__error()
    }
}

#[allow(dead_code)] // Internal recovery seam; no public route is active.
fn clear_directory(directory: &File) -> io::Result<()> {
    // SAFETY: duplicates a live descriptor for fdopendir ownership.
    let fd = unsafe { libc::fcntl(directory.as_raw_fd(), libc::F_DUPFD_CLOEXEC, 0) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: fd is a directory descriptor. fdopendir owns it on success.
    let stream = unsafe { libc::fdopendir(fd) };
    if stream.is_null() {
        // SAFETY: fdopendir failed and did not consume this descriptor.
        unsafe { libc::close(fd) };
        return Err(io::Error::last_os_error());
    }
    struct Stream(*mut libc::DIR);
    impl Drop for Stream {
        fn drop(&mut self) {
            // SAFETY: this type owns exactly one fdopendir result.
            unsafe { libc::closedir(self.0) };
        }
    }
    let stream = Stream(stream);
    loop {
        // SAFETY: write errno only for this readdir EOF/error distinction.
        unsafe { *errno_ptr() = 0 };
        // SAFETY: stream remains live until this scope exits.
        let entry = unsafe { libc::readdir(stream.0) };
        if entry.is_null() {
            // SAFETY: errno pointer remains valid for current thread.
            let errno = unsafe { *errno_ptr() };
            return if errno == 0 {
                Ok(())
            } else {
                Err(io::Error::from_raw_os_error(errno))
            };
        }
        // SAFETY: readdir returned a live dirent whose d_name is NUL-terminated.
        let name = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) };
        if name.to_bytes() == b"." || name.to_bytes() == b".." {
            continue;
        }
        remove_entry(directory, name)?;
    }
}

/// Reap a root only after validating its ownership xattr through this descriptor.
/// All descendants and final unlink use descriptor-relative syscalls. A name
/// replacement after validation is detected before final unlink and preserved.
#[allow(dead_code)] // Internal recovery seam; no public route is active.
pub(super) fn reap_owned_scratch(
    parent: &File,
    name: &OsStr,
    after_validation: impl FnOnce(),
) -> io::Result<bool> {
    let name = component_os(name)?;
    let expected = match identity(parent, &name) {
        Ok(identity) => identity,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error),
    };
    if expected.2 != 0o040000 {
        return Ok(false);
    }
    let directory = open_directory(parent, &name)?;
    if !has_ownership(&directory)? {
        return Ok(false);
    }
    clear_directory(&directory)?;
    after_validation();
    if identity(parent, &name)? != expected {
        return Err(changed());
    }
    // SAFETY: owned root identity was revalidated against retained staging parent.
    if unsafe { libc::unlinkat(parent.as_raw_fd(), name.as_ptr(), libc::AT_REMOVEDIR) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(true)
}

fn component(name: &str) -> io::Result<CString> {
    if name.is_empty() || name == "." || name == ".." || name.contains('/') {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "expected one normal component",
        ));
    }
    CString::new(name).map_err(Into::into)
}

fn changed() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "owned scratch name changed identity or type",
    )
}

fn identity(parent: &File, name: &CString) -> io::Result<(u64, u64, u32)> {
    let mut info = std::mem::MaybeUninit::<libc::stat>::uninit();
    // SAFETY: live parent, NUL-terminated component, correctly sized output.
    // Do not follow the final entry; initialize output only on native success.
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

struct OwnedName {
    parent: File,
    name: CString,
    identity: (u64, u64, u32),
    attempted: bool,
}
impl OwnedName {
    fn remove(&mut self) -> io::Result<()> {
        // One attempt only, including on error. Never blindly retry during Drop.
        self.attempted = true;
        if identity(&self.parent, &self.name)? != self.identity {
            return Err(changed());
        }
        let flags = if self.identity.2 == 0o040000 {
            libc::AT_REMOVEDIR
        } else {
            0
        };
        // SAFETY: one retained parent and exact component owned by this guard.
        // Directory removal is NONRECURSIVE; a new unknown child prevents it.
        // Stable private ownership is a precondition, not a hostile-writer lock.
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

pub(super) struct Directory {
    file: File,
    owned: OwnedName,
}
impl Directory {
    pub(super) fn reserve(parent: &File, wait: Wait<'_>) -> io::Result<Self> {
        Self::reserve_with(parent, wait, || {
            format!(
                ".tmp-{}-tree-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            )
        })
    }

    fn reserve_with(
        parent: &File,
        wait: Wait<'_>,
        mut name: impl FnMut() -> String,
    ) -> io::Result<Self> {
        for _ in 0..32 {
            wait.check()?;
            match Self::create(parent, &name()) {
                Ok(directory) => return Ok(directory),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => (),
                Err(error) => return Err(error),
            }
        }
        Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "scratch reservation exhausted 32 candidates",
        ))
    }

    fn create(parent: &File, name: &str) -> io::Result<Self> {
        let name = component(name)?;
        let parent = parent.try_clone()?;
        // SAFETY: single component and a live directory handle. Atomic mkdir
        // reserves a NEW name; AlreadyExists never adopts another allocation.
        if unsafe { libc::mkdirat(parent.as_raw_fd(), name.as_ptr(), 0o700) } != 0 {
            return Err(io::Error::last_os_error());
        }
        // If identity acquisition fails, leave the allocation rather than
        // guessing ownership for destructive cleanup; the I/O error propagates.
        let id = identity(&parent, &name)?;
        if id.2 != 0o040000 {
            return Err(changed());
        }
        let owned = OwnedName {
            parent,
            name,
            identity: id,
            attempted: false,
        };
        let flags = libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC;
        // SAFETY: owned strings/descriptor; success transfers the new fd to File.
        let fd = unsafe { libc::openat(owned.parent.as_raw_fd(), owned.name.as_ptr(), flags) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        let file = unsafe { File::from_raw_fd(fd) };
        let meta = file.metadata()?;
        if (meta.dev(), meta.ino(), meta.mode() & 0o170000) != id {
            return Err(changed());
        }
        Ok(Self { file, owned })
    }

    pub(super) fn directory(&self, name: &str) -> io::Result<Self> {
        Self::create(&self.file, name)
    }

    pub(super) fn mark_owned_scratch(&self) -> io::Result<()> {
        set_ownership(&self.file)
    }

    pub(super) fn file(&self, name: &str) -> io::Result<RegularFile> {
        let name = component(name)?;
        let parent = self.file.try_clone()?;
        let flags = libc::O_RDWR | libc::O_CREAT | libc::O_NOFOLLOW | libc::O_CLOEXEC;
        // SAFETY: live directory, single component, explicit mode with O_CREAT.
        // O_EXCL rejects ALL existing names, including links and directories.
        let fd = unsafe {
            libc::openat(
                parent.as_raw_fd(),
                name.as_ptr(),
                flags | libc::O_EXCL,
                0o600,
            )
        };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        let file = unsafe { File::from_raw_fd(fd) };
        let meta = file.metadata()?;
        let id = (meta.dev(), meta.ino(), meta.mode() & 0o170000);
        if !meta.is_file() {
            return Err(changed());
        }
        Ok(RegularFile {
            file,
            owned: OwnedName {
                parent,
                name,
                identity: id,
                attempted: false,
            },
        })
    }

    pub(super) fn finish(mut self) -> io::Result<()> {
        self.owned.remove()
    }
}

pub(super) struct RegularFile {
    file: File,
    owned: OwnedName,
}
impl RegularFile {
    pub(super) fn rewind(&mut self) -> io::Result<()> {
        self.file.rewind()
    }
    pub(super) fn finish(mut self) -> io::Result<()> {
        self.owned.remove()
    }
}
impl Read for RegularFile {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        self.file.read(bytes)
    }
}
impl Write for RegularFile {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.file.write(bytes)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scratch_atomic_reservation_has_a_finite_collision_budget() {
        let fixture = tempfile::TempDir::new().unwrap();
        std::fs::create_dir(fixture.path().join("occupied")).unwrap();
        std::fs::write(fixture.path().join("occupied/retained"), b"original").unwrap();
        let parent = File::open(fixture.path()).unwrap();
        let mut attempts = 0;
        let result = Directory::reserve_with(&parent, Wait::Try, || {
            attempts += 1;
            "occupied".into()
        });
        assert_eq!(result.err().unwrap().kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(attempts, 32);
        assert_eq!(
            std::fs::read(fixture.path().join("occupied/retained")).unwrap(),
            b"original"
        );
        assert_eq!(std::fs::read_dir(fixture.path()).unwrap().count(), 1);
    }

    #[test]
    fn scratch_owned_descriptors_are_close_on_exec() {
        let fixture = tempfile::TempDir::new().unwrap();
        let parent = File::open(fixture.path()).unwrap();
        let directory = Directory::reserve(&parent, Wait::Try).unwrap();
        let file = directory.file("probe").unwrap();
        for handle in [
            &directory.file,
            &directory.owned.parent,
            &file.file,
            &file.owned.parent,
        ] {
            // SAFETY: only query a live, borrowed native descriptor.
            let flags = unsafe { libc::fcntl(handle.as_raw_fd(), libc::F_GETFD) };
            assert!(flags >= 0);
            assert_ne!(flags & libc::FD_CLOEXEC, 0);
        }
        file.finish().unwrap();
        directory.finish().unwrap();
    }
}
