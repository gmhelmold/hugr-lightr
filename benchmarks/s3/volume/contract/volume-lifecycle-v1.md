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
same pending or active nonce and is fsynced at every transition. For a release,
matching `registry.active` must precede `registry.terminal`, then
`registry.sync`, then same-run `owner.remove`. Missing terminal or sync refuses
release. Registry is matching only when it is readable, well-formed, names same
run ID and nonce, and has terminal status. This contract leaves terminal-status
payload owned by run lifecycle; recovery requires its durable terminal marker,
not an inferred exit result.

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
6. After matching active registry, terminal registry status, and terminal sync
   are durable in that order, release removes only active entry matching
   `(run_id, nonce, process_start_token)`. No broad PID, run ID, or nonce
   deletion is allowed.

## Recovery, Remove, And Prune

Recovery removes active owner only with positive death proof:

* readable matching registry plus terminal status; or
* readable matching registry plus Unix `/proc/<pid>/stat` absent or token
  mismatch.

Recovery removes pending owner only when coordinator death is proven by same
token check and no active owner carries pending nonce. Missing, malformed,
unreadable, or nonce-mismatched registry is ambiguity. Ambiguity retains owner
and refuses `rm` and `prune`.

`rm` and `prune` events occur inside their matching actor's
`lock.acquire`/`lock.release` interval. Trace lock intervals are serial and
non-overlapping. They lock, recover, snapshot owners, then delete only empty
snapshot. Non-empty snapshot refuses deletion. Lock loss, fsync/rename failure,
or barrier failure refuses operation without deletion.

## Fixture Oracle

`schema.json` is source of truth for fixture fields and event requirements.
`verify.rb` parses schema, fixtures, and goldens; it fails malformed input,
unknown events, ordering violations, fault/golden drift, missing release
terminal/sync sequence, missing `rm`/`prune` lock coverage, overlapping lock
intervals, and missing positive death proof. `fixtures.json` is typed trace input.
Every fixture has `initial`,
a complete ordered `trace`, and optional terminal `fault`. References such as
`nonce_a` resolve from top-level fixture values. `goldens.json` maps each
fixture ID to exact operation outcome, final observable state, and observations.

Trace event grammar:

| Event | Required fields | Meaning |
| --- | --- | --- |
| `lock.acquire` / `lock.release` / `lock.loss` | `actor` | Acquire, release, or lose volume lock. |
| `owner.pending` | `nonce`, `coordinator` | Build pending candidate. |
| `owner.active` | `nonce`, `run_id`, `process`, `mount_id` | Replace same-nonce pending candidate. |
| `pending.remove` | `nonce` | Remove exact pending owner after failed spawn. |
| `owner.remove` | `nonce`, `run_id`, `process_start_token` | Remove exact active owner. |
| `write.tmp` | `owners` | Write complete candidate `owners.json.tmp`. |
| `sync_all.tmp` | none | Sync tmp file after its write. |
| `rename.owners` | none | Atomically replace `owners.json`; this linearizes candidate. |
| `fsync.parent` | none | Sync directory after rename. |
| `barrier.spawn` / `barrier.release` | `run_id` | Spawn pre-exec child or permit exec. |
| `registry.active` | `run_id`, `nonce`, `pid`, `process_start_token`, `mount_id` | Write matching active run registry transition. |
| `registry.terminal` | `run_id`, `nonce` | Write terminal marker. |
| `registry.sync` | `run_id` | Sync registry transition. |
| `process.observe` | `pid`, `token`, `result` | Observe `absent`, `mismatch`, or `matching`. |
| `registry.observe` | `run_id`, `nonce`, `result` | Observe `terminal`, `readable`, or `unreadable`. |
| `recover.active` | `nonce`, `run_id`, `process_start_token` | Start recovery removal after positive death proof for exact active owner identity. |
| `rm` / `prune` | `actor` | Request deletion inside matching actor lock after recovery and owner snapshot. |
| `fault` | `point`, `operation` | Inject named failure immediately after point; trace stops. |

`owner.pending` and `owner.active` values are candidates only. They become
durable only after ordered `write.tmp`, `sync_all.tmp`, `rename.owners`, and
`fsync.parent`. `barrier.release` is invalid unless active owner plus matching
registry transition are durable. `fault.point` is one of `lock.loss`,
`sync_all.tmp`, `rename.owners`, `fsync.parent`, or `barrier.release`.

An event immediately followed by `fault` is attempted and fails; it does not
produce its normal durable postcondition. `rename.owners` remains linearized
before a later `fsync.parent` fault.

Any `recover.active` trace must carry `nonce`, `run_id`, and
`process_start_token` matching full active owner in `initial.owners`: `phase`,
`nonce`, `run_id`, `pid`, `process_start_token`, and `mount_id`. It must also
encode matching full run registry in `initial.registries`, with same fields plus
`terminal_status`. Positive proof is either matching terminal registry
observation or matching readable registry plus absent/mismatched observed
process token. Shorthand owner labels are forbidden for recovery.

Every failure golden requires `outcome: "refused:<point>"`,
`state.volume_exists: true`, and `observations.delete_attempted: false`.
For parent-fsync failure, rename has already linearized empty owner state;
volume still remains because caller must refuse deletion. Other durability
failures retain pre-failure owners. S3-5B tests must execute traces in separate
processes where stated by Sprint 03 and mutation-probe acquire, release,
recovery, and refusal edges. Fixtures are protocol goldens, not current-runtime
tests. `test_gate.sh` runs verifier locally and in benchmark CI.
