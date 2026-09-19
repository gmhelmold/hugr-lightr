//! Darwin PTY allocation with close-on-exec set by each creating syscall.
//!
//! openpty followed by F_SETFD leaves a fork/exec inheritance window. A leaked
//! master can prevent disconnect from reaching the exiting session leader.
use std::ffi::CStr;
use std::fs::File;
use std::io;
use std::os::fd::{AsRawFd, FromRawFd};

pub(super) fn open() -> io::Result<(File, File)> {
    let master = open_device(c"/dev/ptmx")?;
    // SAFETY: grant/unlock act on our live, exclusively owned PTY master.
    if unsafe { libc::grantpt(master.as_raw_fd()) } < 0
        || unsafe { libc::unlockpt(master.as_raw_fd()) } < 0
    {
        return Err(io::Error::last_os_error());
    }
    // Darwin TIOCPTYGNAME writes its fixed 128-byte name into caller storage;
    // unlike ptsname, no process-global scratch buffer is shared across threads.
    let mut name = [0u8; 128];
    // SAFETY: correct Darwin request and a writable buffer of its required size.
    if unsafe {
        libc::ioctl(
            master.as_raw_fd(),
            libc::TIOCPTYGNAME as libc::c_ulong,
            name.as_mut_ptr(),
        )
    } < 0
    {
        return Err(io::Error::last_os_error());
    }
    let name = CStr::from_bytes_until_nul(&name)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "unterminated PTY name"))?;
    if name.to_bytes().is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "empty PTY name"));
    }
    let slave = open_device(name)?;
    Ok((master, slave))
}

fn open_device(path: &CStr) -> io::Result<File> {
    // SAFETY: path is terminated. Both flags are set atomically by open, before
    // any concurrent spawn can see an inheritable descriptor. O_NOCTTY keeps
    // the parent from acquiring a controlling terminal; child setup is separate.
    let fd = unsafe {
        libc::open(
            path.as_ptr(),
            libc::O_RDWR | libc::O_NOCTTY | libc::O_CLOEXEC,
        )
    };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: this successful open transferred exactly one descriptor to us.
    Ok(unsafe { File::from_raw_fd(fd) })
}
