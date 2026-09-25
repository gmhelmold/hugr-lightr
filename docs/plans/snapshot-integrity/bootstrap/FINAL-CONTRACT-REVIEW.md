# SI-00 final contract and compatibility review

Date: 2026-09-17. Reviewer: ChatGPT in the author/coordinator role.
Authority: the owner's explicit `OK pode executar`, `pode seguir ate fechar`
and `siga` instructions in the current campaign. This is delegated technical
acceptance, not a fabricated independent reviewer or a claim that the owner
separately examined every byte. It waives no evidence, integration or release gate.

## Decision and exact scope

Accept ADR-0020 as the design input for SI-01 through SI-05, with the concrete
interpretations below. The immutable vectors and the documented protocol are
accepted together, not as interchangeable substitutes for runtime evidence.
SI-00 closure still requires the final acceptance receipt and reviewed integration.
No production codec, readiness receipt, pending descriptor or new history
record exists merely because this review accepts their specification.

The existing-protocol repairs under A07 are separate: owned transient directories,
literal path/target preservation and formatting. They do not establish
G-FOUNDATION or G-ACTIVATION and do not close SI-01 or SI-02.

## Compatibility matrix

| Existing contract | Accepted disposition |
|---|---|
| ADR-0009 payload CAS / LMF1 | Keep object addressing, payload and manifest encodings. Bounded receipt/history/pending JSON is an explicit exception to JSON-only-at-borders for these small control records, not a JSON manifest or hot payload format. |
| ADR-0004 current RefRecord and grammar | Preserve encoding, grammar and key derivation. Enforce length/full-consumption before accepting or encoding. Reserved zero-name tags cannot identify a valid old name. |
| ADR-0010 stat index | Remains an optional cache/status optimization; stat equality cannot establish the identity of a fresh published capture. No memoization redesign is authorized. |
| ADR-0017 portability | Preserve native engine boundaries. Exact materialization uses preserve-or-reject links, not the old silent copy/omission fallback. Five native storage profiles are mandatory evidence, not VM/engine qualification. |
| Existing public Result types | Preserve the Store wrappers and ordinary successful return types. Structured publication failure is carried by an error payload, not by converting uncertain state into Ok. See the error contract below. |
| Legacy history and image metadata | Decode and retain conservatively; never certify a historical publication or tuple association from matching hashes alone. Navigation only inside a verified suffix, including an explicitly adopted present-day baseline. |
| Mixed versions / moves | No concurrent old writers, automatic downgrade or cross-filesystem transfer of assurance. Offline movement needs target-side qualification; current binary cannot make an old binary honor new metadata. |
| Existing CLI / Docker aliases | Preserve existing successful output and usage/child-exit conventions. Maintenance is an additive Lightr-only surface; no Docker alias begins mutating the store implicitly. |

## Error and support surface frozen for SI-03

Use `PublicationOutcome` and `PublicationFailure` with operation ID, stage,
original OS cause and optional affected relative path. Keep an original
`std::io::Error` as a source, including its raw OS code; do not retain only a
formatted string. Preserve existing `LightrError::Io` compatibility by boxing
this structured error in its `std::io::Error` payload when an old wrapper is
used. An accessor may downcast that payload; nested source inspection must
recover the original OS error. New internal APIs use the structured result
without a lossy roundtrip. Exact type placement must not introduce a crate cycle.

`Published` is ordinary exit 0. `NotPublished`, `CommitUncertain`,
`CommittedMaintenanceFailure`, `RecoveryRequired`, `RecoveryBlockedResources`,
`LegacyHistoryUnverified` and unsupported capability/representation/topology
failures are exit 1, with the outcome and original cause in the existing
`lightr: ...` error line. Ref grammar/not-found usage paths retain exit 2;
child command status retains its existing propagation. Do not route Store I/O
through `die_internal`, which is a usage-class helper. `AttributionUnknown`
is a separate inspection field, not a failure or automatic replay instruction.
These additions need error-mapping tests for CLI, MCP and library wrappers.

The selected additive maintenance surface is under `lightr system`:

| Future command | Effect / output |
|---|---|
| `store-inspect [--name REF] [--json]` | Read only: schema=1, assurance profile, tuple/provenance, pending phase, request attribution and resource blockers. Does not call mutable Store initialization or create absent roots. |
| `store-recover --name REF [--json]` | Explicit phase-based recovery under EX Store lease; no operation replay. An absent journal returns no-op state, not ownership of a past request. |
| `store-reap-scratch [--json]` | Reap only registered owned scratch under the matching resource guard; report collected/skipped/failed. No CAS sweep, name retirement or guessed ownership. |
| `store-adopt --name REF --root DIGEST --config DIGEST_OR_none --image-manifest DIGEST_OR_none [--json]` | Explicit present-day baseline for an existing legacy name. Require current root match, every dependency verified and an explicit choice for both metadata pointers. A stale root fails; association is owner-selected now, never retroactive proof. |

These commands are a frozen implementation contract, **not available commands**
in the SI-00 binary. They are wired only in SI-03's final coordinated activation.
Inspect may report an unconfigured store without creating it. Read-only inspection
of existing protocol state opens already established locks; if a necessary
identity/lock cannot be established without mutation, it reports that limitation.
Recovery and adoption use normal configured-root topology preflight, not user
supplied metadata filesystem paths. There is no automatic whole-store migration,
extra request ledger, background daemon or emergency-space reserve.

## Wire validity versus legal transaction transitions

The ten frozen vectors test byte framing, field binding and permitted record
representations. Runtime publication additionally checks the operation's live
preconditions under its locks. For example, a structurally valid first-publication
record with a parent does not authorize inventing a previous destination; actual
snapshot/tag/build/undo publication derives the destination parent under the
ref lock. A metadata-only change can keep the exact current RefRecord while
changing other tuple components; equality of root/record does not erase a tuple
change. Adoption may preserve current record bytes, but creates a new, dated
provenance boundary in history. The existing before==after adoption rule remains.

Frozen framing: domain bytes concatenated with tag/length/body, then ordinary
BLAKE3, not BLAKE3 derive-key mode. Preserve exact accepted frame bytes for
idempotent recovery. Reject unknown tags/versions, duplicate or missing fields,
noncanonical base64/digests, overlength or trailing data before unbounded work.
The Python oracle is not a production streaming decoder; Rust allocation,
bounded reads, all failure injection and crash transitions still require E01-E22.
History retention after UNTAG is tied to the exact prior lifetime. The descriptor's
retirement count/digest must be checked against an enumerated, name-bound history
set; it never authorizes recursive deletion of a user-selected path.

## Resource, caller and platform ratification

The following source-review records are accepted as the **baseline conversion
inventory**: CONTRACTS.md, caller-map.md, CALLER-REVIEW-ADDENDUM.md and the
regenerable raw occurrence inventory. The 73 direct sites / 35 files were
resolved by source reading into 70 relevant bindings and three unrelated
network/CRI homonyms. Their claim is not compiler-resolved whole-program proof.
A07 changes only CAS private staging and scan conversion; no new root family or
public caller is added. New private helpers/tests must appear in the regenerated
inventory, without relabeling test occurrences as production consumers.

All named mutation/read groups are owned: snapshot/build/import/pull/load/tag,
config-only reads and writes, history/diff/undo/bisect/MCP, run preparation,
materializers, views/CRI adapters, AC writers/readers and collector raw joins.
Raw32 build-manifest AC roots and LRR1 output/error roots are distinct. Presentation
`list_refs` cannot supply authoritative GC enumeration. Bench direct namespace
resets remain an SI-03 conversion obligation, not proof from an old green benchmark.
Network, healthcheck, volume and runtime process-state homonyms remain outside
CAS scratch cleanup. Baseline inventory is sufficient to start inert primitives;
actual converted-code/cfg completeness is rechecked at G-ACTIVATION and SI-04.

Accept the existing domain/order map: Store SH/EX lease; shared-index EX resource
leaf section; sorted ref locks; sorted digest locks. Cache locks are released
before key locks; standalone cache users cannot call into Store while locked.
Aliases converge by native filesystem identity, not spelling. Unknown identity
or overlapping authoritative roots is rejected. Initial anchor/lock establishment
must be serialized so replacing an inode cannot split the domain.

Five native profiles have concrete storage smoke/artifact capability. Per-profile
clone/link/permission support is taken from the measured receipts, not runner
names: fallback copying remains mandatory when native clone is unavailable.
The APFS non-UTF8 fixture is rejected by the OS at creation; the conversion
policy is tested independently. A native creation refusal is not a passing
Lightr scan execution. Windows file synchronization is not directory power-loss
certification. Hardware durability and unsupported mounts remain outside scope.

Resource recovery is progress conditional on capacity: preserve pending, acquire
the appropriate guard, collect only proven scratch, otherwise free unrelated
space or increase quota, then retry recovery. Required headroom is conservatively
the sum of complete payloads needing rewrite, simultaneous bounded metadata
staging and measured filesystem allocation/inode overhead. No platform-independent
free-space minimum is promised. The 32 MiB probe establishes ENOSPC test capability;
quota/inode limits and interruption of actual runtime recovery remain E18/SI-05.
This is a mapped downstream test obligation, not an unavailable mandatory smoke
profile hidden by SI-00 closure.

## Evidence and handoff boundary

Accept E01-E22's registered owner/oracle recipes and C10's fixture-specific
sampling/budget-registration process. F6 cannot use legacy incorrect history as
a semantic reference. No numerical final-candidate budget is invented by this
review; it must be preregistered after the matching correct reference exists.
Historical ENOENT occurrences remain attributed only to the extent of their
actual traces; a controlled collision or later success does not resolve every
past incident.

The final SI-00 receipt must bind one exact source/test/workflow candidate, native
matrix, immutable schema checks, full F0-F5, formatting and workspace integration
results, including compiled negative controls. It must enumerate the five
axioms and state whether reviewed integration occurred. Only that completed
handoff makes SI-01 READY; closing an issue does not enable a protocol. No external
review, coding subagent, release, main merge or real-data migration is claimed.
