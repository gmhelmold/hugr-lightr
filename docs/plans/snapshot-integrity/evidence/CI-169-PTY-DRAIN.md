# PR #169: Darwin PTY output-loss correction

Date: 2026-09-19. Repair within #169, part of #147 / #152. Qualification pending.
Base source: 2cfd7bf84c90161cff7e64b9705f3afa2b785c84.

## Observed failure and cause

Full CI 35415473360, current ARM job 105823167301, failed the original
open_exec_tty_uses_pty_master_no_stderr test with UnexpectedEof. Its crate
reported 23 passes and one failure. The native Windows link witness and the
other campaign gates passed; they cannot override this full-CI failure.

The old child hook called setsid but never acquired a controlling terminal.
On Darwin, closing the last slave reference can discard unread output.
TIOCSCTTY alone is not enough to retain the vnode through stdio closure:
opening /dev/tty acquires the session reference used by the exit drain.

A bounded C experiment on the owner's Intel Mac used fresh PTYs and owned
children. The child wrote one line, closed stdin/stdout/stderr, then notified
its parent through an independent pipe before the parent read the master.
setsid-only and TIOCSCTTY-only returned zero bytes; TIOCSCTTY followed by
opening/closing /dev/tty retained all 18 terminal bytes. Every child returned
exit code 7. The initial C compilation failed before execution; after adding
the required signal header it compiled with -Wall -Wextra -Werror. Earlier
Python observations did not distinguish the orderings and are not causal proof.
This controlled native sequence is not a retrospectively recovered remote stack.

## Correction and boundary

Only macOS open_exec_tty invokes the new child setup: checked setsid,
TIOCSCTTY on the already wired slave, then open and close /dev/tty. Failures
propagate through Command's existing spawn-error path. The temporary descriptor
is closed; the kernel owns the controlling-terminal reference until exit.
No parent-held slave, relay, daemon, output spool, sleep, retry or StreamSession
shape change is introduced. Linux namespace-child terminal setup and non-Unix
behavior are unchanged. The original echo test remains byte-identical.

Terminal output must be drained concurrently with waiting for process exit, or
the client must close its master handles. Darwin can wait for unread terminal
output during exit; immediate exit-status availability before reading is NOT
promised. No timeout is used to discard valid unread output. New tests cover a
delayed reader, client disconnect, output-free exit, rejected non-terminal
setup and the actual helper's raw master/merged output/nonzero exit status.

## Five acceptance groups

Success criteria: preserve the written line after all child stdio descriptors
close; preserve actual child status and merged output; retain a raw master;
reject failed native setup rather than launching an incorrectly wired child.

Completeness criteria: retain the original echo fixture; test delayed reading,
EOF, disconnect, empty output, setup failure and the real helper; execute on
Intel and ARM macOS. A helper child entry point is not an additional witness.

Quality standards: pinned Rust 1.96.0, rustfmt and denied-warning Clippy;
line-level review of the small pre-exec syscall sequence; bounded test waits
and output; RAII cleanup of only owned children and scratch; preserve Linux
namespace behavior and all existing gates.

Invariants: no guessed output, swallowed errors, race-hiding sleeps or skips;
no public vocabulary, dependency, protocol, authorization or main-branch change.
Read/write/exit ordering and native-platform scope remain explicit.

Definition of Done: native regression suites pass on Intel and ARM; a compiled
control that opens /dev/null instead of /dev/tty fails the exact output-loss
assertion, then restored code passes. The original echo test passes before and
after the control. All full and campaign Actions pass at the exact candidate,
review is labeled coordinator self-review, merge uses expected head, and fresh
parent #146 CI passes before delivery is closed. SI-01 remains open.

## Primary implementation references

Apple XNU bsd/kern/tty.c TIOCSCTTY and ttywait;
bsd/kern/tty_tty.c cttyopen; bsd/kern/kern_exit.c exit session teardown.
https://github.com/apple-oss-distributions/xnu/tree/main/bsd/kern

## Executable evidence

scripts/ci/pty_controls.py archives its exact source and records each command,
exit code and raw log hash. Native PTY drain controls runs on macos-15-intel
and macos-15 in addition to the existing complete CI. The seven-method selection
contains six behavior regressions plus one re-executed child helper. A timeout
or compiler failure is never accepted as the missing-reference control.
Actions results, candidate SHA/tree and reviewed merge identity belong in the
live PR review; writing this receipt alone does not qualify the repair.

## Qualification findings retained

The first repair candidate (7546c62) failed compilation before running tests:
the pinned libc defines TIOCSCTTY as u32 but ioctl takes c_ulong. Commit f78efd0
adds the explicit lossless request conversion; no error result is converted.

On f78efd0 the ARM native suite executed six methods: five passed, while the
non-terminal fixture expected ENOTTY but Darwin returned ENODEV (19). The
workload did not launch. /dev/tty unavailability may report ENODEV after the
ioctl stage; the test now accepts only that terminal-specific error or ENOTTY,
not arbitrary failure. An additional closed-slave test requires exact EBADF
and rejection before exec. The implementation is unchanged by this fixture
correction. Prior failing artifacts remain historical evidence.
