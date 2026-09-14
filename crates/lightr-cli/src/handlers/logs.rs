//! `lightr logs` handler — read/stream a run's stdout/stderr log files.
//!
//! Docker-parity flags (WP-LIFE-LOGS): `--tail N`, `-f/--follow`,
//! `--since <ts>`, `-t/--timestamps`. The base contract (no flags ⇒ full dump)
//! is byte-for-byte preserved.
//!
//! Honesty note: lightr's detached runs write RAW stdout/stderr to
//! `stdout.log`/`stderr.log` — the on-disk format carries NO per-line
//! timestamps. So `-t/--timestamps` and `--since` cannot synthesize a per-line
//! clock from nothing; rather than fabricate one (tense-law), they fall back to
//! the file's last-modified time as a single honest signal and say so on stderr.

use std::io::Write;
use std::path::Path;

use lightr_run::{logs, run_status, LogStream, RunStatus};

use crate::{exit::die_lightr, lightr_home};

/// Bundle of the WP-LIFE-LOGS flags, threaded from dispatch.
pub struct LogOpts<'a> {
    pub stderr: bool,
    pub both: bool,
    pub follow: bool,
    /// `--tail N` — print only the last N lines. `None` ⇒ all (our base
    /// contract; a literal `--tail all` is parsed to `None` upstream so the
    /// no-flag full-dump behavior is preserved).
    pub tail: Option<usize>,
    /// `--since <ts>` — raw timestamp string (unix seconds; RFC3339 lenient).
    pub since: Option<&'a str>,
    /// `-t/--timestamps` — surface the (file-level) timestamp signal.
    pub timestamps: bool,
}

pub fn run(id: &str, opts: &LogOpts) -> i32 {
    let home = lightr_home();
    // WP-RUNFLAGS: resolve a `--name` (or id-prefix) to the run id, like `rm`, so
    // `logs <name>` works. Unresolvable ⇒ "No such container" + exit 1 (Docker
    // parity, WP-EXIT-CODE). A bare existing id still resolves to itself.
    let resolved = match lightr_run::resolve(&home, id) {
        Ok(rid) => rid,
        Err(_) => {
            eprintln!("Error: No such container: {id}");
            return 1;
        }
    };
    let run_dir = home.join("run").join(&resolved);

    // Docker `logs <missing>` → "No such container" + exit 1 (WP-EXIT-CODE).
    if !run_dir.exists() {
        eprintln!("Error: No such container: {id}");
        return 1;
    }

    let stream = if opts.both {
        LogStream::Both
    } else if opts.stderr {
        LogStream::Stderr
    } else {
        LogStream::Stdout
    };

    // Honest disclosure: the on-disk log format has no per-line timestamps, so
    // `--since`/`-t` cannot operate per-line. We surface each selected stream's
    // file mtime and say so, rather than fabricate a clock (tense-law).
    let enrich = opts.timestamps || opts.since.is_some();

    // Fast path: no tail, no enrichment, no follow ⇒ delegate to the frozen
    // lightr-run reader so the base dump is byte-for-byte preserved.
    if opts.tail.is_none() && !enrich && !opts.follow {
        return match logs(&run_dir, stream, false) {
            Ok(()) => 0,
            Err(e) => die_lightr(&e),
        };
    }

    let stream_paths = stream_paths(&run_dir, &stream);
    if enrich {
        emit_timestamp_note(&stream_paths, opts.since);
    }

    let (initial_paths, follow_paths) =
        initial_and_follow_paths(stream_paths, opts.since, file_mtime_seconds);

    // Snapshot every followed stream. `--since` skips only initial backlog;
    // old streams retain their offset and receive later appends under follow.
    let mut offsets = Vec::with_capacity(follow_paths.len());
    for p in &follow_paths {
        let (data, offset) = match initial_log_bytes(p, opts.tail) {
            Ok(value) => value,
            Err(e) => return die_lightr(&e),
        };
        if initial_paths.contains(p) {
            if let Err(e) = write_log_bytes(&data) {
                return die_lightr(&e);
            }
        }
        offsets.push(offset);
    }

    if !opts.follow {
        return 0;
    }

    // Bounded follow: poll for appends, stop when the run has exited and the
    // streams are drained, OR when a hard cap is hit (never hang forever —
    // no-daemon discipline: nothing of ours should spin unbounded).
    follow_bounded(&resolved, &home, &follow_paths, offsets)
}

/// Resolve the concrete log file path(s) for the selected stream.
fn stream_paths(run_dir: &Path, stream: &LogStream) -> Vec<std::path::PathBuf> {
    let out = run_dir.join("stdout.log");
    let err = run_dir.join("stderr.log");
    match stream {
        LogStream::Stdout => vec![out],
        LogStream::Stderr => vec![err],
        LogStream::Both => vec![out, err],
    }
}

/// Read initial bytes and offset from one snapshot. Missing stream ⇒ empty.
fn initial_log_bytes(path: &Path, tail: Option<usize>) -> lightr_core::Result<(Vec<u8>, u64)> {
    if !path.exists() {
        return Ok((Vec::new(), 0));
    }
    let data = std::fs::read(path).map_err(lightr_core::LightrError::Io)?;
    let offset = data.len() as u64;
    Ok((select_tail(&data, tail).to_vec(), offset))
}

fn write_log_bytes(data: &[u8]) -> lightr_core::Result<()> {
    let mut out = std::io::stdout();
    out.write_all(data).map_err(lightr_core::LightrError::Io)?;
    out.flush().map_err(lightr_core::LightrError::Io)?;
    Ok(())
}

/// Pure tail selection: the last `tail` lines of `data` (or all when `None`),
/// returned as a byte slice into `data`. Line terminators are preserved; a
/// single trailing empty segment after the final '\n' is not over-counted.
fn select_tail(data: &[u8], tail: Option<usize>) -> &[u8] {
    let Some(n) = tail else { return data };
    if n == 0 {
        return &data[data.len()..];
    }
    // Walk backwards counting '\n' boundaries. We want the byte offset just
    // after the (n)-th-from-last line start. A trailing '\n' terminates the
    // last line and is not itself a separator that begins a new (empty) line.
    let end = data.len();
    let mut newlines = 0usize;
    let mut i = end;
    // Skip a single trailing newline so it doesn't count as an extra line.
    let scan_end = if i > 0 && data[i - 1] == b'\n' {
        i - 1
    } else {
        i
    };
    i = scan_end;
    while i > 0 {
        if data[i - 1] == b'\n' {
            newlines += 1;
            if newlines == n {
                return &data[i..];
            }
        }
        i -= 1;
    }
    data
}

/// Hard cap on follow polling so the command never hangs forever even if a run
/// never writes a final `exited` status (supervisor vanished). 200ms/poll.
const FOLLOW_MAX_POLLS: u32 = 3000; // ~10 minutes ceiling
const FOLLOW_POLL_MS: u64 = 200;

#[derive(Debug, PartialEq, Eq)]
enum FollowStop {
    Drained,
    PollCap,
}

fn follow_stop_reason(terminal: bool, had_new: bool, polls: u32) -> Option<FollowStop> {
    if terminal && !had_new {
        Some(FollowStop::Drained)
    } else if polls >= FOLLOW_MAX_POLLS {
        Some(FollowStop::PollCap)
    } else {
        None
    }
}

/// Stream appends to `paths`, stopping when the run has exited and the streams
/// are drained, or when the poll cap is reached. Bounded — no infinite spin.
fn follow_bounded(
    id: &str,
    home: &Path,
    paths: &[std::path::PathBuf],
    mut offsets: Vec<u64>,
) -> i32 {
    let mut polls = 0u32;
    loop {
        let mut had_new = false;
        for (p, off) in paths.iter().zip(offsets.iter_mut()) {
            match append_from(p, off) {
                Ok(new) => had_new |= new,
                Err(e) => return die_lightr(&e),
            }
        }

        // Exit gate: the run has stopped (or is unresolvable) AND nothing new
        // this round ⇒ done. Treating Unknown/Err as terminal keeps a vanished
        // supervisor from pinning the loop to the poll cap.
        let terminal = matches!(
            run_status(home, id),
            Ok(RunStatus::Exited(_)) | Ok(RunStatus::Unknown) | Err(_)
        );
        polls += 1;
        match follow_stop_reason(terminal, had_new, polls) {
            Some(FollowStop::Drained) => return 0,
            Some(FollowStop::PollCap) => {
                // Bounded stop — honest, not a silent hang.
                eprintln!("lightr: logs --follow stopped at poll cap ({FOLLOW_MAX_POLLS})");
                return 0;
            }
            None => {}
        }
        std::thread::sleep(std::time::Duration::from_millis(FOLLOW_POLL_MS));
    }
}

/// Write any bytes in `path` past `*offset` to stdout; advance `*offset`.
fn append_from(path: &Path, offset: &mut u64) -> lightr_core::Result<bool> {
    let (bytes, new_off) = bytes_after(path, *offset)?;
    if bytes.is_empty() {
        return Ok(false);
    }
    let mut out = std::io::stdout();
    out.write_all(&bytes)
        .map_err(lightr_core::LightrError::Io)?;
    out.flush().map_err(lightr_core::LightrError::Io)?;
    *offset = new_off;
    Ok(true)
}

/// Pure read-after-offset: bytes of `path` past `offset` and the new offset
/// (the file's full length). Missing file ⇒ empty + same offset. Lets follow be
/// tested deterministically without capturing stdout.
fn bytes_after(path: &Path, offset: u64) -> lightr_core::Result<(Vec<u8>, u64)> {
    if !path.exists() {
        return Ok((Vec::new(), offset));
    }
    let data = std::fs::read(path).map_err(lightr_core::LightrError::Io)?;
    let start = (offset as usize).min(data.len());
    let new_off = data.len() as u64;
    Ok((data[start..].to_vec(), new_off))
}

/// File-level timestamp signals, one per selected stream. Printed to stderr so
/// they never corrupt log bytes on stdout.
fn emit_timestamp_note(paths: &[std::path::PathBuf], since: Option<&str>) {
    let streams = paths
        .iter()
        .map(|path| {
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("unknown")
                .to_string();
            (name, file_mtime_seconds(path))
        })
        .collect::<Vec<_>>();
    eprintln!("{}", timestamp_note(since.is_some(), &streams));
}

fn timestamp_note(since: bool, streams: &[(String, Option<u64>)]) -> String {
    let mtimes = streams
        .iter()
        .map(|(path, mtime)| {
            format!(
                "{path}={}",
                mtime.map_or_else(|| "unknown".to_string(), |time| time.to_string())
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    if since {
        format!(
            "lightr: logs has no per-line timestamps; --since compares against \
             each selected stream's last-modified time: {mtimes}"
        )
    } else {
        format!(
            "lightr: logs has no per-line timestamps; -t reports the log file's \
             last-modified time: {mtimes}"
        )
    }
}

/// `--since` filters each stream independently. Raw logs have no line clock, so
/// a stream is included whole only when its own mtime is at or after cutoff.
fn filter_paths_since<F>(
    paths: Vec<std::path::PathBuf>,
    since: Option<&str>,
    mut mtime: F,
) -> Vec<std::path::PathBuf>
where
    F: FnMut(&Path) -> Option<u64>,
{
    let Some(cutoff) = since.and_then(parse_since) else {
        return paths;
    };
    paths
        .into_iter()
        .filter(|path| mtime(path).is_none_or(|time| time >= cutoff))
        .collect()
}

/// `--since` filters existing backlog only. Follow must keep all selected
/// streams so bytes appended after setup are never silently dropped.
fn initial_and_follow_paths<F>(
    paths: Vec<std::path::PathBuf>,
    since: Option<&str>,
    mtime: F,
) -> (Vec<std::path::PathBuf>, Vec<std::path::PathBuf>)
where
    F: FnMut(&Path) -> Option<u64>,
{
    let initial = filter_paths_since(paths.clone(), since, mtime);
    (initial, paths)
}

fn file_mtime_seconds(path: &Path) -> Option<u64> {
    std::fs::metadata(path)
        .ok()?
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_secs())
}

/// Parse a `--since` value: unix seconds. We avoid a chrono dep — only the
/// unix-seconds form is parsed precisely; any other string yields `None`
/// (lenient include, honest about the limitation in the stderr note above).
fn parse_since(s: &str) -> Option<u64> {
    s.trim().parse::<u64>().ok()
}

#[cfg(test)]
#[path = "logs_tests.rs"]
mod tests;
