//! Darwin exec PTY setup. Called only in the forked child, before exec.
//!
//! setsid alone does not establish a controlling terminal. TIOCSCTTY alone
//! does not retain the vnode reference used by Darwin's exit-time drain.
//! Opening /dev/tty establishes that reference; closing this temporary fd
//! leaves the kernel's session reference until exit. No parent-held slave,
//! relay thread, output buffer or altered StreamSession shape is needed.
use std::io;

/// Establish the child's controlling terminal and its exit-time drain reference.
///
/// # Safety
/// Call only from Command::pre_exec after stdin has been wired to this child's
/// freshly allocated PTY slave. Only native calls and OS-error capture occur;
/// no allocation, formatting, locks or process-global parent changes are used.
pub(super) unsafe fn configure_child() -> io::Result<()> {
    if unsafe { libc::setsid() } < 0 {
        return Err(io::Error::last_os_error());
    }
    if unsafe { libc::ioctl(libc::STDIN_FILENO, libc::TIOCSCTTY, 0) } < 0 {
        return Err(io::Error::last_os_error());
    }
    let terminal = unsafe {
        libc::open(
            c"/dev/tty".as_ptr(),
            libc::O_RDWR | libc::O_NOCTTY | libc::O_CLOEXEC,
        )
    };
    if terminal < 0 {
        return Err(io::Error::last_os_error());
    }
    if unsafe { libc::close(terminal) } < 0 {
        // Do not retry close: its descriptor state is not portable on error.
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(test)]
#[path = "stream_tty_tests.rs"]
mod tests;
