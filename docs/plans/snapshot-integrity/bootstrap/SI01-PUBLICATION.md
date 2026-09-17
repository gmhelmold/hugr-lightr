# SI-01 additive CAS publication slice

This implementation consumes Accepted ADR-0020 and the existing Store lease.
It does not change current Store, snapshot, CLI or GC routing. Do not run legacy
writers against these new receipt namespaces. The new API requires stable,
caller-managed local Store directories; complete public-path C12 confinement
and filesystem qualification are still integration obligations.

## API and proof boundary

`StoreLease::prepare_reader` captures actual reader bytes in private staging,
then consumes that staging through `LeasedStagedFile::publish`.
`StoreLease::prepare_existing` verifies a requested CAS digest and requalifies
unconfirmed bytes by ordinary streaming copy into a fresh file. Both return
`PreparedObject<'lease>` only after payload and readiness confirmation. A digest
or decoded receipt alone cannot construct this proof, and the proof cannot
outlive the actual shared Store lease. No mutable payload handle is exposed.

Callers observe Created, Requalified, Reused or LeaseCached work. Only the
single leader returning Created increments a new-object count. A cached reader
still validates this invocation's supplied bytes before consulting the map.

## Publication order

Acquire the digest lock under the borrowed Store lease; inspect existing
payload/receipt without following internal links; verify digest, actual length,
assurance and required attributes. A malformed receipt, missing confirmed
payload or corrupt existing payload fails explicitly, without repairing it
silently from caller bytes.

Absent readiness requires a newly written private payload, final identity/hash
verification, final permissions, checked writable-handle file synchronization,
atomic installation and checked payload-directory/ancestor barriers. Only then
is a bounded readiness record written, synchronized, installed and its namespace
confirmed. A stage pathname substituted before installation is rejected by
comparing its native identity with the retained handle.

A confirmed payload is immutable for participating writers. A different lease
streams and checks its bytes and reconstructs the small readiness record into
fresh metadata staging. This restates the receipt's namespace barriers without
replacing the confirmed payload, including after an earlier receipt barrier
failed. A same-lease completed result can be shared without repeating I/O.

Unix payload mode is 0444 before final synchronization; Windows synchronizes
the retained writable private handle without copying source read-only flags.
Windows does not claim directory-entry power-loss durability. This is checked
OS synchronization under the profile assumptions, not hardware certification.

## Failure and concurrency

A pre-install failure produces no confirmed object. A failure at/after attempted
installation reports CommitUncertain and never issues a proof. The same lease
retains that exact shared failure, original OS cause, phase and leader operation
ID; retry requires a new operation/lease. A following lease checks the state and
rewrites unconfirmed payloads instead of accepting existence or retrying fsync
on the old file. Shared failures roundtrip through the existing legacy error
wrapper without discarding the original source.

The per-lease map holds no filesystem operation under its short mutex.
Concurrent waiters receive the completed result; cancellation of one waiter
does not cancel the leader. A leader unwind wakes waiters with a failure rather
than leaving an immortal in-flight entry. Distinct digests can make progress.
Cancellation is checked at publication stages and waits, not as preemption of
arbitrary synchronous read/write/fsync syscalls. Cooperative per-buffer capture
cancellation and CoW remain separate work.

Actual child-process tests terminate an installer after the payload rename,
before its namespace confirmation/receipt. Another process must rewrite and
confirm those bytes. A separate process reuses confirmed payloads without
changing their native identity. These demonstrate process termination behavior,
not simulated power failure. Abandoned staging after abrupt termination is not
reaped by this slice; C13 reaping remains separately guarded work.

## Evidence recipes

Native policy names the portable behavior/parent tests, excluding the child
helper's ordinary no-op invocation. There are 28 Unix behavior/parent methods
plus one child helper; two behavior methods are Unix-only. Counts from platform
runs overlap and are not unique-test sums.

`publication_controls.py` works on an archived disposable checkout. It requires
pristine/restored suites, then compiles five defects independently and requires
the exact intended assertion: omitted file barrier, omitted directory barrier,
existence-only adoption, replacement of a confirmed payload, and skipped staged
native identity. It also requires a valid lifetime snippet to compile and the
otherwise matched early-release snippet to produce exactly E0505.

These controls qualify their explicit slice, not full SI-01 or G-FOUNDATION.
Remaining obligations include full C12 source topology, CoW/fallback behavior,
cooperative copy cancellation, broader resource/metadata-writer integration,
phase-aware recovery/reaping consumers and the complete package DoD. No issue
closure, real-store migration, public activation or merge follows from this doc.
