# Sprint 02: Runtime, OCI, Image, Dockerfile, and BuildKit Parity

## Status

Planning contract. Implementation claims require differential evidence defined
here. S2 follows the frozen Docker 28.3.2 corpus from Sprint 01.

## Outcome

S2 removes silent semantic loss from Dockerfile, BuildKit, OCI, and image paths,
then proves supported behavior against Docker 28.3.2 with raw evidence. It does
not claim full Docker or BuildKit parity. A feature is either measured supported
or explicitly fail-closed.

## Product Bar

Lightr remains daemonless and imageless. No S2 feature may add an idle daemon,
hidden image store, or provision before an Action Cache lookup.

The owner targets are 10x efficiency, 100x less weight, and 300x faster. These
are measurement gates, not present-tense claims. A factor is reported only when
both Docker and Lightr commands succeed against identical fixture bytes on named
hardware. Failed or incomparable runs report their reason, never a factor.

## Compatibility Contract

### Dockerfile

Existing supported instructions retain behavior. S2 makes parser/executor
boundaries strict:

- unknown instruction flags fail before execution;
- `RUN --mount` never becomes shell text silently;
- ONBUILD behavior is either executed on a derived base or rejected before build;
- a parsed ownership, platform, source-stage, or config field changes execution
  and action identity, or fails closed;
- cache identity includes every input that can change output.

### BuildKit

`--cache-from` cannot remain a no-op. Until verified source identity and cache
lookup semantics exist, it fails with a stable unsupported error.

`--secret`, `--ssh`, and `RUN --mount=type=cache|secret|ssh` remain unsupported
until their contracts exist. Secret bytes must not enter command logs, output
trees, action keys, or raw benchmark records. Later support requires an explicit
input reference, authorization boundary, cleanup proof, and cache-key contract.

Attestations, SBOM/provenance, exporters, alternate frontends, emulation, and
manifest-list output remain explicit unsupported surfaces in S2 unless a later
accepted work package upgrades one with end-to-end evidence.

### OCI and Image

OCI config is content-critical. Bad, missing, unreadable, or digest-mismatched
config aborts import/pull/load before publishing a ref. Archive traversal input
is rejected, not skipped. Platform selection either selects a verified matching
manifest or fails closed; no first-manifest fallback.

## Differential Evidence

Every S2 supported scenario records:

- frozen spec digest and source commit;
- Docker client, server, and API versions; Docker must equal 28.3.2;
- Lightr binary digest and version;
- fixture tree digest, command digest, stdout/stderr digests, exit code, elapsed
  time, host OS, architecture, kernel, and named hardware identity;
- cold, warm, and cache-invalidating mode where applicable;
- assertions proving output equivalence, not only success.

Median and range use the same sample count after one warmup. `factor = docker /
lightr` is valid only for paired successful samples from same hardware and
fixture digest.

## Work Packages

| WP | Scope | Owner files | Depends |
|---|---|---|---|
| S2-0 | Corpus classification, fixtures, acceptance contract | `benchmarks/**`, this document | -- |
| S2-1 | Dockerfile strict parsing and ONBUILD semantics | `crates/lightr-build/src/build/parse/**`, `exec_instr_*`, sibling tests | S2-0 |
| S2-2 | BuildKit semantic firewall and supported cache contract | build CLI/buildkit modules, sibling tests | S2-0 |
| S2-3 | OCI config, path, platform, and ref-publication integrity | `crates/lightr-oci/src/oci/**`, OCI tests | S2-0 |
| S2-4 | Image/runtime configuration propagation differential tests | CLI run/build/image acceptance tests | S2-1, S2-3 |
| S2-5 | Docker/Lightr performance harness and raw artifact validation | bench handler/tests, benchmark scripts | S2-1, S2-2, S2-3 |
| S2-6 | CI, evidence ledger, independent review | workflow/CI only after S2-5 | S2-4, S2-5 |

No work package shares an owned implementation file. Shared corpus, CI, and
ledger files are lead-owned integration points.

## Gates

1. Mutation tests prove every parser or validator rejection is reachable.
2. No unsupported feature is accepted then ignored.
3. No invalid OCI input publishes a partial ref.
4. Differential test output uses same fixture digest and asserts output behavior.
5. Performance records are raw, version-pinned, named-hardware evidence.
6. Regression thresholds never weaken without explicit owner waiver.
7. Full CI, benchmark smoke, independent cold review, and mutation probes pass
   before integration.

## Delivery Order

`S2-0 -> (S2-1, S2-2, S2-3) -> S2-4 -> S2-5 -> S2-6`.

S2-1, S2-2, and S2-3 may run in parallel only after S2-0 is committed. Every
agent works one isolated branch and stops after its PR is green. The lead owns
contracts, integration, evidence review, and merge.
