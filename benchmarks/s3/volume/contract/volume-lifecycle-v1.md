# S3 Named-Volume Lifecycle v1

Source: `benchmarks/TECHLEAD-SPRINT-03.md` lines 205-241. This contract is
normative for S3-5B. It freezes durable mount ownership; it does not claim a
runtime implementation.

## Scope And Capability

Each named volume owns `.lightr/owners.json` and `.lightr/lock`. `owners.json`
has version `1` and one `owners` array. Unix supports this contract only when
`flock(.lightr/lock)` covers acquire, release, recovery, `rm`, and `prune`.
Windows and every platform without equally atomic locking plus stable process
start tokens are engine-gated: no acquire, recovery, `rm`, or `prune` claim is
permitted.

`owners.json` envelope:

```json
{"version":1,"owners":[OWNER]}
```

Every owner has `nonce`, exactly 64 hexadecimal characters, and one
of these exclusive shapes:

```json
{"phase":"pending","nonce":"HEX64","coordinator_pid":123,"coordinator_start_token":"linux:123:456"}
{"phase":"active","nonce":"HEX64","run_id":"RUN","pid":124,"process_start_token":"linux:124:457","mount_id":"MOUNT"}
```

Pending owner has exactly `phase`, `nonce`, `coordinator_pid`, and
`coordinator_start_token`. Active owner has exactly `phase`, `nonce`, `run_id`,
`pid`, `process_start_token`, and `mount_id`. Extra, missing, or cross-phase
fields are malformed. Multiple active entries are valid shared mounts; each has
distinct mount identity. Effective mount refcount equals active-owner count;
pending owners are never counted and no independent mutable refcount exists.
PID alone never proves ownership.

On Linux, process token is `linux:<pid>:<starttime>`. `starttime` is field 22
of `/proc/<pid>/stat`, parsed only after final `)` delimiter. A missing,
unreadable, malformed, PID-mismatched, or token-mismatched process observation
does not prove a live process; it is usable only as stated under recovery.

## Durable Writes And Linearization

While holding lock, writer writes next complete `owners.json.tmp`, calls
`sync_all`, atomically renames tmp to `owners.json`, then fsyncs parent
directory. Rename linearizes state. No caller may report success before parent
fsync completes. Lock loss, write failure, `sync_all` failure, rename failure,
or parent fsync failure refuses operation; no delete follows failure.

Each run also owns `$LIGHTR_HOME/run/<run_id>/volume-owner.json`. It records
same pending or active nonce and is fsynced at every transition. Teardown writes
and fsyncs terminal status before release. Registry is matching only when it is
readable, well-formed, names same run ID and nonce, and has terminal status.
This contract leaves terminal-status payload owned by run lifecycle; recovery
requires its durable terminal marker, not an inferred exit result.

## Acquire And Release

1. Lock volume. Write durable pending owner with fresh nonce, coordinator PID,
   and coordinator start token. Do not write child PID, child token, run ID, or
   mount ID in pending state.
2. Spawn child behind pipe barrier before exec. Barrier remains closed.
3. Read child PID and Linux process token. Replace same-nonce pending entry
   atomically with active entry containing run ID, child PID/token, and mount
   ID. Fsync matching run registry transition.
4. Release child only after active owner and run registry are durable. Barrier
   failure refuses operation and child does not proceed to exec.
5. Spawn failure removes only exact pending nonce while locked.
6. After terminal registry status is durable, release removes only active entry
   matching `(run_id, nonce, process_start_token)`. No broad PID, run ID, or
   nonce deletion is allowed.

## Recovery, Remove, And Prune

Recovery removes active owner only with positive death proof:

* readable matching registry plus terminal status; or
* readable matching registry plus Unix `/proc/<pid>/stat` absent or token
  mismatch.

Recovery removes pending owner only when coordinator death is proven by same
token check and no active owner carries pending nonce. Missing, malformed,
unreadable, or nonce-mismatched registry is ambiguity. Ambiguity retains owner
and refuses `rm` and `prune`.

`rm` and `prune` lock, recover, snapshot owners, then delete only empty
snapshot. Non-empty snapshot refuses deletion. Lock loss, fsync/rename failure,
or barrier failure refuses operation without deletion.

## Fixture Oracle

`fixtures.json` defines operation traces. `goldens.json` maps every fixture ID
to exact final owner and delete/refusal result. S3-5B tests must execute each
trace in separate processes where stated by Sprint 03: normal teardown, shared
mounts, failed spawn, SIGKILL owner, stale/PID-reused ledger, concurrent
`rm`/`prune`. Tests must mutation-probe acquire, release, recovery, and refusal
edges. Fixtures are protocol goldens, not current-runtime tests.
