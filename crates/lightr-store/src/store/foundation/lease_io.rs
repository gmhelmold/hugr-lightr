//! Native handles for additive SI-01 locks. Handles are never cloned or exposed.
//! Requires stable, caller-managed local directories, not arbitrary public paths.
use super::lease::Wait;
use std::fs::{self, File, OpenOptions, TryLockError};
use std::io;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Identity(u64, u64);

#[cfg(unix)]
pub(super) fn identity(file: &File) -> io::Result<Identity> {
    use std::os::unix::fs::MetadataExt;
    let m = file.metadata()?;
    Ok(Identity(m.dev(), m.ino()))
}

#[cfg(windows)]
pub(super) fn identity(file: &File) -> io::Result<Identity> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{
        GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION,
    };
    let mut info = std::mem::MaybeUninit::<BY_HANDLE_FILE_INFORMATION>::uninit();
    // SAFETY: file owns a valid live handle; the API writes the full output on
    // success. No uninitialized output is read when the function reports zero.
    let ok = unsafe { GetFileInformationByHandle(file.as_raw_handle() as _, info.as_mut_ptr()) };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    let info = unsafe { info.assume_init() };
    Ok(Identity(
        u64::from(info.dwVolumeSerialNumber),
        (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow),
    ))
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn options(directory: bool) -> OpenOptions {
    let mut o = OpenOptions::new();
    o.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        o.custom_flags(
            libc::O_NOFOLLOW | libc::O_NONBLOCK | if directory { libc::O_DIRECTORY } else { 0 },
        );
        o.mode(0o600);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::{
            FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
        };
        o.custom_flags(
            FILE_FLAG_OPEN_REPARSE_POINT
                | if directory {
                    FILE_FLAG_BACKUP_SEMANTICS
                } else {
                    0
                },
        );
    }
    o
}

fn check_type(file: &File, directory: bool) -> io::Result<()> {
    let meta = file.metadata()?;
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;
        if meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return Err(invalid("managed lock namespace is a reparse point"));
        }
    }
    if meta.is_dir() != directory || (!directory && !meta.is_file()) {
        return Err(invalid("managed lock object has the wrong type"));
    }
    Ok(())
}

#[derive(Debug)]
pub(super) struct Directory {
    path: PathBuf,
    _handle: File,
    id: Identity,
}

impl Directory {
    pub(super) fn open(path: &Path) -> io::Result<Self> {
        let path = fs::canonicalize(path)?;
        let handle = options(true).open(&path)?;
        check_type(&handle, true)?;
        Ok(Self {
            id: identity(&handle)?,
            path,
            _handle: handle,
        })
    }

    pub(super) fn path(&self) -> &Path {
        &self.path
    }
    pub(super) fn id(&self) -> Identity {
        self.id
    }

    pub(super) fn verify(&self) -> io::Result<()> {
        let file = options(true).open(&self.path)?;
        check_type(&file, true)?;
        if identity(&file)? != self.id {
            return Err(invalid("managed directory identity changed"));
        }
        Ok(())
    }

    /// Checked namespace barrier on the declared native profile. Windows has
    /// no portable directory-entry durability promise in ADR-0020.
    pub(super) fn sync_using(&self, sync: &dyn Fn(&File) -> io::Result<()>) -> io::Result<()> {
        self.verify()?;
        #[cfg(unix)]
        sync(&self._handle)?;
        #[cfg(windows)]
        let _ = sync;
        Ok(())
    }

    pub(super) fn same_filesystem(&self, other: &Self) -> bool {
        self.id.0 == other.id.0
    }

    pub(super) fn open_file(&self, name: &str) -> io::Result<Option<File>> {
        self.verify()?;
        let path = self.path.join(name);
        let file = match options(false).open(&path) {
            Ok(file) => file,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e),
        };
        check_type(&file, false)?;
        self.verify()?;
        Ok(Some(file))
    }

    pub(super) fn child(&self, name: &str) -> io::Result<Self> {
        self.verify()?;
        let path = self.path.join(name);
        let mut builder = fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        match builder.create(&path) {
            Ok(()) => (),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => (),
            Err(e) => return Err(e),
        }
        // Deliberately do not canonicalize a managed child: aliases are allowed
        // for the supplied root only, never for internal lock/staging entries.
        let handle = options(true).open(&path)?;
        check_type(&handle, true)?;
        self.verify()?;
        Ok(Self {
            id: identity(&handle)?,
            path,
            _handle: handle,
        })
    }
}

/// Owns one acquired lock; aliases of the OS descriptor do not own this guard.
/// Stable lock files are never unlinked, truncated, or treated as scratch.
#[derive(Debug)]
pub(super) struct NativeLock {
    _file: File,
    #[cfg(unix)]
    owner_process: u32,
}

#[cfg(unix)]
impl Drop for NativeLock {
    fn drop(&mut self) {
        // Only the acquiring process releases this lock. A forked child's
        // inherited destructor must not unlock its still-active parent's guard.
        // Forked leases are not valid for work; acquire a new lease after exec.
        if self.owner_process == std::process::id() {
            // Closing alone waits for every duplicated descriptor. Drop cannot
            // report unlock errors; close remains a conservative fallback, not
            // a guarantee of progress on an OS failure. No retries or unlink.
            let _ = File::unlock(&self._file);
        }
    }
}

impl NativeLock {
    pub(super) fn acquire(
        dir: &Directory,
        name: &str,
        shared: bool,
        wait: Wait<'_>,
    ) -> io::Result<Self> {
        Self::acquire_observed(dir, name, shared, wait, || {})
    }

    pub(super) fn acquire_observed(
        dir: &Directory,
        name: &str,
        shared: bool,
        wait: Wait<'_>,
        mut on_contention: impl FnMut(),
    ) -> io::Result<Self> {
        wait.check()?;
        dir.verify()?;
        let path = dir.path.join(name);
        let file = options(false)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)?;
        check_type(&file, false)?;
        let id = identity(&file)?;
        loop {
            wait.check()?;
            let result = if shared {
                file.try_lock_shared()
            } else {
                file.try_lock()
            };
            match result {
                Ok(()) => {
                    let locked = Self {
                        _file: file,
                        #[cfg(unix)]
                        owner_process: std::process::id(),
                    };
                    wait.check()?;
                    dir.verify()?;
                    let observed = options(false).open(&path)?;
                    check_type(&observed, false)?;
                    if identity(&observed)? != id {
                        return Err(invalid("lock file identity changed during acquisition"));
                    }
                    return Ok(locked);
                }
                Err(TryLockError::WouldBlock) => {
                    on_contention();
                    match wait.remaining()? {
                        None => return Err(io::Error::from(io::ErrorKind::WouldBlock)),
                        Some(remaining) => {
                            thread::park_timeout(remaining.min(Duration::from_millis(5)))
                        }
                    }
                }
                Err(TryLockError::Error(e)) => return Err(e),
            }
        }
    }
}

/// Verify the owned staging name still denotes the retained regular-file handle.
/// Managed ancestor stability is a precondition; this is not hostile-path confinement.
pub(crate) fn verify_file_path(expected: &File, path: &Path) -> io::Result<()> {
    let observed = options(false).open(path)?;
    check_type(&observed, false)?;
    if identity(expected)? != identity(&observed)? {
        return Err(invalid("staged file identity changed"));
    }
    Ok(())
}

#[cfg(all(test, unix))]
mod release_tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn lease_drop_releases_even_with_a_duplicated_os_handle() {
        let root = TempDir::new().unwrap();
        let directory = Directory::open(root.path()).unwrap();
        let lock = NativeLock::acquire(&directory, ".gc.lock", true, Wait::Try).unwrap();
        // Models the duplicated open-file description that fork can retain
        // temporarily before exec. The production API never exposes cloning.
        let inherited = lock._file.try_clone().unwrap();
        assert_eq!(
            NativeLock::acquire(&directory, ".gc.lock", false, Wait::Try)
                .unwrap_err()
                .kind(),
            io::ErrorKind::WouldBlock
        );
        drop(lock);
        let exclusive = NativeLock::acquire(&directory, ".gc.lock", false, Wait::Try);
        assert!(
            exclusive.is_ok(),
            "owner Drop must explicitly release its lock despite a duplicated descriptor"
        );
        drop(exclusive);
        drop(inherited);
    }
}

#[cfg(all(test, unix))]
#[path = "lease_release_tests.rs"]
mod owner_release_tests;
