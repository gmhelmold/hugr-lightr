//! One privately owned directory stream; no content writes or entry traversal.
use super::super::topology_io::{key, open_at};
use super::super::SourcePath;
use super::{DirectoryLimits, TopologyInspection, Wait};
use std::ffi::CStr;
use std::fs::File;
use std::io;
use std::os::fd::{AsRawFd, IntoRawFd};
use std::path::Path;
use std::ptr::NonNull;

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn open_directory(
    inspection: &TopologyInspection,
    relative: Option<&Path>,
    wait: Wait<'_>,
) -> io::Result<File> {
    match relative {
        Some(path) => inspection.open_source_descendant(SourcePath::Directory(path), wait),
        None => inspection.source_handle().try_clone(),
    }
}

pub(super) fn list_checked(
    inspection: &TopologyInspection,
    relative: Option<&Path>,
    limits: DirectoryLimits,
    wait: Wait<'_>,
    after_read: impl FnMut() -> io::Result<()>,
) -> io::Result<Vec<String>> {
    wait.check()?;
    inspection.revalidate(wait)?;
    let directory = open_directory(inspection, relative, wait)?;
    let identity = key(&directory)?;
    wait.check()?;
    // Keep the original directory pinned and use a NEW open-file description.
    // dup/try_clone would share an offset advanced by a previous enumeration.
    let scan = open_at(&directory, b".", true)?;
    let mut stream = Stream::open(scan)?;
    let names = collect(&mut stream, limits, wait, after_read);
    let closed = stream.close();
    // A scan error remains primary. Close is checked even on scan failure, and
    // a close failure after successful enumeration cannot become success.
    let names = finish_scan(names, closed)?;
    wait.check()?;
    inspection.revalidate(wait)?;
    let current = open_directory(inspection, relative, wait)?;
    if key(&current)? != identity {
        return Err(invalid("listed directory changed identity"));
    }
    wait.check()?;
    Ok(names)
}

fn finish_scan(names: io::Result<Vec<String>>, closed: io::Result<()>) -> io::Result<Vec<String>> {
    names.and_then(|names| closed.map(|()| names))
}

trait Names {
    // The slice is used only until the next read on this locally owned stream.
    fn next_name(&mut self) -> io::Result<Option<&[u8]>>;
}

fn collect(
    stream: &mut impl Names,
    limits: DirectoryLimits,
    wait: Wait<'_>,
    mut after_read: impl FnMut() -> io::Result<()>,
) -> io::Result<Vec<String>> {
    let mut names = Vec::new();
    let mut bytes = 0usize;
    let (mut dot, mut parent) = (false, false);
    loop {
        wait.check()?;
        let entry = stream.next_name()?;
        after_read()?;
        wait.check()?; // EOF does not erase a cancellation/deadline.
        let Some(name) = entry else { break };
        if name == b"." || name == b".." {
            let seen = if name == b"." { &mut dot } else { &mut parent };
            if *seen {
                return Err(invalid("directory stream repeated a dot entry"));
            }
            *seen = true;
            continue;
        }
        if name.is_empty() || name.contains(&0) || name.contains(&b'/') {
            return Err(invalid("invalid native directory component"));
        }
        if names.len() >= limits.max_entries {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "directory entry budget exceeded",
            ));
        }
        bytes = bytes
            .checked_add(name.len())
            .filter(|n| *n <= limits.max_name_bytes)
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "directory name-byte budget exceeded",
                )
            })?;
        let name =
            std::str::from_utf8(name).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        names.try_reserve(1).map_err(allocation)?;
        let mut owned = String::new();
        owned.try_reserve_exact(name.len()).map_err(allocation)?;
        owned.push_str(name);
        names.push(owned);
    }
    wait.check()?;
    names.sort_unstable();
    if names.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(invalid("duplicate directory-name observation"));
    }
    wait.check()?;
    Ok(names)
}

fn allocation(error: std::collections::TryReserveError) -> io::Error {
    io::Error::new(io::ErrorKind::OutOfMemory, error)
}

struct Stream(Option<NonNull<libc::DIR>>);
impl Stream {
    fn open(file: File) -> io::Result<Self> {
        // SAFETY: a readable owned directory; fdopendir acquires descriptor
        // ownership on success only. Failed calls leave File responsible.
        let raw = unsafe { libc::fdopendir(file.as_raw_fd()) };
        let raw = NonNull::new(raw).ok_or_else(io::Error::last_os_error)?;
        let _ = file.into_raw_fd();
        Ok(Self(Some(raw)))
    }
    fn close(mut self) -> io::Result<()> {
        let raw = self.0.take().expect("open directory stream");
        // SAFETY: sole owner consumed exactly once. Never retry close or close
        // the underlying descriptor independently after uncertain disposal.
        if unsafe { libc::closedir(raw.as_ptr()) } == 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }
}
impl Names for Stream {
    fn next_name(&mut self) -> io::Result<Option<&[u8]>> {
        let raw = self.0.expect("open directory stream");
        // SAFETY: local, unshared stream. readdir supplies a NUL-terminated
        // name borrowed until the next call; no d_type/d_ino trust is inferred.
        unsafe {
            *errno_address() = 0;
            let entry = libc::readdir(raw.as_ptr());
            if entry.is_null() {
                let code = *errno_address();
                return if code == 0 {
                    Ok(None)
                } else {
                    Err(io::Error::from_raw_os_error(code))
                };
            }
            Ok(Some(CStr::from_ptr((*entry).d_name.as_ptr()).to_bytes()))
        }
    }
}
impl Drop for Stream {
    fn drop(&mut self) {
        if let Some(raw) = self.0.take() {
            // SAFETY: panic cleanup, exactly once. Ordinary paths check close.
            let _ = unsafe { libc::closedir(raw.as_ptr()) };
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
#[path = "topology_listing_tests.rs"]
mod tests;
