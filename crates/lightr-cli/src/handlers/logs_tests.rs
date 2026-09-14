//! Tests for the `lightr logs` handler — split out via `#[path]` to keep
//! logs.rs under the 400-line godfile cap (house convention).
//!
//! WP-LIFE-LOGS adds `--tail N`, `-f/--follow`, `--since`, `-t/--timestamps`.
//! The pure selection/streaming helpers (`select_tail`, `bytes_after`,
//! `since_excludes_all`, `parse_since`) carry the load and touch NO
//! process-global state, so they are trivially parallel-safe. The exit-code
//! contract (unknown id ⇒ 2; behavior-preserved no-flag dump ⇒ 0) is exercised
//! end-to-end under the crate-wide `ENV_LOCK` while `LIGHTR_HOME` is set, since
//! `lightr_home()` reads a process-global env var (same pattern as inspect).

use std::fs;

use super::{
    bytes_after, follow_stop_reason, initial_and_follow_paths, initial_log_bytes, parse_since,
    select_tail, stream_paths, timestamp_note, FollowStop,
};
use super::{run as logs_run, LogOpts};
use crate::test_lock::ENV_LOCK;
use lightr_run::LogStream;

// ── select_tail (the --tail N core) ─────────────────────────────────────────

#[test]
fn tail_none_returns_all() {
    let data = b"a\nb\nc\n";
    assert_eq!(select_tail(data, None), data);
}

#[test]
fn tail_last_n_lines() {
    let data = b"l1\nl2\nl3\nl4\nl5\n";
    // Last 2 lines, terminators preserved.
    assert_eq!(select_tail(data, Some(2)), b"l4\nl5\n");
}

#[test]
fn tail_n_larger_than_lines_returns_all() {
    let data = b"only\ntwo\n";
    assert_eq!(select_tail(data, Some(99)), data);
}

#[test]
fn tail_zero_returns_empty() {
    let data = b"a\nb\n";
    assert_eq!(select_tail(data, Some(0)), b"");
}

#[test]
fn tail_no_trailing_newline() {
    let data = b"first\nsecond"; // last line unterminated
    assert_eq!(select_tail(data, Some(1)), b"second");
}

#[test]
fn tail_single_line_no_newline() {
    let data = b"solo";
    assert_eq!(select_tail(data, Some(1)), b"solo");
    assert_eq!(select_tail(data, Some(5)), b"solo");
}

#[test]
fn tail_empty_input() {
    assert_eq!(select_tail(b"", Some(3)), b"");
    assert_eq!(select_tail(b"", None), b"");
}

#[test]
fn stdout_and_stderr_select_only_requested_streams() {
    let tmp = tempfile::tempdir().unwrap();
    assert_eq!(
        stream_paths(tmp.path(), &LogStream::Stdout),
        vec![tmp.path().join("stdout.log")]
    );
    assert_eq!(
        stream_paths(tmp.path(), &LogStream::Stderr),
        vec![tmp.path().join("stderr.log")]
    );
    assert_eq!(
        stream_paths(tmp.path(), &LogStream::Both),
        vec![tmp.path().join("stdout.log"), tmp.path().join("stderr.log")]
    );
}

// ── bytes_after (the --follow append core) ──────────────────────────────────

#[test]
fn follow_streams_appends_then_no_more() {
    // Parallel-safe: unique tempdir per test, no shared/global state.
    let tmp = tempfile::tempdir().unwrap();
    let log = tmp.path().join("stdout.log");

    fs::write(&log, b"first\n").unwrap();
    let (chunk1, off1) = bytes_after(&log, 0).unwrap();
    assert_eq!(chunk1, b"first\n");
    assert_eq!(off1, 6);

    // No new bytes ⇒ empty, offset unchanged (the loop's "no progress" signal).
    let (chunk_none, off_same) = bytes_after(&log, off1).unwrap();
    assert!(chunk_none.is_empty());
    assert_eq!(off_same, off1);

    // Append more ⇒ only the new bytes stream.
    fs::write(&log, b"first\nsecond\n").unwrap();
    let (chunk2, off2) = bytes_after(&log, off1).unwrap();
    assert_eq!(chunk2, b"second\n");
    assert_eq!(off2, 13);
}

#[test]
fn follow_missing_file_is_empty_not_error() {
    let tmp = tempfile::tempdir().unwrap();
    let missing = tmp.path().join("stderr.log");
    let (chunk, off) = bytes_after(&missing, 0).unwrap();
    assert!(chunk.is_empty());
    assert_eq!(off, 0);
}

#[test]
fn follow_offset_past_eof_clamps() {
    let tmp = tempfile::tempdir().unwrap();
    let log = tmp.path().join("stdout.log");
    fs::write(&log, b"abc").unwrap();
    // Offset beyond EOF (e.g. file truncated) ⇒ no panic, empty slice.
    let (chunk, off) = bytes_after(&log, 999).unwrap();
    assert!(chunk.is_empty());
    assert_eq!(off, 3);
}

#[test]
fn follow_setup_emits_append_after_initial_tail_exactly_once() {
    let tmp = tempfile::tempdir().unwrap();
    let log = tmp.path().join("stdout.log");
    fs::write(&log, b"first\nsecond\n").unwrap();

    // This read and returned offset are same snapshot. Simulate append while
    // initial tail waits to print, then verify follow emits only new bytes.
    let (initial, offset) = initial_log_bytes(&log, Some(1)).unwrap();
    fs::write(&log, b"first\nsecond\nthird\n").unwrap();
    let (append, next) = bytes_after(&log, offset).unwrap();
    assert_eq!(initial, b"second\n");
    assert_eq!(append, b"third\n");
    assert_eq!(next, 19);
}

#[test]
fn bounded_follow_stops_after_drain_or_poll_cap() {
    assert_eq!(
        follow_stop_reason(true, false, 0),
        Some(FollowStop::Drained)
    );
    assert_eq!(follow_stop_reason(true, true, 0), None);
    assert_eq!(
        follow_stop_reason(false, false, super::FOLLOW_MAX_POLLS),
        Some(FollowStop::PollCap)
    );
}

// ── --since honest semantics ────────────────────────────────────────────────

#[test]
fn parse_since_unix_seconds() {
    assert_eq!(parse_since("1717600000"), Some(1_717_600_000));
    assert_eq!(parse_since("  42 "), Some(42));
    assert_eq!(parse_since("not-a-ts"), None);
    assert_eq!(parse_since("2026-06-19T00:00:00Z"), None); // lenient include
}

#[test]
fn timestamp_disclosure_is_explicitly_mtime_only() {
    let streams = vec![
        ("stdout.log".to_string(), Some(1_717_600_000)),
        ("stderr.log".to_string(), Some(1_717_600_001)),
    ];
    let note = timestamp_note(false, &streams);
    assert!(note.contains("no per-line timestamps"));
    assert!(note.contains("stdout.log=1717600000"));
    assert!(note.contains("stderr.log=1717600001"));
    assert!(note.contains("-t reports"));

    let since = timestamp_note(true, &streams);
    assert!(since.contains("no per-line timestamps"));
    assert!(since.contains("--since compares"));
    assert!(since.contains("stdout.log=1717600000"));
    assert!(since.contains("stderr.log=1717600001"));
}

#[test]
fn since_skips_old_backlog_but_follow_keeps_both_streams() {
    let tmp = tempfile::tempdir().unwrap();
    let stdout = tmp.path().join("stdout.log");
    let stderr = tmp.path().join("stderr.log");
    let (initial, follow) =
        initial_and_follow_paths(vec![stdout.clone(), stderr.clone()], Some("20"), |path| {
            if path == stdout {
                Some(10)
            } else if path == stderr {
                Some(30)
            } else {
                None
            }
        });
    assert_eq!(initial, vec![stderr.clone()]);
    assert_eq!(follow, vec![stdout.clone(), stderr.clone()]);

    // No or malformed cutoff preserves all initial and follow streams.
    assert_eq!(
        initial_and_follow_paths(vec![stdout.clone(), stderr.clone()], None, |_| Some(0)),
        (
            vec![stdout.clone(), stderr.clone()],
            vec![stdout.clone(), stderr.clone()]
        )
    );
    assert_eq!(
        initial_and_follow_paths(
            vec![stdout.clone(), stderr.clone()],
            Some("yesterday"),
            |_| Some(0)
        ),
        (vec![stdout.clone(), stderr.clone()], vec![stdout, stderr])
    );
}

// ── exit-code contract (end-to-end, under ENV_LOCK) ─────────────────────────

fn base_opts() -> LogOpts<'static> {
    LogOpts {
        stderr: false,
        both: false,
        follow: false,
        tail: None,
        since: None,
        timestamps: false,
    }
}

#[test]
fn unknown_run_id_exits_1() {
    // Docker parity (WP-EXIT-CODE): `logs <missing>` → "No such container",
    // exit 1 (a missing container is NOT a usage error).
    let tmp = tempfile::tempdir().unwrap();
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    // SAFETY: single-threaded under ENV_LOCK.
    unsafe { std::env::set_var("LIGHTR_HOME", tmp.path()) };
    let code = logs_run("does-not-exist", &base_opts());
    unsafe { std::env::remove_var("LIGHTR_HOME") };
    assert_eq!(code, 1);
}

#[test]
fn no_flags_dumps_full_log_exit_0() {
    let tmp = tempfile::tempdir().unwrap();
    let run_dir = tmp.path().join("run").join("r1");
    fs::create_dir_all(&run_dir).unwrap();
    fs::write(run_dir.join("stdout.log"), b"hello world\n").unwrap();
    // Mark exited so the (non-follow) base path returns immediately.
    fs::write(run_dir.join("status"), b"exited 0\n").unwrap();

    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    unsafe { std::env::set_var("LIGHTR_HOME", tmp.path()) };
    let code = logs_run("r1", &base_opts()); // behavior-preserved path
    unsafe { std::env::remove_var("LIGHTR_HOME") };
    assert_eq!(code, 0);
}

#[test]
fn tail_path_exit_0() {
    let tmp = tempfile::tempdir().unwrap();
    let run_dir = tmp.path().join("run").join("r2");
    fs::create_dir_all(&run_dir).unwrap();
    fs::write(run_dir.join("stdout.log"), b"a\nb\nc\n").unwrap();

    let mut opts = base_opts();
    opts.tail = Some(2);

    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    unsafe { std::env::set_var("LIGHTR_HOME", tmp.path()) };
    let code = logs_run("r2", &opts);
    unsafe { std::env::remove_var("LIGHTR_HOME") };
    assert_eq!(code, 0);
}
