//! Native clone into a nonexistent leaf in an exclusively owned directory.
//! Source handles remain borrowed for the syscall. No public path resolution.
use crate::store::cow::CowRung;
use std::fs::File;
use std::io;
use std::path::Path;

pub(super) fn can_fallback(error: &io::Error) -> bool {
    if error.kind() == io::ErrorKind::Unsupported {
        return true;
    }
    #[cfg(unix)]
    if let Some(code) = error.raw_os_error() {
        return [
            libc::EXDEV,
            libc::ENOTTY,
            libc::EOPNOTSUPP,
            libc::ENOSYS,
            libc::EINVAL,
        ]
        .contains(&code);
    }
    #[cfg(windows)]
    if let Some(code) = error.raw_os_error() {
        // ERROR_INVALID_FUNCTION, NOT_SAME_DEVICE, NOT_SUPPORTED,
        // INVALID_PARAMETER (e.g. unsupported extent alignment).
        return [1, 17, 50, 87].contains(&code);
    }
    false
}

#[cfg(any(target_os = "linux", windows))]
fn new_output(path: &Path) -> io::Result<File> {
    let mut options = std::fs::OpenOptions::new();
    options.read(true).write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path)
}

/// Apple's fclonefileat takes the already-open source, never reopens its path.
/// https://github.com/apple-oss-distributions/xnu/blob/main/bsd/man/man2/clonefile.2
#[cfg(target_os = "macos")]
pub(super) fn clone_file(
    source: &File,
    path: &Path,
    _checkpoint: &dyn Fn() -> io::Result<()>,
) -> io::Result<(File, CowRung)> {
    use std::ffi::CString;
    use std::os::fd::AsRawFd;
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
    let destination = CString::new(path.as_os_str().as_encoded_bytes())
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
    // SAFETY: source owns a valid fd; destination is NUL-terminated and lives
    // through the call. The private destination leaf is absent. No raw fd is
    // converted to an owned File and no source attributes are changed.
    if unsafe { libc::fclonefileat(source.as_raw_fd(), libc::AT_FDCWD, destination.as_ptr(), 0) }
        != 0
    {
        return Err(io::Error::last_os_error());
    }
    let read = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)?;
    // clone inherits source mode; change ONLY the newly owned output to allow
    // the required writable flush handle. Stable managed ancestors are assumed.
    read.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)?;
    crate::store::foundation::verify_file_path(&read, path)?;
    crate::store::foundation::verify_file_path(&file, path)?;
    Ok((file, CowRung::Clone))
}

#[cfg(target_os = "linux")]
pub(super) fn clone_file(
    source: &File,
    path: &Path,
    _checkpoint: &dyn Fn() -> io::Result<()>,
) -> io::Result<(File, CowRung)> {
    use std::os::fd::AsRawFd;
    let file = new_output(path)?;
    // SAFETY: both File owners keep their fds live; request is the FICLONE
    // Linux ABI constant and has a scalar source fd, not a user pointer.
    // `as _` selects libc's glibc/musl request argument type (constant fits both).
    if unsafe { libc::ioctl(file.as_raw_fd(), 0x4004_9409 as _, source.as_raw_fd()) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok((file, CowRung::Reflink))
}

/// Actual FSCTL result, never a claim inferred from file-system naming.
/// https://learn.microsoft.com/en-us/windows/win32/api/winioctl/ns-winioctl-duplicate_extents_data
#[cfg(windows)]
pub(super) fn clone_file(
    source: &File,
    path: &Path,
    _checkpoint: &dyn Fn() -> io::Result<()>,
) -> io::Result<(File, CowRung)> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::System::Ioctl::{
        DUPLICATE_EXTENTS_DATA, FSCTL_DUPLICATE_EXTENTS_TO_FILE,
    };
    use windows_sys::Win32::System::IO::DeviceIoControl;
    let length = i64::try_from(source.metadata()?.len())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "clone length exceeds i64"))?;
    let file = new_output(path)?;
    file.set_len(length as u64)?;
    let mut offset = 0;
    while offset < length {
        _checkpoint()?;
        // Each request stays below the native per-request limit. Unsupported
        // final extent alignment is a reported fallback, not invented success.
        let count = (length - offset).min(1024 * 1024 * 1024);
        let data = DUPLICATE_EXTENTS_DATA {
            FileHandle: source.as_raw_handle() as _,
            SourceFileOffset: offset,
            TargetFileOffset: offset,
            ByteCount: count,
        };
        let mut returned = 0;
        // SAFETY: typed input and result storage outlive the synchronous call;
        // both handles have live File owners, output is uniquely read/write.
        let ok = unsafe {
            DeviceIoControl(
                file.as_raw_handle() as _,
                FSCTL_DUPLICATE_EXTENTS_TO_FILE,
                &data as *const _ as _,
                std::mem::size_of_val(&data) as u32,
                std::ptr::null_mut(),
                0,
                &mut returned,
                std::ptr::null_mut(),
            )
        };
        if ok == 0 {
            return Err(io::Error::last_os_error());
        }
        _checkpoint()?;
        offset += count;
    }
    Ok((file, CowRung::RefsBlockClone))
}

#[cfg(not(any(target_os = "macos", target_os = "linux", windows)))]
pub(super) fn clone_file(
    _: &File,
    _: &Path,
    _: &dyn Fn() -> io::Result<()>,
) -> io::Result<(File, CowRung)> {
    Err(io::ErrorKind::Unsupported.into())
}
