# SI-01 first slice — pure primitives verified; full work package open

**Date:** 2026-09-17. **Campaign:** #152. **WP:** #147. **Delivery:** draft PR #155.
**Authorization:** the owner explicitly requested SI-01 execution.
**Decision:** retain and review this executable partial delivery. Do not close
SI-01 or establish G-FOUNDATION from these results. No merge was performed.

## Exact identities

| Identity | Value |
|---|---|
| Contract | v2.2 at `e30811971f6a1c2a1e2fdb0a91dc8ccf9a06879d`; Accepted ADR-0020 |
| Accepted SI-00 base | `0c48886d2c82e671c33466ca0d5ff1640f90985d` |
| First pure commit | `048886701758c3b636dffef1e6af46f4ac2d386e` |
| Final executed source head | `99ae6cba151c2702246a9d79f3f429d236bc95c4` |
| Source head tree | `3f26d92f39560bf6da47ab839e6168bbe64ea64a` |
| Actual tested PR merge | `4c8fbc4b2386177060e95b0ee66e9ec28203fd61` |
| Tested merge parents | `9ab0eb764d8f637b16c139199f3bbf39cc4f74f6`, `99ae6cba151c2702246a9d79f3f429d236bc95c4` |
| Tested merge tree | `9c76793c0e553cd5538ec8051bd5bbc2195434df` |
| Execution-input fingerprint | `7b913a73e95b6448779aae27e95433d5235437cef9290e712fd886b46ea09d2e` |

The local source/index reconstructed from the accepted artifact matched its
Git tree; both published source trees matched the locally prepared trees.
The tested merge archive was separately reconstructed as a Git tree and matched
the API tree above. Every input-manifest hash from all five native archives
matched that source. The later commit containing THIS receipt is not the tested
executable identity. PR merge checkout is a test artifact, not an integration
merge performed by this delivery.

## Implemented scope

- `Digest::of_reader`: existing BLAKE3 implementation, bounded 64 KiB buffer,
  checked actual length, interrupted-read retry and original I/O propagation.
  Existing `of_file`, Store methods and callers are unchanged.
- `Readiness`: compiled Rust codec for ADR-0020's exact readiness frame; body
  limit 4,096 bytes, header validation before body allocation/read, checksum,
  strict typed JSON, canonical lower-case digest, duplicate/unknown-field and
  trailing-byte rejection. Both frozen SI-00 readiness vectors encode exactly.
- `PublicationFailure`: operation ID, phase, outcome and original I/O cause
  survive the existing LightrError wrapper and remain available by downcast.
- Five new Rust test methods, native on every required profile, and four exact
  compiling negative controls in a disposable source archive.

This is actual additive Rust implementation, not documentation alone. It has
NO receipt writer, Preparation factory, lease/proof implementation, new CAS
routing, ref transaction, cache lock or garbage-collector modification.
`Readiness::new/read` return data, not PreparedObject or filesystem assurance.
`Assurance::native` chooses the expected OS profile, not volume qualification.
**production_protocol_enabled=false.** No dependency/Cargo.lock change, unsafe
code or new skip was introduced. The streaming method is in lightr-core to reuse
its existing BLAKE3 dependency rather than add a new storage dependency.

## Publication boundary

A separate create-tree call containing the file-preparation modules was blocked
by the tool because its safety status could not be determined. That publication
was not retried through a different route. It is not included in these commits,
in the actual PR source tree, or in the published-source delivery patch.
Unreferenced Git trees/local prototypes are not delivered implementation or
runtime evidence. This is a tool-level block, not a new GitHub 403 or a demand
for the owner to toggle repository permissions.

The pure codec/error/hash work is independently publishable and does not perform
the blocked file-preparation action. The broader SI-01 remains uncompleted.

## Executed gates

| Run | Result on the exact candidate above |
|---|---|
| [35256952475 — workspace](https://github.com/gmhelmold/hugr-lightr/actions/runs/35256952475) | Formatting, workspace build, workspace tests, all-targets Clippy with warnings denied and the three existing compiled integration controls passed |
| [35256952473 — native/bootstrap](https://github.com/gmhelmold/hugr-lightr/actions/runs/35256952473) | Five native profiles, complete artifact matrix and full F0-F5 CLI sequence passed |
| [35256952715 — pure controls](https://github.com/gmhelmold/hugr-lightr/actions/runs/35256952715) | Four mutants compiled and failed their exact named tests; pristine suite passed before and after |

Workspace logs contain **1,665 passed, zero failed, one historical ignore and
zero filtered**, across 35 suite summaries. These overlap the native counts.
Do not add those counts as unique tests or call the baseline a new-protocol
qualification. The first unformatted pure commit failed formatting; its exact
rustfmt suggestion was applied, and the final candidate passed formatting.

| Native storage/index profile | Passed executions | Historical ignores | Formatting exit |
|---|---:|---:|---:|
| Linux x86_64 | 134 | 1 | 0 |
| Linux aarch64 | 134 | 1 | 0 |
| macOS Intel | 120 | 1 | 0 |
| macOS Apple Silicon | 120 | 1 | 0 |
| Windows x86_64 MSVC | 116 | 0 | 0 |

**624 native executions**, including the five new methods on each platform.
There are five new distinct methods, not 624 new tests. The four ignores are
preexisting and retain their prior qualification boundaries. All five archives
were independently downloaded, archive-hashed, internally rehashed and passed
the canonical native validator offline with independently supplied expected
checkout, parents, tree, inputs and profile. No foreign executable was run in
the authoring container; no local Rust compiler was available.

### Causal observations, not just a mutant count

| Mutation | Exact observation after successful compilation |
|---|---|
| Disable checksum comparison | `ready_matches_frozen_si00_golden_bytes` failed its damaged-checksum assertion |
| Remove effective body-length bound | `oversized_header_is_rejected_before_reading_body` detected an attempted overlength body read |
| Allow unknown JSON fields | `readiness_rejects_resigned_semantic_corruption` detected acceptance of a correctly checksummed extra field |
| Drop the structured error payload in the legacy wrapper | `structured_failure_survives_legacy_wrapper_and_source_chain` could no longer recover the payload |

Each returned test exit 101 with exactly one named failed test; build failures,
timeouts and unrelated failures do not qualify. The source was restored between
cases and the five-method suite passed again at the end. The copied vectors
were compared to the frozen SI-00 authority, not regenerated by the candidate.

### Real CLI regression surface

The unchanged public pipeline completed **107 measured iterations and 16
warmups**, totaling **369 successful new/warm snapshot and hydrate commands**
with independent restored-tree comparisons. All F0-F5 sample sets completed.
The retained CLI binary SHA-256 is
`3dbc8e9e09df0a6cdfe7f7665f0fc832d06133efabca4beb3cde230e36ad2b69`.
Four traced minimal cases also roundtripped with unchanged sources, and the
existing bounded duplicate-content diagnostic completed its 20 attempts.
These are regression observations on the current legacy routing, not evidence
that the absent SI-01 preparation or SI-03 recovery implementations work.
No final campaign budget or historical incident attribution is fabricated.

## Author/coordinator self-review

Read the whole delivered delta against accepted ADR-0020 and the current Store
surfaces. Confirmed bounded allocation ordering, strict decode behavior, exact
wire compatibility, error-source ownership and absence of any Store write path.
Rechecked that selecting a profile cannot be confused with a PreparedObject.
The proof is limited: codec tests cannot establish durable publication, object
requalification, lease lifetime or concurrent collection. Those remain below.
This is explicitly the author's self-review, not independent human/agent review.

## SI-01 five-axiom accounting — no full-package criterion waived

| Criteria | Actual boundary |
|---|---|
| SC01.1, SC01.2 | Not completed: preparation/verification routing and failure/retry reuse are absent. |
| SC01.3 | Partial: wire/error/hash primitives exist; native flush, leases and concurrent publication are absent. |
| SC01.4 | Not completed: no staging/preparation path was delivered. No OCI named pointer is written. |
| QS01.1 | Streaming and contextual errors apply to this slice; RAII/publication obligations remain open. |
| QS01.2 | No unsafe, global failure hook or activation switch; later platform/lock code still needs review. |
| QS01.3, QS01.4 | Not completed: clone/copy, writable flush, scratch/resource guarantees are not exercised by this slice. |
| CC01.1 | Not completed: C03 call-site integration remains absent. |
| CC01.2 | Partial: malformed readiness frames, length/digest syntax and reader failures tested; filesystem/concurrency cases remain. |
| CC01.3 | Formatting fixed and rerun; historical incidents preserved, not newly diagnosed here. |
| CC01.4, CC01.5 | Not completed: dedup/wake-up/cancellation, topology and resource-domain obligations remain. |
| DOD01.1 | This slice passes its native/workspace/causal gates; full required SI-01 witnesses do not yet exist. |
| DOD01.2 | Explicit pure-slice self-review only; no lease-lifetime review claim. |
| DOD01.3, DOD01.4 | Not completed: no accepted/integrated foundation or scratch/phase handoff. |
| INV01.1 | No prepared proof is issued; receipt data must not be treated as one. |
| INV01.2 | Error information preserved; no references or CAS objects are modified by this slice. |
| INV01.3 | Existing failures are not suppressed, but future platform propagation remains to implement. |
| INV01.4 | No readiness publication, resource recovery or pending metadata manipulation is performed. |

All 21 criterion IDs and five axiom groups remain in force in #147. No agent,
main/integration merge, real-store migration or new protocol was started. Keep
#147 open and PR #155 draft; SI-02 production integration is not released.

## Raw evidence retention

| Artifact ID | Kind | Archive SHA-256 |
|---|---|---|
| 10513471836 | Pure controls and exact source | `efaad5a0fce45c31d8ba9179fdf7680e7d701a249234e9a7529e49f0c7580e71` |
| 10513607313 | Workspace integration and source | `b707698d943e2ed4cab19af9fdee20ec8b2d08ea349fba6adc46debdee828d85` |
| 10512651958 | Native matrix verdict | `4da07858fba7cab8355be983a2b7ad7f1346b88a296331bd5242550659a00b5c` |
| 10512462033 | Windows native | `dca510a5a204262c1f53d0e2d941cd361e4cd628141639879ccac6edae766ff5` |
| 10512456963 | Linux x86_64 native | `712315a9904b6e12ea1f2c443ffc63714793b7ce37bacff977ee421cbd8569fe` |
| 10512821822 | Linux aarch64 native | `95deae2b0fe564106e6ba28b7c9e3ec6b0b5b0fdb7729e57015fa063e2ee8da7` |
| 10512881759 | macOS Intel native | `dbfe127806b26fb637d5ab6de348e5addc2dc58bf2b9f6a11152fd38f34d4563` |
| 10513222232 | macOS ARM native | `58f68d02978c08358196e0e19109c88f3e6c01a0437223ca6c123c20d0fa8031` |
| 10512642426 | CLI baseline/diagnostics/binary | `a996f8e2380c04f9857dd41d2507b5d2f0648dde9244d9e30acf3ea1bdcbb93a` |

GitHub artifacts expire on 2026-10-17. The accompanying durable delivery retains
these raw archives and a verifier; archive hashes alone do not replace bytes.
Checks are source/evidence conformance, not independent signed attestation.
