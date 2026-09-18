# CI-160: control timeout ordering and explicit fixture readiness

Status: correction candidate for #160 / PR #159, NOT full qualification.
Parent source: aa61dd456a341bab62eac98ceb0467e526f31158.
No merge, release or snapshot protocol activation is authorized by this receipt.

## Two distinct defects

The ACK helper configured its read timeout after sending metadata. A fast
receiver can write its ACK and close before that option is set. On the tested
Intel Mac, Darwin rejects that option with EINVAL even though the reply is still
buffered and readable. Configure the timeout before send_fd instead. Keep the
existing valid-ACK check, endpoint ownership and error propagation; add no retry.

A separate fixture weakness waited for the socket pathname rather than completed
listener setup. The new in-process fixture uses a private readiness callback and
an owned channel. The public host entry calls the same body with a no-op callback.
The signal follows successful bind/listen and accept-thread creation. Attach,
frame, DHCP, DNS and teardown assertions and four-by-eight parallelism remain.
No waiting on pathname presence and no retry of a failed attach are added.

This explicit signal implementation is distinct from the previously preserved,
unpublished listener-probe patch. That old patch is not applied by this commit.

## Cause-specific evidence

Environment: owner's Intel x86_64 Mac, macOS 15.3.2; pinned Rust 1.96.0.
RUSTFLAGS=-D warnings -C link-arg=-Wl,-rpath,/usr/lib/swift.
CARGO_BUILD_JOBS=2; isolated worktree and target directory.

A standalone Rust control using std UnixStream, without descriptor transfer,
proved option-before-reply success and option-after-close EINVAL/22 while both
cases read the queued ACK. Source SHA-256:
40be6db58822962cd3900cfa8950336a1252e50e8123b862ca15988d8a5c1388.
Output SHA-256:
3acbc7746c2ea82996ed4382d2cba94305bdaa2a2157ac94d9bf6adad6235c37.
Apple's published sosetoptlock has an explicit closed-socket EINVAL path:
https://github.com/apple-oss-distributions/xnu/blob/main/bsd/kern/uipc_socket.c
The native control, not that moving upstream branch alone, proves local behavior.

Three coordinated helper regressions cover a valid ACK, invalid ACK and absent
ACK after peer close. The old send-before-timeout ordering compiles and fails all
three on this Mac. Configuring the option before send makes all three pass.
The scheduling observer coordinates peer completion immediately after send;
it is empty in production. The control-only peer does not prove FD installation;
existing complete lifecycle/ownership tests provide that separate coverage.

The readiness regression passes on the candidate. Moving its notification before
bind compiles and fails the exact accepting-listener assertion. The original
source is restored after that negative control. No compile failure counts as a
causal test result.

## Results and remaining qualification

- ACK cases: three expected failures before, three passes after.
- Readiness: one pass, one compiled expected failure on the early-signal mutant.
- Complete vswitch selection: 48 passed, zero failed/ignored.
- Full lightr-run library with vz and four test threads: 269 passed, TWO FAILED,
  zero ignored. This is retained as a rejection, not hidden by selected success.
- Candidate feature-enabled all-targets Clippy and fmt/diff checks: passed.

The full-suite failures are wait_then_exit_observed_via_background_writer and
stop_reaps_the_whole_process_group. Their causes have not been established by
this correction. Do not infer that these are unrelated, harmless or new merely
from their names. A follow-up tool request to inspect their details was rejected
before execution; no diagnosis is claimed from it.

The original aa61dd4 attach failure had no syscall trace. The deterministic ACK
ordering test demonstrates a concrete failing schedule in that helper, not a
retrospectively recovered stack. A further error-only diagnostic batch ended on
its tenth run at the separately known startup connect timeout/ENOENT, not EINVAL.
Historical and diagnostic failures remain evidence, not superseded success.

Keep #159 draft and #160 open until the exact complete candidate and integration
requirements pass. Parent #146 needs fresh post-integration verification.
All existing WP axioms/criteria remain unchanged. Main and storage activation
remain untouched.
