//! Bounded link-text observation, without following the final symbolic link.
use super::super::Wait;
use super::topology_io::{key, unsupported, Key};
use super::{SourcePath, TopologyInspection};
use std::ffi::{CStr, CString, OsStr};
use std::fs::File;
use std::io;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Stage {
    Checked,
    Pinned,
    Read,
    Validated,
}

pub(super) fn read(
    inspection: &TopologyInspection,
    relative: &Path,
    limit: usize,
    wait: Wait<'_>,
) -> io::Result<String> {
    read_checked(inspection, relative, limit, wait, |_| Ok(()))
}

fn read_checked(
    inspection: &TopologyInspection,
    relative: &Path,
    limit: usize,
    wait: Wait<'_>,
    mut checkpoint: impl FnMut(Stage) -> io::Result<()>,
) -> io::Result<String> {
    wait.check()?;
    let bytes = relative.as_os_str().as_bytes();
    if limit == 0 || limit > u16::MAX as usize {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid link target byte budget",
        ));
    }
    if bytes.len() > 4096 || bytes.split(|b| *b == b'/').count() > 128 {
        return Err(unsupported("source link path work budget exceeded"));
    }
    if bytes.is_empty()
        || bytes.contains(&0)
        || bytes
            .split(|b| *b == b'/')
            .any(|p| p.is_empty() || p == b"." || p == b"..")
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "source link requires normal relative components",
        ));
    }
    inspection.revalidate(wait)?;
    let (prefix, leaf) = match bytes.iter().rposition(|b| *b == b'/') {
        Some(index) => (&bytes[..index], &bytes[index + 1..]),
        None => (&b""[..], bytes),
    };
    let parent = open_parent(inspection, prefix, wait)?;
    let parent_key = key(&parent)?;
    let name = CString::new(leaf)?;
    let pinned = pin(&parent, &name, || checkpoint(Stage::Checked))?;
    let expected = key(&pinned)?;
    if !expected.same_volume(parent_key) {
        return Err(unsupported("source link crosses a native mount boundary"));
    }
    checkpoint(Stage::Pinned)?;
    wait.check()?;
    check_binding(&parent, &name, expected, || checkpoint(Stage::Checked))?;
    let target = read_target(&parent, &name, &pinned, limit)?;
    checkpoint(Stage::Read)?;
    wait.check()?;
    check_binding(&parent, &name, expected, || checkpoint(Stage::Checked))?;
    let current_parent = open_parent(inspection, prefix, wait)?;
    if key(&current_parent)? != parent_key {
        return Err(changed());
    }
    inspection.revalidate(wait)?;
    checkpoint(Stage::Validated)?;
    wait.check()?; // Source-link final checkpoint, including completed reads.
    Ok(target)
}

fn changed() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "source link or parent identity changed",
    )
}

fn open_parent(inspection: &TopologyInspection, prefix: &[u8], wait: Wait<'_>) -> io::Result<File> {
    if !inspection.source_handle().metadata()?.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "source link root is not a directory",
        ));
    }
    if prefix.is_empty() {
        wait.check()?;
        inspection.source_handle().try_clone()
    } else {
        inspection.open_source_descendant(
            SourcePath::Directory(Path::new(OsStr::from_bytes(prefix))),
            wait,
        )
    }
}

fn check_binding(
    parent: &File,
    name: &CStr,
    expected: Key,
    before_open: impl FnMut() -> io::Result<()>,
) -> io::Result<()> {
    let current = pin(parent, name, before_open)?;
    if key(&current)? != expected {
        return Err(changed());
    }
    Ok(())
}

fn pin(
    parent: &File,
    name: &CStr,
    mut before_open: impl FnMut() -> io::Result<()>,
) -> io::Result<File> {
    let mut info = std::mem::MaybeUninit::<libc::stat>::uninit();
    // SAFETY: live parent, exact single NUL-terminated component, sized output.
    // Refuse known nonlinks before opening; this is not hostile path confinement.
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
    if info.st_mode & libc::S_IFMT != libc::S_IFLNK {
        return Err(io::Error::from_raw_os_error(libc::EINVAL));
    }
    before_open()?;
    #[cfg(target_os = "linux")]
    let flags = libc::O_PATH | libc::O_NOFOLLOW | libc::O_CLOEXEC;
    // O_SYMLINK selects the link itself. Combining it with O_NOFOLLOW on Darwin
    // rejects links (ELOOP), rather than giving the Linux O_PATH behavior.
    #[cfg(target_os = "macos")]
    let flags =
        libc::O_RDONLY | libc::O_SYMLINK | libc::O_CLOEXEC | libc::O_NONBLOCK | libc::O_NOCTTY;
    // SAFETY: no create/write flag; wrap only a successfully returned descriptor.
    let fd = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    let file = unsafe { File::from_raw_fd(fd) };
    if !file.metadata()?.file_type().is_symlink() {
        return Err(changed());
    }
    Ok(file)
}

fn read_target(parent: &File, name: &CStr, pinned: &File, limit: usize) -> io::Result<String> {
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(limit + 1)
        .map_err(|e| io::Error::new(io::ErrorKind::OutOfMemory, e))?;
    bytes.resize(limit + 1, 0);
    #[cfg(target_os = "linux")]
    let (fd, path) = {
        let _ = (parent, name);
        (pinned.as_raw_fd(), c"") // read directly from the O_PATH symlink descriptor.
    };
    #[cfg(target_os = "macos")]
    let (fd, path) = {
        let _ = pinned; // Keeps the identity alive across both binding checks.
        (parent.as_raw_fd(), name)
    };
    // SAFETY: live fd, borrowed NUL-terminated path and initialized writable buffer.
    // readlinkat does not NUL-terminate. One extra byte distinguishes exact fit
    // from truncation; no retry or metadata-size trust supplies the answer.
    let used =
        unsafe { libc::readlinkat(fd, path.as_ptr(), bytes.as_mut_ptr().cast(), bytes.len()) };
    if used < 0 {
        return Err(io::Error::last_os_error());
    }
    let used = used as usize;
    if used > limit {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "source link target exceeds byte budget",
        ));
    }
    bytes.truncate(used);
    if bytes.is_empty() || bytes.contains(&0) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "empty or NUL-containing source link target",
        ));
    }
    String::from_utf8(bytes).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

#[cfg(test)]
#[path = "topology_link_tests.rs"]
mod tests;
