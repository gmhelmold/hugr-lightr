use std::ffi::CStr;
use std::fs::File;
use std::io;
use std::os::fd::{AsRawFd, FromRawFd, IntoRawFd};
use std::ptr::NonNull;

pub(super) fn require_child_count(directory: &File, expected: usize) -> io::Result<()> {
    let dot = std::ffi::CString::new(".").expect("static directory component");
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
