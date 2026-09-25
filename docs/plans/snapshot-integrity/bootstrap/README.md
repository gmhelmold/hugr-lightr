# SI-00 bootstrap delivery

Issue #153; campaign #152; draft PR #154, targeting `fix/snapshot-integrity`.
Owner authorized SI-00 execution on 2026-09-17. The technical contract remains
v2.2 at `e308119`; no runtime protocol is enabled by these scripts or documents.

## Delivered surfaces

- [ADR-0020](../../../adr/0020-snapshot-integrity-activation.md): proposed narrow
  amendments, bounded metadata framing, source-level activation and migration limits.
- [Seams](CONTRACTS.md): callable interfaces, bounds, error/outcome mapping,
  actual Store/cache domains, roots and recovery support obligations.
- [Caller map](caller-map.md): inspected production boundaries, raw32 build-cache
  root, explicit limits of syntactic inventory, and owning WPs.
- [Experiments](EXPERIMENTS.md): E01–E22 checkpoints, negative controls,
  bounded incident investigation and performance registration methodology.
- [Native policy](expected-tests.json): required host triples, actual existing
  smoke names and the one visible preexisting ignored test.
- `scripts/si00/`: executable validator, native runner, complete-matrix gate,
  inventory, OS probes, constrained-filesystem probe, fixture generator, CLI
  baseline and adversarial tests of evidence validation. Python stdlib only.
- `.github/workflows/si00-bootstrap.yml`: immutable action pins, read-only token,
  actual five-target execution, raw artifacts and a fail-closed matrix check.

## Reproduce without touching a real user Store

From a checked-out commit with Rust 1.96.0, Git and Python >=3.9:

```sh
python3 -m unittest discover -s scripts/si00 -p 'test_*.py' -v
python3 scripts/si00/inventory.py --output /tmp/si00-inventory.json
python3 scripts/si00/native.py --profile linux-x86_64 --out /tmp/si00-native-fresh
```

Choose the actual matching profile; cross-compilation cannot satisfy it. Output
directories must be new. Native tests use the repository's existing disposable
fixtures; probes allocate only under the owned output. Artifacts include test
binaries, logs, hashes, exact checkout parents, required policy and capability
observations. Baseline formatting failure remains an explicit failed formatting
gate; a successful native capability job does not mean fmt or runtime passed.

On an isolated Linux runner with GNU time:

```sh
cargo +1.96.0 build --locked --release --bin lightr
python3 scripts/si00/baseline.py --binary target/release/lightr --out /tmp/si00-cli-fresh
```

This is the pre-remediation baseline, not a pass against final performance
budgets. Source/destination/Store are disjoint owned temporary roots. F0–F5
require restored-tree equality; failure is retained, not silently omitted.
F6's new history semantics cannot be measured using the legacy binary.

`resource_probe.py` is CI-only unless deliberately run in a disposable Linux
host with passwordless sudo and loop/mount/mkfs support. It fills only an owned
32 MiB ext4 image, verifies retained bytes and safe retry, and unmounts it. On
unmount failure it leaves that isolated mount intact and fails rather than
recursively deleting mounted contents. This proves a runner capability, not
Lightr recovery. Do not run it on an arbitrary developer workstation.

## Trust and stage boundaries

The aggregate receives expected identity/policy from its own checkout and
requires all profile artifacts, commands, binary hashes and named tests. The
receipt cannot choose its required tests or mark ignored required tests passed.
A retained binary and consistent logs make the run reproducible; this is not a
cryptographic proof that a malicious runner executed code honestly.

Input fingerprints cover tracked source, embedded JSON/Swift/shell/build inputs,
fixtures and workflows. Data-only SI00 review/receipt paths are explicitly
excluded; a future compiler/test dependency on those paths requires expanding
the input definition. Git checkout and binary identities remain independent:
`lightr-cli/build.rs` embeds Git SHA/date, so a documentation commit can change a
rebuilt CLI's version bytes. Never relabel an older baseline binary as a newer
one merely because production Rust did not change. Every measurement keeps its
actual binary SHA, checkout and build context.

No `performance-budget.json` containing invented limits is provided. After the
correct reference exists, register a reviewer-approved canonical JSON budget
with candidate SHA, timestamp, fixture/profile/metric/unit, sample requirement
and ceiling, then call `validate_benchmarks` with the externally pinned digest.
It computes the comparison and rejects empty, malformed, wrong-unit, nonfinite,
late or over-budget data. Synthetic unit-test numbers are not performance data.

## Completion decision

SI-00 can deliver bootstrap evidence while remaining open for its formal
review and integration prerequisites. ADR-0020 and its concrete schema choices
are **Proposed**, not silently Accepted. G-FOUNDATION is not issued by these
scripts; SI-01 and SI-03 protocol implementation await accepted shared decisions
and reviewed integration. No main/integration merge, real-store adoption,
release or coding-agent session is implied by CI jobs or this delivery.

The package receipt in `../evidence/SI-00.md` records actual runs, test counts,
known runtime failures, limits and per-criterion disposition. A green baseline
cannot close the original macOS incident or the remediation campaign.
