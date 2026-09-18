//! Small read-only native operations. No create, unlink, chmod or chdir.
#[cfg(target_os = "macos")]
use std::ffi::CStr;
use std::ffi::CString;
use std::fs::File;
use std::io;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::MetadataExt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct Key {
    pub(super) device: u64,
    pub(super) inode: u64,
    pub(super) mount: u64,
}
impl Key {
    pub(super) fn same_volume(self, other: Self) -> bool {
        self.device == other.device && self.mount == other.mount
    }
}

pub(super) fn unsupported(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::Unsupported, message)
}

pub(super) fn open_at(parent: &File, name: &[u8], directory: bool) -> io::Result<File> {
    let name = CString::new(name)?;
    let flags = libc::O_RDONLY
        | libc::O_CLOEXEC
        | libc::O_NONBLOCK
        | if directory {
            libc::O_DIRECTORY
        } else {
            libc::O_NOFOLLOW
        };
    // SAFETY: parent owns a live descriptor; name is NUL-terminated and borrowed
    // for the call. No creation flag/mode. A successful new descriptor is owned
    // exactly once by File; a failed descriptor is never wrapped.
    let fd = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags) };
    if fd < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(unsafe { File::from_raw_fd(fd) })
    }
}

pub(super) fn definitely_absent(parent: &File, name: &[u8]) -> io::Result<bool> {
    let name = CString::new(name)?;
    let mut info = std::mem::MaybeUninit::<libc::stat>::uninit();
    // SAFETY: valid descriptor/string/output storage; output is never read.
    // A dangling link is PRESENT, never permission to invent a missing suffix.
    let result = unsafe {
        libc::fstatat(
            parent.as_raw_fd(),
            name.as_ptr(),
            info.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if result == 0 {
        return Ok(false);
    }
    let error = io::Error::last_os_error();
    if error.kind() == io::ErrorKind::NotFound {
        Ok(true)
    } else {
        Err(error)
    }
}

pub(super) fn key(file: &File) -> io::Result<Key> {
    let metadata = file.metadata()?;
    Ok(Key {
        device: metadata.dev(),
        inode: metadata.ino(),
        mount: mount(file)?,
    })
}

#[cfg(target_os = "linux")]
fn mount(file: &File) -> io::Result<u64> {
    let mut fs = std::mem::MaybeUninit::<libc::statfs>::uninit();
    // SAFETY: live descriptor and correctly sized output, read only on success.
    if unsafe { libc::fstatfs(file.as_raw_fd(), fs.as_mut_ptr()) } != 0 {
        return Err(io::Error::last_os_error());
    }
    let fs = unsafe { fs.assume_init() };
    // Allowed local families for read-only inspection, not durability: ext,
    // XFS, Btrfs, tmpfs and overlay. Network/unknown filesystems are rejected.
    if !matches!(
        fs.f_type as u64,
        0xef53 | 0x58465342 | 0x9123683e | 0x01021994 | 0x794c7630
    ) {
        return Err(unsupported("unqualified topology filesystem"));
    }
    let mut stat = std::mem::MaybeUninit::<libc::statx>::uninit();
    let empty = c"";
    // SAFETY: AT_EMPTY_PATH addresses the live handle. A non-null empty string
    // works on kernels before 6.11 too. Requested data is checked via stx_mask.
    if unsafe {
        libc::statx(
            file.as_raw_fd(),
            empty.as_ptr(),
            libc::AT_EMPTY_PATH,
            libc::STATX_MNT_ID,
            stat.as_mut_ptr(),
        )
    } != 0
    {
        return Err(io::Error::last_os_error());
    }
    let stat = unsafe { stat.assume_init() };
    if stat.stx_mask & libc::STATX_MNT_ID == 0 {
        return Err(unsupported("native mount identity unavailable"));
    }
    Ok(stat.stx_mnt_id)
}

#[cfg(target_os = "macos")]
fn mount(file: &File) -> io::Result<u64> {
    let mut fs = std::mem::MaybeUninit::<libc::statfs>::uninit();
    // SAFETY: live descriptor and correctly sized output; only read on success.
    if unsafe { libc::fstatfs(file.as_raw_fd(), fs.as_mut_ptr()) } != 0 {
        return Err(io::Error::last_os_error());
    }
    let fs = unsafe { fs.assume_init() };
    let name: Vec<u8> = fs.f_fstypename.iter().map(|c| *c as u8).collect();
    let name = CStr::from_bytes_until_nul(&name)
        .map_err(|_| unsupported("unterminated filesystem identifier"))?;
    if name.to_bytes() != b"apfs" {
        return Err(unsupported(
            "only APFS topology inspection is qualified on macOS",
        ));
    }
    Ok(file.metadata()?.dev())
}
