# SI-01: cooperative reader-to-staging cancellation

This implements the cancellation portion of the accepted C01-C03/SI-01 contract.
It does not establish G-FOUNDATION, enable public Store/CLI routing, change a
storage format, or repair the separate NativeLock lifetime counterexample.

## Behavior

`Digest::of_reader_checked` checks a caller-owned callback before and after each
read, including EOF. Callback errors are terminal and are never treated as
retryable OS `Interrupted` errors. `of_reader` retains its uncancelled behavior
through a no-op callback. Both reject an invalid reader length before slicing.

`LeasedStagedFile::copy_with_wait` checks the existing deadline/cancellation
before creating staging and during source copy, short writes, staged hashing,
and before/after file synchronization. `StoreLease::prepare_reader` and the
copy step used for requalification pass their Wait value to this path. Final
staged verification before payload installation uses the same checkpoints.

A cancellation during capture returns NotPublished, with the original operation
ID and the actual Stage/Verify/PayloadSync phase. Cleanup is restricted to the
existing operation-owned staging allocation. Cancellation before preparation
enters the lease-local single-flight map does not poison a later capture on the
same lease. Ordinary read/write errors retain their original OS cause even if
a cancellation happens concurrently. WriteZero and invalid short-write counts
remain explicit errors. No process-global test hook or dependency is added.

## Limits

A running synchronous OS call is not preempted. A reader/writer that never
returns cannot be interrupted by checkpoints. The preliminary existing-object
inspection in Layout::inspect is not yet checkpointed. CoW and public topology
validation retain their own outstanding obligations. StagedFile's uncancelled
API remains available for existing callers without Wait.

## Reproduction

Use Rust 1.96.0 in a disposable checkout and retain exact commit/tree IDs:

```sh
cargo +1.96.0 test --locked -p lightr-core --lib checked_hash_
cargo +1.96.0 test --locked -p lightr-store --lib capture_cancellation_
cargo +1.96.0 test --locked -p lightr-store --lib checkpoint_
python3 scripts/si01/cancellation_controls.py --out /new/output/path
```

The controls compare pristine/restored suites with five compiling regressions.
Required assertions, not timeouts or compilation errors, determine the result.
Native policy requires all new Store test names. Full Store/workspace gates
remain separate and continue to include the active lock-release counterexample.
