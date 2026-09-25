# Sprint 03: Ecosystem Evidence and Surface Closure

## Status and Rule

This is an execution plan, not an implementation claim. S3 follows S1/S2 and
owns compose, networking, storage, security, health, secrets/configs, and logs
evidence. The compatibility target remains Docker CLI/Engine `28.3.2`.

S3 is brownfield-first. Existing behaviour is lifted only when a named shipped
test witnesses it. Unshipped behaviour is a build work package, never a lifted
requirement. Every unsupported or engine-gated surface stays typed as such.

## Existing Surface Map

| Surface | Current state | Evidence witness | S3 treatment |
|---|---|---|---|
| Compose project, profiles, listener registration, down | shipped | `crates/lightr-cli/src/handlers/compose.rs` tests; `crates/lightr-build/src/build/compose/**` tests | lift + acceptance |
| Compose suspended-state resume and packet-trigger activation | not proven | supervisor currently cold-spawns after TCP `accept()` | ADR gap; stop gate |
| Network create/ls/rm/inspect | shipped | `crates/lightr-cli/src/handlers/network_tests.rs` | lift + acceptance |
| Network connect/disconnect | unsupported | `handlers/network.rs` explicit exit-2 path | typed refusal test |
| Volume administration | shipped | `crates/lightr-store/src/store/volume_tests.rs` | lift + acceptance |
| Named-volume mounting and in-use refcount | not shipped | `handlers/volume.rs` states `in_use=false` | build before parity claim |
| Healthcheck | shipped | `crates/lightr-run/src/healthcheck_tests.rs` | lift + acceptance |
| Secrets/configs | shipped, local-disk boundary | `crates/lightr-run/src/run/tests/secrets_tests.rs` | lift + permission/key acceptance |
| Logs | shipped, file-mtime timestamp boundary | `crates/lightr-cli/src/handlers/logs_tests.rs` | lift + acceptance |
| Security controls | shipped subset varies by engine | engine/run tests selected by S3-0A | inventory before claim |

## Frozen S3 Laws

1. No resident Lightr daemon. Supervisors are stack-scoped and TTL-bounded.
2. Docker comparison uses Lightr `--eager`. Lazy mode is separate acceptance.
   It must not claim packet-trigger activation or state resume until exact
   runtime evidence proves the ADR-0015 semantics.
3. A secret/config changes the memo key and a missing store ref aborts before
   spawn. Local secrets are filesystem-permission protected files, not a vault.
4. No live network hot-plug claim. `network connect`/`disconnect` return typed
   unsupported errors until daemonless semantics exist.
5. A named volume is not supported for workload storage until mounting and live
   in-use accounting are implemented and tested.
6. Engine-specific security behaviour is reported as supported, engine-gated,
   or unsupported. It is never generalized from one engine to another.
7. No performance factor, median, range, or speed claim without raw artifacts,
   named hardware, and reproduce command.

## Frozen Evidence Contract

S1 raw schema v1 and S2 differential schema v2 remain frozen. S3 gets a
separate envelope with `schema_version: 3` and `s3_evidence_version: 1`,
decoded only by new commands:

```text
bench-runner run-s3 --spec benchmarks/s3/scenarios.yaml --chunk N --chunks N \
  --rounds N --out DIR --docker PATH --lightr PATH
bench-runner merge-s3 --input DIR --out DIR
```

`run-s3` writes `records.s3.jsonl`; `merge-s3` writes `merged.s3.jsonl` and
`summary.s3.json`. A record is exactly this JSON object:

```text
{
  "schema_version": 3,
  "s3_evidence_version": 1,
  "case_id": "[a-z0-9][a-z0-9._-]{0,127}",
  "round": "u32",
  "tool": "docker|lightr",
  "outcome": "passed|failed|timed_out|skipped",
  "elapsed_ms": "u128",
  "fixture_sha256": "64 lowercase hex",
  "host_identity": "non-empty OS-native string",
  "docker_client_version": "28.3.2",
  "docker_server_version": "28.3.2",
  "lightr_sha256": "64 lowercase hex",
  "lightr_engine": "native|ns|vz|fc|null",
  "command_sha256": "64 lowercase hex",
  "assertions": "non-empty S3Assertion[]",
  "semantic_receipt_blake3": "64 lowercase hex",
  "surface_receipt": "non-empty canonical JSON object"
}
```

`S3Assertion` is exactly `{id: String, expected: JSON, observed: JSON,
passed: bool, detail: String, shared: bool}`. `id` is unique per record.
`expected`, `observed`, and `surface_receipt` use JCS RFC 8785 canonical JSON,
encoded as UTF-8. JSON numbers are signed integers in
`[-9007199254740991, 9007199254740991]`; fractions and out-of-range values are
rejected. Parsing rejects duplicate object keys, non-finite numbers, and invalid
UTF-8 before JCS. Surface receipts compare by complete JCS byte equality. The
shared projection is shared assertions sorted
by bytewise UTF-8 `id`; its BLAKE3 preimage is `S3R1\0` followed, per assertion,
by big-endian u32 byte lengths plus JCS bytes of `id`, `expected`, `observed`,
and ASCII `true` or `false`. Its BLAKE3 hex is required
`semantic_receipt_blake3`. `detail` is tool-local and excluded.

Candidate grouping key is `(case_id, round)`. A comparable pair contains exactly
Docker plus Lightr rows, both passed, equal fixture/host identity, Docker
versions `28.3.2`, non-empty shared projection, and equal
`semantic_receipt_blake3`.
Then `factor = docker.elapsed_ms / lightr.elapsed_ms` only when
`lightr.elapsed_ms > 0`; otherwise it is null with `zero_lightr_elapsed`. Every
other pair has null factor plus exactly one reason. Merge applies this ordered
table: (1) row count/tool set is not exactly Docker plus Lightr →
`incomplete_pair`; (2) either Docker version is not `28.3.2` →
`unsupported_version`; (3) either outcome is not passed → `unsuccessful_pair`;
(4) fixture or host identity differs, Docker `lightr_engine` is not null, or
Lightr `lightr_engine` is null → `incomparable_pair`; (5) either
shared projection is empty → `no_shared_assertions`; (6)
`semantic_receipt_blake3` differs → `output_mismatch`; (7) Lightr elapsed is zero →
`zero_lightr_elapsed`; otherwise emit factor. Malformed or mixed evidence is
rejected before pair evaluation. Every overlapping branch has a fixture golden.
`summary.s3.json` is `{schema_version:3,s3_evidence_version:1,factors:[...],
statistics:null|{sample_count:u64,median_factor:f64,factor_range:{min:f64,max:f64}}}`.

S3 commands reject v1/v2 rows, unknown versions, blank lines, duplicate
`(case_id, round, tool)` tuples, missing required fields, bad grammar, and mixed
host/tool pairs. Diagnostics are exactly `unsupported S3 schema version: N`,
`unsupported S3 evidence version: N`, `empty required S3 field: FIELD`,
`duplicate S3 tuple: (CASE, ROUND, TOOL)`, and `mixed S3 evidence: REASON`.

Legacy goldens are captured at baseline `bea26dd`: `run`, `merge`,
`run-differential`, and `merge-differential` each run valid input, blank lines,
unknown fields, unknown schema versions, and mixed files. S3-0B asserts each
legacy stdout, stderr, exit code, and output bytes remain identical, while each
legacy command rejects schema 3. S3 commands reject v1/v2. This matrix includes
the existing v1 `merge` blank-line skip behavior unchanged.

`benchmarks/s3/scenarios.yaml` is S3-only scenario ownership. It rejects case
ID duplicates and records a `legacy_overlap: none|CASE_ID` check against v1/v2
corpora; `CASE_ID` is an error. S3-0B supplies positive S3 fixture rows and
negative v1/v2/mixed/duplicate/missing-receipt fixtures. Root
`.github/workflows/benchmark.yml` runs `run-s3` then `merge-s3` through the
isolated S3 job/artifact topology below.

## Semantic Matrices

### Compose

Each S3 scenario declares argv arrays for Docker command, Lightr command,
readiness probe, teardown command, and resource kinds. Every argv token may use
only `${DOCKER}`, `${LIGHTR}`, `${FIXTURE_DIR}`, and `${S3_NAMESPACE}`; unknown
placeholders reject. Every declared resource name is exactly
`${S3_NAMESPACE}` or `${S3_NAMESPACE}-SUFFIX`; literal resource names reject.
Runner expands tokens once, records expanded command digest, and checks each
declared Docker resource by exact expanded name. Runner generates a
128-bit random `run_nonce`, records it in both receipts, and namespaces every
Docker project/container/network/volume/image as `s3-<case>-<round>-<nonce>`.
It checks resources absent before command and after teardown, serializes cases
that claim host ports, and rejects a command containing a resource name outside
the generated namespace. Lightr gets a private `LIGHTR_HOME` and project name.
Comparable compose scenarios must contain `lightr compose up --eager`; missing
or conflicting eager flags reject before recording.

For eager comparable cases, `elapsed_ms` starts immediately before tool exec and
stops only when its identical declared readiness oracle succeeds; command return
before readiness does not stop timer. Preflight, observations, teardown, and
cleanup stay outside the timer. `surface_receipt.registration_elapsed_ms` records
command-return time separately and never enters a factor. Observations are
captured after readiness and before teardown. Every eager comparable case declares
non-empty shared assertions for command exit, service port readiness, dependency
ordering, selected profiles, project isolation, teardown, logs, and health state
when configured. Deleting one shared assertion or making normalized Docker and
Lightr observations diverge must produce `no_shared_assertions` or
`output_mismatch`, never a factor.

Lazy cases are not Docker comparisons. S3-1B is a hard predecessor for every
lazy case. Before listener bind it writes
`<stack>/services/<name>/state.json` as JCS
`{phase:"suspended",instance_id,artifact_sha256,pid:null}`. Listener accepts a
connection but must not resume until it reads one payload byte; it buffers then
replays that byte after resume. State transitions to `resuming` then `running`
with same `instance_id` and `artifact_sha256`, plus workload PID. Receipt records
accept, first-byte, and resume monotonic nanoseconds and the state JSON digest.
A no-payload probe connects then waits 500 ms, polling state/PID every 50 ms; it
requires phase `suspended` and no PID. A payload probe requires first-byte before
resume, same identity/artifact, byte replay, and running PID. Repeated
connections, proxy behavior, TTL, and `compose down` cleanup are separate
assertions. Current accept-triggered cold spawn fails this gate. S3 exits only
after runtime proof, or an owner-approved ADR supersedes ADR-0015.

Lazy `surface_receipt` is exactly JCS object
`{kind:"lazy_v1",state_path:String,state_sha256:String,instance_id:String,
artifact_sha256:String,pid:null|u32,accept_ns:u64,first_byte_ns:u64,
resume_ns:u64,replay_sha256:String}`. Verifier reads `state_path`, recomputes
state SHA-256, requires receipt identity/artifact/PID match state, requires
`accept_ns <= first_byte_ns <= resume_ns`, and compares replay SHA-256 to sent
payload. Tampered timestamp, identity, PID, state digest, or replay fixtures
must fail before a lazy case records passed.

### Security

`benchmarks/s3/security/inventory.json` is authoritative. It has one row for
every closed inventory item: user, hostname, labels, tty, init, privileged,
read-only rootfs, cap-add, cap-drop, seccomp, AppArmor, memory limit, CPU limit,
pids limit, ulimit, OOM score adjustment, shared-memory size, tmpfs, network
mode, add-host, healthcheck, secret, and config. Each row maps CLI parser field,
RunConfig field, RunSpec/ExecSpec field, enforcement site, and engine outcome.
S3-3 adds a compile-forced `SecurityControl` mapping at each parser-to-spec
lowering and a test that the mapping equals this inventory. The gate fails an
unmapped/duplicate control, missing engine outcome/oracle/mutation probe, or a
source mapping not present in the inventory. Every row names engine/platform
prerequisite, enforced/refused/unsupported result, observable oracle, safe
fixture boundary, and exact shipped witness or build work package. Unknown rows
fail closed; one engine's result never generalizes to another.

### Volume Lifecycle

S3-5A freezes durable ownership before code. Each volume has
`.lightr/owners.json` version 1. Every owner has
`phase:"pending"|"active"` and `nonce:32-byte-hex`. Pending has exactly
`coordinator_pid`, `coordinator_start_token`, and no `run_id`/child PID/mount;
active has exactly `run_id`, `pid`, `process_start_token`, and `mount_id`, with
no coordinator fields. PID alone never establishes ownership. Unix serializes every acquire, release, rm,
and prune under `flock(.lightr/lock)`; Windows is engine-gated until an equally
atomic lock exists. A writer writes `owners.json.tmp`, `sync_all`, atomically
renames it to `owners.json`, then fsyncs parent directory. That rename is the
linearization point.

On Linux `process_start_token` is `linux:<pid>:<starttime>`, where `starttime`
is field 22 of `/proc/<pid>/stat`, parsed after its final `)` delimiter. Other
platforms are engine-gated until they expose an equally stable token. Acquire is
two-phase. First it fsyncs a `pending` owner with fresh nonce plus coordinator
PID/token, never a child PID. It spawns child behind a pipe barrier before exec,
reads child PID/token, then atomically replaces same-nonce pending owner with an
`active` owner containing run ID, child PID/token, and mount IDs. Only then does
parent release child through barrier. Failed spawn removes exact pending nonce
under lock.

`$LIGHTR_HOME/run/<run_id>/volume-owner.json` records pending/active nonce and
is fsynced at each transition; terminal status is fsynced after teardown. Release
removes only matching active `(run_id, nonce, start token)` after terminal status.
Shared mounts are distinct active owner records. Recovery removes an active owner
only with positive death proof: readable matching registry plus terminal status,
or readable matching registry plus Unix `/proc/<pid>/stat` absence or mismatched
start token. It removes pending only when coordinator death is proven by the same
token check and no active record carries its nonce. Missing, malformed, unreadable,
or nonce-mismatched registry is ambiguity and refuses rm/prune. rm/prune lock,
recover, snapshot owners, and refuse non-empty snapshots before deletion.
Lock-loss or fsync/rename/barrier failure refuses operation without deletion.
S3-5B uses separate processes to test normal teardown, shared mounts, failed
spawn, SIGKILL owner, stale/PID-reused ledger, and concurrent rm/prune. It
mutation-probes acquire, release, recovery, and refusal edges.

## Work Packages

| WP | Owner files | Dependency | Acceptance |
|---|---|---|---|
| S3-0A | `benchmarks/s3/corpus/**`, `benchmarks/s3/corpus_gate/**` | none | Generate `corpus.json`, `requirements.json`, `goldens.json`, and `cards.json` schemas together. Clause ID records baseline `bea26dd`, path, symbol, SHA-256 of `git show bea26dd:PATH`, package, manifest, target selector exactly one of `--lib`, `--bin NAME`, or `--test NAME`, plus `cargo test --manifest-path PATH -p PACKAGE SELECTOR SYMBOL -- --exact`, expected exit, stdout/stderr SHA-256, golden ID, and card ID. Gate runs argv in baseline worktree and requires `running 1 test` plus `test SYMBOL ... ok` before accepting hashes; it rejects post-baseline paths, missing/duplicate mappings, byte drift, bad result, or unexecuted witness with `s3 corpus: REASON`. |
| S3-0B | `benchmarks/s3/{contract,fixtures}/**`, `benchmarks/runner/**` | S3-0A | Implement frozen `s3_evidence_version: 1`, `run-s3`, and `merge-s3`; preserve v1/v2 behavior; run positive and negative envelope fixtures. |
| S3-1A | `benchmarks/s3/compose/eager/**`, `crates/lightr-build/src/build/compose/**`, `crates/lightr-cli/src/handlers/compose.rs` | S3-0B | Eager-only semantic matrix and exact assertion mutation probes. |
| S3-1B | `benchmarks/s3/compose/lazy/**`, compose ownership, `crates/lightr-run/src/run/{suspend.rs,spawn.rs}`, `crates/lightr-engine/src/engine/{mod.rs,native.rs,ns/**,vz/**}` | S3-1A | Freeze `SuspendResume` API, state-file authority, barrier, crash/TTL cleanup; implement and prove first-payload suspended-state resume before any lazy case. No cold-spawn behavior is renamed resume. |
| S3-2 | `benchmarks/s3/network/**`, `crates/lightr-cli/src/handlers/network*.rs`, `crates/lightr-run/src/network/**` | S3-0B | Registry lifecycle, spawn-time membership/DNS where engine permits, and hot-plug typed refusal. |
| S3-3 | `benchmarks/s3/security/**`, `crates/lightr-core/src/core/limits.rs`, `crates/lightr-run/src/{healthcheck,secrets}.rs`, `crates/lightr-cli/src/handlers/run/**`, `crates/lightr-engine/src/engine/{spec.rs,ns/**}` | S3-0B | Complete security matrix before claims; reject zero-effective sub-milli CPU before provisioning; test CPU rounding/overflow and memory-unit overflow; health retry/start-period/timeout; secret/config key, modes, and missing-ref failure. |
| S3-4 | `benchmarks/s3/logs/**`, `crates/lightr-cli/src/handlers/logs*.rs`, `crates/lightr-run/src/run/logs.rs` | S3-0B | stdout/stderr, tail, bounded follow, missing run, and mtime-only timestamp disclosure. |
| S3-5A | `benchmarks/s3/volume/contract/**` | S3-0B | Freeze named-volume mount/refcount lifetime, crash cleanup, and engine-gated semantics before code. |
| S3-5B | `crates/lightr-store/src/store/volume*.rs`, `crates/lightr-cli/src/handlers/volume.rs`, `crates/lightr-run/src/run/{mount,bindmat,types,specdisk,spawn}.rs`, `benchmarks/s3/volume/**` | S3-1B, S3-5A | Mounted read/write lifecycle; remove/prune refuses in-use volume; no stale refcount after teardown. |
| S3-6 | `benchmarks/s3/report/**`, `.github/workflows/benchmark.yml` | S3-1A, S3-1B, S3-2, S3-3, S3-4, S3-5B | `s3-verify` builds/tests runner; `s3-smoke` needs it, writes only `benchmarks/work/s3/raw/chunk-0`, then uploads `s3-raw-0` with `if: always()`. `s3-merge` needs verify/smoke and runs `if: always() && needs.s3-verify.result == 'success'`; it downloads `s3-raw-*` with missing artifacts tolerated, runs merge wrapper, captures status, uploads raw/partial merged workspace with `if: always()` and `retention-days: 30`, then exits captured failure. |

## Conflict Map and Order

`S3-0A -> S3-0B` is sequential because both define shared evidence terms.
After S3-0B, S3-1A, S3-2, S3-3, S3-4, and S3-5A are parallel. S3-1B follows
S3-1A; S3-5B follows S3-1B and S3-5A. S3-6 follows S3-1A, S3-1B, S3-2, S3-3, S3-4, and
S3-5B.

No surface agent edits `benchmarks/runner/**` except S3-0B. No surface agent
edits shared workflow/report paths except S3-6.

## Required Probes

- Compose: delete a shared eager assertion, proxy behavior, listener cleanup, or
  packet/resume behavior; respective acceptance fails.
- Network: turn a hot-plug refusal into success; refusal test fails.
- Secrets/configs: remove key contribution or relax secret mode; test fails.
- Health: remove retry/start-period transition; state-machine test fails.
- Logs: remove bounded follow exit or disclosure; test fails.
- Volumes: remove acquire/release, stale recovery, or live in-use check;
  lifecycle/removal/prune test fails.
- Evidence: delete required receipt or mix incompatible records; merge fails.

## Sprint Exit

S3 exits only when every declared surface has a named `run-s3` case, raw JSONL
artifact, and retained digest; every engine-specific result is correctly typed;
S3-1B proves the ADR semantics or an owner-approved ADR supersedes them; S3-5B
closes volume storage; and S3 CI is green. S4 and S5 claims remain out of scope.
