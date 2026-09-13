# Benchmark Runner Contract

## Scope

`benchmarks/benchmark-spec.yaml` is sole scenario source. `bench-runner` is a
standalone Rust crate under `benchmarks/runner/`; root Cargo workspace is not
modified.

```text
bench-runner verify-spec --spec PATH
bench-runner run --spec PATH --chunk N --chunks N --rounds N \
  --mode cold|warm|invalidate --out DIR \
  --docker PATH --lightr PATH
bench-runner merge --input DIR --out DIR
```

`verify-spec` rejects malformed YAML, non-250 corpus, duplicate IDs, unknown
category/tag/availability, missing non-supported reason, and missing supported
fixture/commands/assertions/source evidence. It permits no executable commands
for a non-supported scenario.

`run` selects source-list order where `index % chunks == chunk`; `chunks > 0`,
`chunk < chunks`, `rounds > 0`, and explicit `--mode` are required for S2
differential evidence. Legacy S1 callers without `--mode` retain their existing
private-per-round state semantics and are recorded as `cold`. Each
supported scenario first runs one unrecorded warmup pair, then records exactly
the supplied paired rounds. It creates
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
failure changes the scoped tool record to `failed`. S2 factor evidence requires
every declared assertion to have `docker_and_lightr` scope. Runner canonicalizes
those assertion results and hashes them; both tool hashes must match. This
compares declared artifact behavior, never Docker internal image IDs. A scenario
with tool-specific assertions is typed `incomparable` and yields no factor.
`command` assertions use
same token expansion and timeout. Assertion details are embedded in the scoped
command record; they are not separate JSONL records.

Supported assertion kinds: `exit_code`, `stdout_exact`, `stdout_regex`,
`stderr_regex`, `file_sha256`, `http_status`, `command`.

## Raw JSONL

Every record contains:

```text
schema_version, scenario_id, availability, tool, mode, round, pair_id, outcome,
started_at_unix_ms, ended_at_unix_ms, elapsed_ms, timeout_secs,
exit_code, command_sha256, stdout_sha256, stderr_sha256, spec_sha256,
fixture_tree_sha256, source_commit, docker_client_version,
docker_server_version, docker_api_version, lightr_version, lightr_sha256,
host_os, host_arch, host_kernel, hardware_id, output_evidence_sha256, assertions
```

`tool` is `docker`, `lightr`, or `skip`. `outcome` is `passed`, `failed`,
`skipped`, `timed_out`, or `incomparable`. `pair_id` is stable
`<scenario_id>:<mode>:<round>` identity shared by Docker and Lightr. Supported
scenarios emit exactly one `docker` and
one `lightr` record for each zero-based `round` in `0..rounds`. Non-supported
scenarios emit one `skip` record with `round: 0` and their reason. Skip records
set `fixture_tree_sha256`, `source_commit`, and `lightr_sha256` to `null`;
all other digest fields are SHA-256 hex, including SHA-256 of empty command
output. Version probe failure is a supported-scenario failure, not a fabricated
version. Schema version is `2`; decode rejects missing, empty, or malformed
required fields and required evidence digests.

`hardware_id` is collected before any record from host OS, architecture, kernel,
and OS-native model probe. macOS probes `sysctl hw.model` then CPU brand; Linux
probes DMI product name then `/proc/cpuinfo` model name. No hardcoded identity:
if all probes fail, `run` fails before writing records.

`merge` reads JSONL recursively, rejects malformed rows and duplicate
`(scenario_id, tool, mode, pair_id)` tuples, writes `merged.jsonl` plus
`summary.json`. For every supported scenario it rejects zero, unpaired, failed,
incomparable, or inconsistent fixture/spec/version/hardware/mode evidence.
Successful summaries report paired count, median and min/max per tool, and
`docker/lightr` factor only after matching output-evidence digests.

## Tests

Unit tests must prove duplicate-ID rejection, source-order chunk partition,
invalid chunk args, missing local fixture commit, failed command, timeout,
typed skip, duplicate raw tuple rejection, malformed evidence, mismatched
output evidence, failed pairs, successful factors, and inconsistent pair
identity. Mutation probe: remove output-evidence comparison; mismatched-output
test must fail before restore.
