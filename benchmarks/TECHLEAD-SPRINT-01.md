# Sprint 01: Frozen Docker Contract and Executable Evidence Harness

## Owner Decisions

- Compatibility target: Docker CLI/Engine `28.3.2`.
- Engine API version is discovered and recorded from the pinned engine; do not
  infer it from a different Docker installation.
- Scope: Docker core, OCI, build, compose, runtime/network/volume/security
  surfaces exposed by current Lightr CLI.
- Explicitly out of Sprint 01: Swarm/orchestration implementation; Docker
  plugin execution. Plugin adapter is research-only and cannot be described as
  compatibility until an actual Docker plugin passes its lifecycle probe.

## Sprint Exit

Sprint passes only when:

1. A machine-readable parity corpus freezes every target operation as
   `supported`, `unsupported`, `hardware_gated`, or `out_of_scope`.
2. Every `supported` operation maps to an evidence-backed Lightr command and a
   Docker `28.3.2` command. No generic or invented shell command remains.
3. `bench-runner` creates fail-closed per-chunk raw JSONL evidence.
4. Clone/extract/report/validate scripts execute noninteractively.
5. CI validates spec, builds runner and Lightr CLI, and runs an actual smoke
   chunk. Full 20-chunk matrix remains manual until smoke is green.

## Frozen Runner Interface

```text
bench-runner verify-spec --spec PATH
bench-runner run --spec PATH --chunk N --chunks N --rounds N --out DIR \
  --docker PATH --lightr PATH
bench-runner merge --input DIR --out DIR
```

All scripts use `benchmarks/scripts/`. CI must never reference
`benchmarks/runner/scripts/`.

## Work Packages

| WP | Owner files | Execution | Dependency |
|---|---|---|---|
| S1-A | `benchmarks/benchmark-spec.yaml`, `benchmarks/parity/**`, `benchmarks/MANIFEST.md` | Build verified Docker 28.3.2 corpus; correct tag taxonomy, peeled Git hashes, repo/fixture paths, and scenario states | none |
| S1-B | `benchmarks/runner/**` | Implement standalone runner; raw evidence, chunking, strict spec validation, merge | frozen interface |
| S1-C | `benchmarks/scripts/00_*.sh`..`06_*.sh`, `verify_hashes.sh` | Real noninteractive environment, fixture, Lightr build, smoke, and chunk scripts | frozen interface |
| S1-D | `benchmarks/reporter/**`, `benchmarks/scripts/07_*.sh`..`09_*.sh`, `validate_contract.py` | Real merge/report/validation/archive from raw evidence | frozen raw-record contract |
| S1-E | `benchmarks/.github/workflows/benchmark.yml`, `benchmarks/ci/**` | CI static path gate + root Lightr build + runner smoke; full matrix manual | S1-B/C/D contracts |
| S1-F | `benchmarks/research/docker-plugin-adapter.md` | Read-only Docker plugin adapter feasibility and one lifecycle probe design | none |

## Conflict Map

- S1-A, B, C, D, E, F own disjoint paths: conflict-free.
- S1-E is authored against frozen interfaces only; it is not landed until
  S1-B/C/D pass their local gates.
- `benchmark-spec.yaml` and `MANIFEST.md` are S1-A-only.
- Lead performs integration, not agents.

## Evidence Gates

### S1-A

- Pin Docker CLI/Engine and Engine API version with command output evidence.
- Resolve annotated tags to peeled commit IDs (`refs/tags/<tag>^{}`).
- Clone pinned repos and assert each declared fixture path exists.
- Every scenario has one status; only `supported` is runnable.
- Mutation probe: alter a fixture path or hash; corpus gate fails.

### S1-B

- `cargo test --manifest-path benchmarks/runner/Cargo.toml`.
- Duplicate-ID, chunk partition, failed command, malformed JSONL, duplicate raw
  record tests.
- Mutation probe: remove duplicate validation; test fails.

### S1-C

- `bash -n` all owned scripts.
- No `read`, `select`, or simulated success in CI path.
- Wrong hash and missing fixture probes fail.

### S1-D

- Synthetic raw JSONL fixture proves CSV, JSON, Markdown generation.
- Missing round and failed non-gated record fail validation.
- No p-value, CI, or effect-size claim without implemented calculation.

### S1-E

- Workflow YAML parse plus static referenced-path check.
- Linux runner uses `sudo docker` or explicit Docker service setup; no relying
  on `usermod` group refresh.
- Smoke invokes actual runner binary and actual root Lightr binary.
- `workflow_dispatch` full matrix uses 20 chunks only after owner verifies
  account concurrency; otherwise matrix may queue but must not fail.

### S1-F

- Describe Docker plugin protocol/lifecycle, privilege boundary, daemonless
  implications, compatibility risks, and a single falsifiable adapter probe.
- No implementation claim.

## Return Card

```text
WP: S1-X
Files: exact list
Tests: commands + output
Mutation probe: red -> restored green
Known limits: exact
Framing error: exact
```

## Integration Order

S1-A -> lead corpus review -> S1-B/C/D local verification -> S1-E -> cold
review -> commit -> PR -> CI -> merge.

## Program Sequence

| Sprint | Scope | Exit evidence |
|---|---|---|
| S1 | Frozen Docker 28.3.2 corpus and executable evidence harness | Real smoke evidence; no simulated pass paths |
| S2 | Runtime, OCI, image, Dockerfile and BuildKit parity | Per-surface Docker/Lightr differential tests and benchmark records |
| S3 | Compose, networking, storage, security, health, secrets/configs and logging | Per-surface acceptance plus raw benchmark evidence |
| S4 | Compat API/socket, Swarm control-plane/service lifecycle and platform validation | API conformance, Swarm lifecycle proof, named-hardware platform evidence |
| S5 | Documentation and publication corpus | Docs rebuilt from S1-S4 evidence; parity ledger reconciled; no unsupported feature claimed done |

### Sprint 5: Documentation Is Evidence-Last

S5 owns documentation only after S1-S4 gates are green.

- Reconcile `docs/spec/parity-audit.md`, `docs/FEATURE-LEDGER.md`,
  `docs/spec/feature-parity.md`, benchmark methods/results, commands, and
  release/runbooks against measured evidence.
- Amend Swarm's current non-goal only under this owner decision and only with
  the S4 acceptance evidence.
- Preserve `honest_gated`, `unsupported`, hardware-gated, and deferred states;
  no prose upgrades a capability.
- Every measured claim links named hardware, exact command/reproduce path, raw
  artifact digest, and CI run URL.
- Final independent cold review checks that docs do not exceed S1-S4 evidence.
