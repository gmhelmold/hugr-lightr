# TechLead Execution Wave: Real Benchmark Harness

## Baseline

Files under `benchmarks/` are untracked. Do not commit, push, or merge without
owner instruction. `benchmark-spec.yaml` is YAML-valid but its 250 scenario
commands and fixture paths are not yet evidence-backed.

## Goal

Produce an executable, fail-closed Docker-vs-Lightr benchmark harness. A green
CI job must mean the runner executed selected scenarios, wrote raw evidence,
and validated declared gates. It must never mean a shell script printed `OK`.

## Non-Negotiable Contract

### Scenario Contract

`benchmark-spec.yaml` remains sole scenario source.

Every runnable scenario needs:

- `id`, `category`, `project`, `docker_cmd`, `lightr_cmd`
- at least four metrics and two taxonomy tags
- `availability: supported|unsupported|hardware_gated|out_of_scope`; every
  non-supported state carries a reason

Runner behavior:

- Scenario selection is deterministic: source-list order, `index % chunks == chunk`.
- Non-supported states record typed skips; none masquerades as pass.
- Missing executable, fixture, command failure, assertion mismatch, timeout, or
  missing raw sample records `failed`; no `echo` success path.
- Raw evidence is JSONL per chunk. Each record includes scenario id, tool,
  round, phase, start/end timestamps, timeout, exit status, stdout/stderr
  digest, outcome, spec digest, fixture tree digest, source commit, Docker
  client/server/API version, Lightr version/digest, and host/kernel identity.
- Runner exits nonzero if any selected supported scenario fails.

### Assertion Contract

`validation` is explanatory prose, never an oracle. Supported scenarios require
structured assertions evaluated after explicit normalization. Supported types:
`exit_code`, `stdout_exact`, `stdout_regex`, `stderr_regex`, `file_sha256`,
`http_status`, and `command`. Missing assertions are spec validation errors.

### Runner CLI Contract

Standalone crate: `benchmarks/runner/` with its own `[workspace]`; do not
modify root Cargo workspace.

```text
bench-runner verify-spec --spec PATH
bench-runner run --spec PATH --chunk N --chunks N --rounds N --out DIR \
  --docker PATH --lightr PATH
bench-runner merge --input DIR --out DIR
```

`verify-spec` checks YAML parse, required fields, unique IDs, metric/tag counts,
known categories, and taxonomy membership. `run` writes JSONL only. `merge`
combines raw JSONL and emits a machine-readable summary; it does not fabricate
statistical claims.

### Fixture Contract

Fixture extraction is evidence-driven.

- Clone exact, verified commits under `benchmarks/work/repos/`.
- Every declared fixture path must exist at the pinned commit.
- Missing declared fixture is a failure, not `echo "not found"`.
- Repo / file inventory mismatches update the spec in a lead-owned follow-up;
  agents do not invent paths or substitute a different repo silently.

### CI Contract

- Canonical scripts path: `benchmarks/scripts/`.
- No `read`, prompts, simulated pass, or commented-out core command in CI path.
- CI invokes `sudo docker` or starts Docker explicitly; never relies on an
  in-session `usermod` change.
- Build root `lightr-cli` via `cargo build -p lightr-cli --release`, then pass
  exact binary path to runner. Do not claim `lightr compose` exists unless CLI
  help confirms it.
- Matrix chunks are 0..19. Full suite is `workflow_dispatch` only. Do not set
  `max-parallel` above account capacity; use 10 until owner confirms current
  GitHub account concurrency is at least 20.

## Wave Plan

| WP | Owner files | Scope | Depends on |
|---|---|---|---|
| W0 | `benchmarks/benchmark-spec.yaml`, this contract | Lead-only fixture inventory/schema corrections | none |
| W1 | `benchmarks/runner/**` | Standalone Rust runner + unit tests | contract |
| W2 | `benchmarks/scripts/00_*.sh` through `06_*.sh`, `verify_hashes.sh` | Non-interactive environment, clone, extract, build, dry-run, chunk execution | runner CLI contract |
| W3 | `benchmarks/scripts/07_*.sh` through `09_*.sh`, `validate_contract.py`, `benchmarks/reporter/**` | Merge, analysis, validation, archive; fail closed | runner raw-record contract |
| W4 | `.github/workflows/benchmark.yml` | CI integration after W1-W3 verify | W1, W2, W3 |

## Conflict Map

- W1/W2/W3: conflict-free.
- W4: sequential after W1-W3; owns workflow only.
- W0: lead-only; no agent edits spec until fixture inventory evidence exists.
- `MANIFEST.md` is lead-only after all files stabilize.

## Acceptance Gates

### W1

- `cargo test --manifest-path benchmarks/runner/Cargo.toml`
- `bench-runner verify-spec --spec benchmarks/benchmark-spec.yaml`
- Synthetic test proves chunk 0 and chunk 1 partition scenario IDs exactly once.
- Mutation probe: remove unique-ID validation; its duplicate-ID test fails.

### W2

- ShellCheck or `bash -n` passes every owned script.
- Scripts use repository-root-derived absolute paths.
- Clone script verifies exact commit after clone.
- Extraction script fails when a required fixture path is absent.
- Chunk script passes `CHUNK`/`TOTAL_CHUNKS` to runner and writes only its chunk directory.
- Mutation probe: alter expected hash or fixture path; script exits nonzero.

### W3

- Merge fails on malformed JSONL or duplicate `(scenario, tool, round)` records.
- Validation fails on missing rounds and a non-gated failed record.
- Analysis emits actual CSV/JSON/Markdown from a checked-in synthetic JSONL fixture.
- Mutation probe: remove one synthetic round; validation fails.

### W4

- Workflow YAML parses.
- All paths exist.
- No interactive commands (`read`, `select`) in CI-invoked scripts.
- `verify-spec` and runner tests run on push/PR.
- Full matrix runs only manually, chunks 0..19, then merge waits for all chunks.
- Mutation probe: alter a script path; static path gate fails.

## Return Card

```text
WP: <id>
Files: <exact list>
Tests: <exact commands + output>
Mutation probe: <break -> red; restore -> green>
Known limits: <none | exact>
Framing error: <one issue in this contract, if any>
```

## Landing Order

W1 + W2 + W3 cold-verified independently, then W4. Lead updates W0 and
`MANIFEST.md` only after all implementation gates pass.
