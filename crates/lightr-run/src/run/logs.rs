//! logs — read or follow stdout/stderr log files for a detached run.

use lightr_core::{LightrError, Result};

use super::paths::read_status_file;
use super::types::LogStream;

pub fn logs(dir: &std::path::Path, stream: LogStream, follow: bool) -> Result<()> {
    use std::io::Write;

    fn print_file(path: &std::path::Path, offset: &mut u64) -> Result<bool> {
        let data = std::fs::read(path).map_err(LightrError::Io)?;
        let start = *offset as usize;
        if start < data.len() {
            std::io::stdout()
                .write_all(&data[start..])
                .map_err(LightrError::Io)?;
            *offset = data.len() as u64;
            return Ok(true);
        }
        Ok(false)
    }

    let stdout_path = dir.join("stdout.log");
    let stderr_path = dir.join("stderr.log");

    if !follow {
        match stream {
            LogStream::Stdout => {
                let _ = print_file(&stdout_path, &mut 0u64);
            }
            LogStream::Stderr => {
                let _ = print_file(&stderr_path, &mut 0u64);
            }
            LogStream::Both => {
                let _ = print_file(&stdout_path, &mut 0u64);
                let _ = print_file(&stderr_path, &mut 0u64);
            }
        }
        return Ok(());
    }

    // Follow mode
    let mut stdout_off = 0u64;
    let mut stderr_off = 0u64;
    let mut polls = 0u32;

    loop {
        match stream {
            LogStream::Stdout => {
                if stdout_path.exists() {
                    let _ = print_file(&stdout_path, &mut stdout_off)?;
                }
            }
            LogStream::Stderr => {
                if stderr_path.exists() {
                    let _ = print_file(&stderr_path, &mut stderr_off)?;
                }
            }
            LogStream::Both => {
                if stdout_path.exists() {
                    let _ = print_file(&stdout_path, &mut stdout_off)?;
                }
                if stderr_path.exists() {
                    let _ = print_file(&stderr_path, &mut stderr_off)?;
                }
            }
        }
        // Check if exited and no new bytes
        let status = read_status_file(dir).unwrap_or_default();
        if status.starts_with("exited") {
            // Drain any remaining
            let mut drained = false;
            match stream {
                LogStream::Stdout => {
                    if stdout_path.exists() {
                        drained |= print_file(&stdout_path, &mut stdout_off)?;
                    }
                }
                LogStream::Stderr => {
                    if stderr_path.exists() {
                        drained |= print_file(&stderr_path, &mut stderr_off)?;
                    }
                }
                LogStream::Both => {
                    if stdout_path.exists() {
                        drained |= print_file(&stdout_path, &mut stdout_off)?;
                    }
                    if stderr_path.exists() {
                        drained |= print_file(&stderr_path, &mut stderr_off)?;
                    }
                }
            }
            if !drained {
                break;
            }
        }

        polls += 1;
        if follow_poll_cap_reached(polls) {
            eprintln!("lightr: logs --follow stopped at poll cap ({FOLLOW_MAX_POLLS})");
            break;
        }

        std::thread::sleep(std::time::Duration::from_millis(200));
    }

    Ok(())
}

/// Direct `lightr-run` callers get same no-hang guarantee as CLI follow.
const FOLLOW_MAX_POLLS: u32 = 3000;

fn follow_poll_cap_reached(polls: u32) -> bool {
    polls >= FOLLOW_MAX_POLLS
}

#[cfg(test)]
mod tests {
    use super::{follow_poll_cap_reached, FOLLOW_MAX_POLLS};

    #[test]
    fn follow_poll_cap_is_bounded() {
        assert!(!follow_poll_cap_reached(FOLLOW_MAX_POLLS - 1));
        assert!(follow_poll_cap_reached(FOLLOW_MAX_POLLS));
    }
}
