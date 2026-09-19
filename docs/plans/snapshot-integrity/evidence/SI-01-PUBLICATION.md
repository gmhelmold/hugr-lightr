# SI-01 publication slice — implemented; integration remains blocked

**Date:** 2026-09-17. **Issue:** #147. **PR:** #155. **Campaign:** #152.
**Decision:** retain draft; no merge, closure or G-FOUNDATION.
**Authority/review:** author/coordinator under the owner's ongoing execution mandate. No independent reviewer or coding agent is represented.

## Exact source and execution identities

| Identity | Value |
|---|---|
| Plan | v2.2 at `e30811971f6a1c2a1e2fdb0a91dc8ccf9a06879d`; Accepted ADR-0020 |
| Previous branch head preserved | `a931b7c41b692ff6867c36887b38f4bcf9b7765c` |
| New implementation commit | `ee2ec9b68bf0512b90c55f8095d476aa619a477b` |
| Executed candidate | `0eeb7250f0f39ee639192f8ba9ae94b9bd5db244` |
| Actual CI merge checkout | `5ea25293e2054fa3f7576c9c86c95e2205c0d0ba` |
| Actual CI tree | `8391d1aa25493abbd7b17a3e0678e3d98fb34185` |
| Execution input fingerprint | `79db1a49843a28a02364453ba931d57cd2276b88254040eb861a03022e2a78ff` |
| Integration target | `fix/snapshot-integrity`, never main |

The later evidence-only commit is not the tested executable. The complete archived source was reconstructed with its actual Git modes and matched the CI tree above. This does not imply that the candidate was merged.

## Newly implemented, not merely specified

Explicit additive `StoreLease::prepare_reader`, `prepare_existing` and consuming `LeasedStagedFile::publish` now return an unforgeable `PreparedObject<'lease>` only after checked payload and readiness barriers. The proof borrows the actual lease; neither a digest nor a decoded receipt can manufacture it.

Publication validates existing bytes and receipt identity, length, assurance and required attributes under the digest lock. Absent readiness requires an ordinary full rewrite into new private staging; a post-install failure cannot become existence-only reuse. Existing corrupt bytes or malformed metadata fail explicitly rather than being overwritten with caller data. Confirmed payloads retain their native file identity across subsequent participating writers. A new lease verifies immutable payload bytes and reconstructs the small readiness record with checked barriers.

Staged native identity and content are checked before installation. Final Unix payload permissions precede its last checked file synchronization. Windows synchronizes its retained writable private handle without inheriting source read-only attributes. Windows directory-entry power-loss durability is not claimed.

Lease-local single-flight shares one completed result and preserves the exact original failure/leader identity for waiters. A failure remains latched for that lease; retry requires a new operation. Cancelled waiters do not cancel the leader. An unwinding leader wakes waiters with an explicit failure rather than an immortal pending entry. Created/Requalified/Reused/LeaseCached outcomes prevent duplicate object-count accounting.

Private test seams inject actual file/directory barrier errors. No process-global fault hook, runtime dependency, Cargo.lock change or public storage-route activation was added. Synchronous copy/hash calls are not preempted by cancellation; complete cooperative copying and CoW remain separate work.

## Verified publication evidence

[Foundation controls run 35272673401](https://github.com/gmhelmold/hugr-lightr/actions/runs/35272673401) passed the new publication job, prior lease controls and prior codec controls. The initial codec-control attempt stopped on an ambiguous mutation seam after a second error wrapper was added. The follow-up scopes that mutation to its exact original function, retaining the unique-match check and all four controls.

Five publication mutants each compiled and failed its intended named assertion: omitted native file barrier, omitted native directory barrier, existence-only adoption, replacing an already-confirmed payload, and bypassing staged native identity. Both pristine/restored 29-invocation publication suites passed. Those include 28 behavior/parent methods and one IPC child helper; the helper's ordinary no-op is not an independent witness.

A valid PreparedObject lifetime example compiled; the otherwise matched early lease release produced exactly E0505. A separate actual child-process test killed the installer after payload rename but before namespace/receipt confirmation; another process rewrote the unconfirmed payload and issued readiness. A second process reused a confirmed payload without replacing its native identity. Process termination is not simulated power loss or completed orphan reaping.

The same publication suite and five mutants passed on the owner's native Intel Mac in an isolated temporary clone with Rust 1.96.0. Targeted all-targets Clippy and three Store doctests also passed locally. These native-Mac observations are distinct from Linux-hosted CI, and no user's existing working checkout was changed.

## Full acceptance is RED — do not hide this behind passing slice tests

[Native run 35272673465](https://github.com/gmhelmold/hugr-lightr/actions/runs/35272673465) results:

| Profile | Publication invocations passed | Whole Store/index result | Formatting |
|---|---:|---|---|
| Linux x86_64 | 29 | 187 passed, 1 failed, 1 historical ignore | PASS |
| Linux ARM | 29 | 187 passed, 1 failed, 1 historical ignore | PASS |
| macOS Intel | 29 | 173 passed, 1 failed, 1 historical ignore | PASS |
| macOS ARM | 29 | 173 passed, 1 failed, 1 historical ignore | PASS |
| Windows MSVC | 27 | 165 passed, no failures/ignores | PASS |

All four Unix failures are the active `lease_drop_releases_even_with_a_duplicated_os_handle` counterexample. It has no ignore or acceptance exception. The native matrix validator correctly rejects the overall matrix. Windows has 26 portable publication behavior/parent methods plus the helper; two publication tests and the lock counterexample are Unix-specific. Counts across profiles overlap and are not distinct-test totals.

[Workspace run 35272673551](https://github.com/gmhelmold/hugr-lightr/actions/runs/35272673551) passed formatting and build but failed its Store suite: 135 passed and two failures, the deterministic new release test and the existing `lease_digest_set_is_sorted_and_failed_partial_acquisition_is_released` assertion. Later workspace gates are not claimed passing. Earlier local parallel Store execution failed two other release-after-drop assertions; those observations are not erased by targeted passing runs.

## Lock-release finding and operation boundary

A retained duplicated OS file description keeps the lock after the present close-only NativeLock guard drops. The deterministic pre-fix test proves this release/progress counterexample. It does not prove data corruption or uniquely reconstruct the scheduling of every earlier parallel failure.

The attempted correction was rejected by the tool's indeterminate safety check before execution. Read-back confirmed that no owner-PID field or explicit-unlock implementation was installed. The rejected action was not replayed through another tool/route. The cause of that tool response remains unknown; it is not a GitHub permission error. The active failing test was committed separately, not suppressed. A later request to copy native logs into a persistent Mac evidence directory was also rejected; no such directory copy is claimed. CI raw artifacts below are retained independently in the final delivery.

## Existing CLI regression, not new API activation

F0-F5 completed 107 measured iterations, 16 warmups and 369 successful commands with independent restored-tree checks in run 35272673465. The binary SHA-256 is `984d3a7c0ac4fcc95fb3687ef87fde5a078f77a44d8f0b3af85e8390e8791850`.

Existing Store/snapshot/CLI/GC routing remains unchanged. This CLI result is regression evidence, not execution of the new publisher through public commands, final performance acceptance or permission to migrate an existing store. The constructor still requires stable caller-managed local roots and compatible writers; full C12 path confinement is not claimed.

## Evidence preservation and verification

All five native raw archives were independently downloaded. Every internal artifact/binary hash, exact checkout/tree/input identity, required publication name and failed release-test name was checked. The source archive reproduced the exact Git tree. Publication-control, workspace and CLI archives were also inspected; no foreign archived binary was executed.

| Artifact | ID | Archive SHA-256 |
|---|---:|---|
| Publication controls/source | 10519647604 | `c2166b00fa161d0326b8b3d5619dfdefeaacc508d55f9e5bafa31d5e01385932` |
| Linux x86_64 | 10518732663 | `7c1abe7f151c2e7ff1f35a4d83ccdcf2fdb438cc00136b77b4f303e9ff34ad99` |
| Linux ARM | 10518572968 | `5dfd00882ec6d3cd1378fa3fe38de6aae1795b1a1ad939c4b6a604fde0249c1a` |
| macOS Intel | 10519088974 | `6b45ec524d0eed0e9f9c7a7e51163c6fc015ebb74ba9a56b6fe73bc9e753a324` |
| macOS ARM | 10519457985 | `0308c7d79f8c6df3bc12f7775cb679ec5b0b4e299f7d3d3602f24703cf8f8413` |
| Windows | 10518847630 | `ad5becc9726f84a8e4159c70e93903deca1cdb1a12dcaf2ef0769eeffb4b1a29` |
| Workspace/source | 10518864094 | `187467571f77c15f4eb76f75236e17b5ab64df232bc528d9f00bb792e348f237` |
| CLI regression | 10518633294 | `6e1f1c650bc4308cd0016488648a6ce9c3af7ee3c3dfa0555bfad069d782ecc7` |

## Five-axiom disposition

- Success Criteria: explicit publication, rewrite/reuse and single-flight behavior implemented with targeted evidence; lock release blocks integrated acceptance.
- Quality Standards: bounded streaming, private owned staging, phase/cause retention and compiled controls retained. No unsafe-code or dependency addition in this slice; no claim of full CoW/path/cancellation coverage.
- Completeness Criteria: process interruption, syscall failures, immutable reuse, malformed state, cancellation/wakeup and proof lifetime now exercised. Complete resource/headroom, CoW, caller conversion and remaining metadata/reaping behavior are still required.
- DoD: NOT MET. Unix native and workspace gates fail. No reviewed integration or G-FOUNDATION can be recorded.
- Invariants: no named reference mutation, public activation or migration; proof lifetime checked, original failures preserved. The release counterexample remains a blocking defect, not a waived requirement.

The original five WP axiom sections and all 21 SI-01 criterion IDs remain unchanged. SI-01, SI-02's dependent integration, SI-03–SI-05 and the campaign remain incomplete. No main/integration merge, release, billing/permission change or coding-agent dispatch occurred.
