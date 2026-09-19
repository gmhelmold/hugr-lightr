//! Read-only anchored descent, not namespace exclusion or a recursive capture.
use super::super::Wait;
use super::topology_io::{key, unsupported, Key};
use super::{SourcePath, TopologyInspection};
use std::ffi::CString;
use std::fs::File;
use std::io;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::{ffi::OsStrExt, fs::MetadataExt};

pub(super) fn open(
    inspection: &TopologyInspection,
    relative: SourcePath<'_>,
    wait: Wait<'_>,
) -> io::Result<File> {
    open_checked(inspection, relative, wait, |_| Ok(()))
}

// A scoped observation seam coordinates tests with actual native operations.
// Production adds no process-global hooks, sleeps, retries or cwd mutation.
fn open_checked(
    inspection: &TopologyInspection,
    relative: SourcePath<'_>,
    wait: Wait<'_>,
    mut after_open: impl FnMut(usize) -> io::Result<()>,
) -> io::Result<File> {
    wait.check()?;
    let (path, directory) = match relative {
        SourcePath::Directory(path) => (path, true),
        SourcePath::File(path) => (path, false),
    };
    let bytes = path.as_os_str().as_bytes();
    if bytes.len() > 4096 {
        return Err(unsupported("descendant path work budget exceeded"));
    }
    if bytes.is_empty()
        || bytes.contains(&0)
        || bytes
            .split(|b| *b == b'/')
            .any(|p| p.is_empty() || p == b"." || p == b"..")
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "descendant path requires exact normal relative components",
        ));
    }
    let count = bytes.split(|b| *b == b'/').count();
    if count > 128 {
        return Err(unsupported("descendant path work budget exceeded"));
    }
    inspection.revalidate(wait)?;
    let root = inspection.source_handle();
    if !root.metadata()?.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "descendant source is not a directory",
        ));
    }
    let root_key = key(root)?;
    let mut opened: Vec<(File, Key)> = Vec::new();
    opened
        .try_reserve_exact(count)
        .map_err(|e| io::Error::new(io::ErrorKind::OutOfMemory, e))?;
    for (index, name) in bytes.split(|b| *b == b'/').enumerate() {
        wait.check()?;
        let parent = opened.last().map_or(root, |item| &item.0);
        let file = open_component(parent, name, index + 1 != count || directory)?;
        let identity = key(&file)?;
        if !identity.same_volume(root_key) {
            return Err(unsupported("descendant crosses a native mount boundary"));
        }
        opened.push((file, identity));
        after_open(index)?;
        wait.check()?;
    }
    // Reopen each single component relative to the SAME retained parent, never
    // the original global pathname. A substituted directory or entry is rejected.
    for (index, name) in bytes.split(|b| *b == b'/').enumerate() {
        wait.check()?;
        let parent = if index == 0 {
            root
        } else {
            &opened[index - 1].0
        };
        let current = open_component(parent, name, index + 1 != count || directory)?;
        if key(&current)? != opened[index].1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "descendant identity changed during acquisition",
            ));
        }
    }
    inspection.revalidate(wait)?;
    wait.check()?;
    // Nonempty normal input guarantees one handle. Other held ancestors close
    // here; the returned File continues to identify its own opened object.
    Ok(opened.pop().expect("validated nonempty descendant").0)
}

fn open_component(parent: &File, name: &[u8], directory: bool) -> io::Result<File> {
    let name = CString::new(name)?;
    let flags = libc::O_RDONLY
        | libc::O_CLOEXEC
        | libc::O_NOFOLLOW
        | libc::O_NONBLOCK
        | if directory { libc::O_DIRECTORY } else { 0 };
    // SAFETY: live borrowed parent and one NUL-terminated component. No create,
    // truncate, write, chmod or unlink; each successful fd is owned exactly once.
    // O_NOFOLLOW applies to every component because each call has only one name.
    let raw = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags) };
    if raw < 0 {
        return Err(io::Error::last_os_error());
    }
    let file = unsafe { File::from_raw_fd(raw) };
    let metadata = file.metadata()?;
    if (directory && !metadata.is_dir()) || (!directory && !metadata.is_file()) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "descendant has the wrong native type",
        ));
    }
    if !directory && metadata.nlink() != 1 {
        return Err(unsupported("multiply linked or unlinked source descendant"));
    }
    Ok(file)
}

#[cfg(test)]
#[path = "topology_descendant_tests.rs"]
mod tests;
