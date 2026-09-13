# Benchmark Runner Contract

## Scope

`benchmarks/benchmark-spec.yaml` is sole scenario source. `bench-runner` is a
standalone Rust crate under `benchmarks/runner/`; root Cargo workspace is not
modified.

```text
bench-runner verify-spec --spec PATH
bench-runner run --spec PATH --chunk N --chunks N --rounds N --out DIR \
  --docker PATH --lightr PATH
bench-runner merge --input DIR --out DIR
bench-runner run-differential --spec PATH --chunk N --chunks N --rounds N --out DIR \
  --docker PATH --lightr PATH --mode cold
bench-runner merge-differential --input DIR --out DIR
```

`verify-spec` rejects malformed YAML, non-250 corpus, duplicate IDs, unknown
category/tag/availability, missing non-supported reason, and missing supported
fixture/commands/assertions/source evidence. It permits no executable commands
for a non-supported scenario.

`run` selects source-list order where `index % chunks == chunk`; `chunks > 0`,
`chunk < chunks`, and `rounds > 0` are required. It creates
`<out>/records.jsonl`, never prints synthetic success. Any selected supported
failure exits nonzero after writing complete records.

## Fixture Materialization

Only `source_evidence.projects[].repo: local` is runnable in S1. Derive repo
root from `spec.parent.parent`, verify `<commit>^{commit}`, then materialize
`fixture.path` from that commit using `git archive` into a scenario-private
directory under `<out>/fixtures/`. Hash that tree with SHA-256 over sorted
relative paths and bytes. Any unknown project type or failed materialization is
a supported-scenario failure.

## Command Environment

Runner expands only these tokens in declared shell commands:

```text
$DOCKER       exact --docker path
$LIGHTR       exact --lightr path
$FIXTURE_DIR  materialized scenario fixture directory
$OUTPUT_DIR   scenario-private output directory
```

Runner shell-quotes every expansion. Declared commands must not wrap these
tokens in shell quotes.

Unknown `$NAME`, missing executable, or a command timeout fails the supported
scenario. Commands run with `sh -c`; each has a 300-second deadline. On Unix,
spawn a new process group and kill that group on timeout. Never substitute a
host `docker` or `lightr` from `PATH`. Every scenario command and assertion
runs with `LIGHTR_HOME` set to its private output subdirectory.

## Assertions

Run declared assertions after both tool commands, once per round. Assertion
failure changes the scoped tool record to `failed`. `command` assertions use
same token expansion and timeout. Assertion details are embedded in the scoped
command record; they are not separate JSONL records.

Supported assertion kinds: `exit_code`, `stdout_exact`, `stdout_regex`,
`stderr_regex`, `file_sha256`, `http_status`, `command`.

## Raw JSONL

Every record contains:

```text
schema_version, scenario_id, availability, tool, round, outcome,
started_at_unix_ms, ended_at_unix_ms, elapsed_ms, timeout_secs,
exit_code, command_sha256, stdout_sha256, stderr_sha256, spec_sha256,
fixture_tree_sha256, source_commit, docker_client_version,
docker_server_version, docker_api_version, lightr_version, lightr_sha256,
host_os, host_arch, host_kernel, assertions
```

`tool` is `docker`, `lightr`, or `skip`. `outcome` is `passed`, `failed`,
`skipped`, or `timed_out`. Supported scenarios emit exactly one `docker` and
one `lightr` record for each zero-based `round` in `0..rounds`. Non-supported
scenarios emit one `skip` record with `round: 0` and their reason. Skip records
set `fixture_tree_sha256`, `source_commit`, and `lightr_sha256` to `null`;
all other digest fields are SHA-256 hex, including SHA-256 of empty command
output. Version probe failure is a supported-scenario failure, not a fabricated
version.

`merge` reads JSONL recursively, rejects malformed rows and duplicate
`(scenario_id, tool, round)` tuples, writes `merged.jsonl` plus `summary.json`.
It reports counts only; no derived statistical claim.

## S2 Differential JSONL

`run-differential` emits schema v2 only. It accepts `--mode cold`; `warm` and
`invalidate` fail with an explicit unsupported diagnostic until S2-5B. Before
writing any v2 row, Docker client and server must both equal `28.3.2`; failures
name observed values. Each v2 row retains every v1 field and adds non-empty
`mode`, `hardware_identity` (OS-native probe, no fallback), `pair_id`, and
`output_equivalence_sha256`, `equivalence_status`, and `cold_precondition`.
Equivalence digest hashes only ordered shared `docker_and_lightr` assertion
kind, declared expected value, and normalized passing result; command streams
never enter it. No shared assertion produces `no_shared_assertions` and no
factor. Before each cold pair, runner removes only declared Docker build tag,
verifies it absent, forces Docker `--no-cache`, and clears/verifies private
`LIGHTR_HOME`; unsupported Docker command shapes reject cold before recording.

`merge` accepts v1 only. `merge-differential` accepts v2 only and rejects v1,
mixed/malformed rows, empty required fields, duplicate `(pair_id, tool)` tuples,
and empty evidence. Its factor is emitted only for successful same-fixture,
same-hardware Docker 28.3.2 pairs with equal output-equivalence digests;
otherwise summary has a typed no-factor reason.

## Tests

Unit tests must prove duplicate-ID rejection, source-order chunk partition,
invalid chunk args, missing local fixture commit, failed command, timeout,
typed skip, and duplicate raw tuple rejection. Mutation probe: remove
duplicate-ID validation; duplicate-ID test must fail before restore.
