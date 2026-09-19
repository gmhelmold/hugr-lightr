//! Read directory entries through an existing handle, without reopening its name.
//! This does not lock directory contents or authorize later output mutations.
use super::super::Wait;
use super::topology_io::open_at;
use std::ffi::CStr;
use std::fs::File;
use std::io;
use std::os::fd::{AsRawFd, IntoRawFd};
use std::ptr::NonNull;

#[derive(Clone, Copy)]
enum Entry {
    Dot,
    Other,
    End,
}

pub(super) fn require_empty(directory: &File, wait: Wait<'_>) -> io::Result<()> {
    wait.check()?;
    // A duplicate shares the caller's directory offset. Opening "." relative to
    // the retained handle gives an independent stream of the SAME directory.
    let scan = open_at(directory, b".", true)?;
    wait.check()?;
    let mut stream = DirectoryStream::open(scan)?;
    let result = scan_empty(wait, || stream.next());
    // Always close. Preserve a read/cancellation error if close also fails; a
    // close failure after a successful scan must not be represented as success.
    let closed = stream.close();
    result.and(closed)?;
    wait.check()
}

fn scan_empty(wait: Wait<'_>, mut next: impl FnMut() -> io::Result<Entry>) -> io::Result<()> {
    // An empty directory has at most '.' and '..', possibly neither. Bound a
    // malformed repeating-dot stream without allocating or collecting names.
    for _ in 0..3 {
        wait.check()?;
        let entry = next()?;
        wait.check()?; // Cancellation after EOF is still cancellation.
        match entry {
            Entry::Dot => (),
            Entry::Other => return Err(io::Error::from_raw_os_error(libc::ENOTEMPTY)),
            Entry::End => return Ok(()),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::InvalidData,
        "directory stream repeated dot entries",
    ))
}

struct DirectoryStream(Option<NonNull<libc::DIR>>);
impl DirectoryStream {
    fn open(file: File) -> io::Result<Self> {
        // SAFETY: the File is an owned, readable directory opened at offset zero.
        // On failure fdopendir leaves it owned by File. On success the stream
        // owns the fd, so the File must not close it independently.
        let pointer = unsafe { libc::fdopendir(file.as_raw_fd()) };
        let pointer = NonNull::new(pointer).ok_or_else(io::Error::last_os_error)?;
        let _ = file.into_raw_fd();
        Ok(Self(Some(pointer)))
    }

    fn next(&mut self) -> io::Result<Entry> {
        let pointer = self.0.expect("open directory stream");
        // SAFETY: this stream is locally owned and never shared across threads.
        // readdir's entry is borrowed only until the next call. Native d_name is
        // NUL-terminated. Clear this THREAD's errno to distinguish EOF from error.
        unsafe {
            *errno_address() = 0;
            let entry = libc::readdir(pointer.as_ptr());
            if entry.is_null() {
                let code = *errno_address();
                return if code == 0 {
                    Ok(Entry::End)
                } else {
                    Err(io::Error::from_raw_os_error(code))
                };
            }
            let name = CStr::from_ptr((*entry).d_name.as_ptr()).to_bytes();
            Ok(if name == b"." || name == b".." {
                Entry::Dot
            } else {
                Entry::Other
            })
        }
    }

    fn close(mut self) -> io::Result<()> {
        let pointer = self.0.take().expect("open directory stream");
        // SAFETY: consume the sole stream owner exactly once. Never retry a
        // failed close or close the underlying fd separately (it may be reused).
        if unsafe { libc::closedir(pointer.as_ptr()) } == 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }
}
impl Drop for DirectoryStream {
    fn drop(&mut self) {
        if let Some(pointer) = self.0.take() {
            // SAFETY: panic fallback only; normal paths use checked close above.
            let _ = unsafe { libc::closedir(pointer.as_ptr()) };
        }
    }
}

#[cfg(target_os = "linux")]
unsafe fn errno_address() -> *mut libc::c_int {
    unsafe { libc::__errno_location() }
}
#[cfg(target_os = "macos")]
unsafe fn errno_address() -> *mut libc::c_int {
    unsafe { libc::__error() }
}

#[cfg(test)]
#[path = "topology_empty_tests.rs"]
mod tests;
