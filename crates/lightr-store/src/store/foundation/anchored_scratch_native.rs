//! Owned, single-component mutations inside a private, lease-protected reserve.
use super::Wait;
use std::ffi::CString;
use std::fs::File;
use std::io::{self, Read, Seek, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::MetadataExt;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

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
