# SI-01 — native leases and borrow-bound staging, verified slice

**Date:** 2026-09-17. **Campaign:** #152. **Work package:** #147. **PR:** #155.
**Decision:** retain the draft PR; this slice is verified, not full SI-01 acceptance.
**Authority/reviewer:** ChatGPT as author/coordinator under the owner's ongoing execution instruction. No independent human/agent review is represented.
**Contract:** execution specification v2.2 at `e30811971f6a1c2a1e2fdb0a91dc8ccf9a06879d`, Accepted design ADR-0020.

## Exact execution identity

| Identity | Value |
|---|---|
| Previous source head | `7473f241f5437db4122e3bbfc3eec31ea1b1cd7a` |
| Current tested source head | `7fbad90ea82b5c62f360a1431ad46332c7f788fd` |
| Actual tested PR merge | `c1a51ae8dff63156832ef81df0dd62fe5f6feca3` |
| Merge parents | `9ab0eb764d8f637b16c139199f3bbf39cc4f74f6`, `7fbad90ea82b5c62f360a1431ad46332c7f788fd` |
| Tested tree | `478db79575fbd2937159a74f0ff0ea404c652e42` |
| Source-head tree | `d4b011fb0e903136c227b443ba303e397d8c3499` |
| Execution-input fingerprint | `7e4af89178805c5332a0864323776711d2fb1faaa73b04ec87165baebfc7bbb3` |

The synthetic merge is a CI checkout, NOT an integration merge. This later evidence file does not change executable inputs and must not replace the tested identity above. SI-00's closure file is present in the merge tree; it is not silently removed from the integration branch.

## Implemented delta

Six Rust files add native locking, three resource domains and staged-file borrowing. A scoped workflow extension, a causal-control script and a named-test-policy extension complete the nine-file executable delta. No Cargo/dependency, existing Store route, CAS install, readiness writer, reference or GC algorithm changed.

`StoreLocks` opens an existing managed root and obtains SH/EX leases on the existing `.gc.lock`. Each lease owns one private native file handle; borrowing a worker opens no additional global handle. Native IDs, not path strings, establish domain equality. `File::try_lock[_shared]` is checked, with finite deadline/cancellation support. Stable lock files are never truncated or deleted by these primitives. Native tests verify interoperability with the existing GC guards.

`LeaseWorker` acquires a sorted, deduplicated set of digest locks, releasing a partially acquired set on error. Distinct digests can progress separately. Its mutable borrow prevents overlapping cache/digest sections within that worker. Cross-worker nesting remains a contract obligation, not a claimed whole-program static proof. No lock upgrade or exclusive-to-shared conversion is provided.

`CacheLocks` locks the cache root independently of any Store. Standalone cache clients and Store workers use the same resource domain. Store A's EX lease cannot authorize shared-cache cleanup or exclude Store B's cache writer. Existing index callers are NOT converted in this slice.

`LeasedStagedFile<'lease>` wraps the verified staging primitive, chooses the managed staging family internally, and borrows the actual Store lease. It does not expose its write handle or create a PreparedObject. Its stage closes/cleans before the retained parent and before the borrowed lease can be released.

| Namespace | Guard / purpose |
|---|---|
| `<store>/.gc.lock` | Same existing Store SH/EX exclusion plane |
| `<store>/.si01-digest-locks/<hex-digest>` | Stable per-digest EX files; not scratch |
| `<store>/.si01-staging/` | Owned staging allocations; lifetime borrows Store SH lease |
| `<cache>/.si01-cache.lock` | Stable shared-cache EX resource lock; not Store-scoped scratch |

## Final executed gates

| Run | Observed result |
|---|---|
| [35266943376 — workspace](https://github.com/gmhelmold/hugr-lightr/actions/runs/35266943376) | Formatting, full workspace build, tests, all-targets Clippy with denied warnings, existing three legacy causal controls: PASS |
| [35266943348 — native/bootstrap](https://github.com/gmhelmold/hugr-lightr/actions/runs/35266943348) | Five native profiles, required named tests, complete artifact matrix, full F0-F5 CLI regression: PASS |
| [35266943301 — SI-01 controls](https://github.com/gmhelmold/hugr-lightr/actions/runs/35266943301) | Five new compiled lease mutants, paired rustc lifetime witness, existing four codec controls: PASS |
| [35266943504 — contract](https://github.com/gmhelmold/hugr-lightr/actions/runs/35266943504) | Immutable contract workflow: PASS; not a new readiness-publication experiment |

Workspace logs contain **1,691 passed, zero failed, one historical ignore, zero filtered**, across 35 summaries. This includes two new doctests: legal release order and compile-fail early release. The separate rustc experiment checks the exact diagnostic rather than accepting any compile failure.

| Native Store/index profile | Passing executions | Historical ignores | Formatting exit |
|---|---:|---:|---:|
| Linux x86_64 / ext4 | 158 | 1 | 0 |
| Linux aarch64 / ext4 | 158 | 1 | 0 |
| macOS Intel / APFS | 144 | 1 | 0 |
| macOS ARM / APFS | 144 | 1 | 0 |
| Windows x86_64 MSVC / NTFS | 138 | 0 | 0 |

Total **742 native executions**, not unique/new tests and not additive to the workspace total. The new Rust test code has **16 behavioral methods** on Unix, 14 portable to Windows, plus **one IPC child helper**. The helper's ordinary no-op invocation is included by libtest in totals but is NOT an independent concurrency witness. Its behavior is exercised by three parent-process tests. Fourteen portable parent/behavior test names are required explicitly by the native policy; the helper is not used as a required witness. No new skip was added.

Tests exercise same-process exclusion, existing-guard interoperability, cancellation after observed native contention, deadlines, shared borrowing across scoped threads, aliases, replacement detection, cross-Store cache exclusion, partial acquisition cleanup, distinct-digest progress, stage lifetime and missing roots. Child-process tests use acquisition handshakes and actual nonblocking probes, including forced termination of a holder. Watchdogs bound failure; elapsed time is not used as proof of contention.

The unchanged public CLI paths completed **107 measured iterations + 16 warmups, 369 successful commands**, with independent restored-tree comparisons. Retained CLI SHA-256: `31eef6c3e93621a1c29cdeadc7cc32bfa7a5553e59ea47090196b7ad21e73bec`. This is legacy-route regression evidence, not execution of the new lease APIs through the CLI or final campaign performance acceptance. F6 and final preregistered budgets remain assigned to their planned stages.

The existing **42 Python SI-00 support methods** also passed locally. No Rust compiler or foreign native binary was run in the authoring container.

## Causal controls and harness repairs

Each new mutant built successfully, then exited 101 with one exact named failed test and its required assertion. Pristine and restored thirteen-test lease suites passed. The mutants independently remove native exclusion, bypass root identity, change cache EX to SH, remove digest sorting, and leak a partially acquired lock set. Compile failure, timeout and unrelated test failure are rejected.

A separately compiled valid lifetime example exited zero. The otherwise matched early-release example exited one with the sole coded error **E0505**, `cannot move out of lease because it is borrowed`. E0463 or a missing dependency cannot satisfy this check.

Three earlier failures are retained, not erased: the initial pinned formatter failure; a sorting-mutant test that mixed sorting and nonadjacent deduplication and therefore reached the wrong assertion; and an E0463 verifier error caused by looking for dependencies only beside Cargo's top-level rlib. The fixes respectively apply rustfmt, assert sorting before separately checking nonadjacent duplicates, and derive dependency search directories from actual compiler artifact JSON. The causal acceptance criteria were not relaxed.

## Author/coordinator review and explicit limits

All six Rust files and the three support changes were inspected. The Windows FFI obtains identity from a live owned File, checks the API's zero failure result before reading MaybeUninit, and retains the file handle. Normal drop releases locks without raw-handle exposure. Unix no-follow/type checks and Windows reparse/type checks are present for internal entries. Native behavior was tested on the five named profiles.

**Precondition:** these constructors operate on stable, already-managed local directories. They are NOT the complete C12 public topology validator, an openat-style confinement layer against arbitrary simultaneous ancestor replacement, or qualification for network/mixed mounts. Detecting a root already replaced before acquisition does not prove every possible path race is excluded. Full public path validation and coordinated caller conversion remain required before activation.

Cancellation here governs lock acquisition, not asynchronous interruption of a running filesystem call or the full streaming-copy pipeline. Cross-worker/ref-lock ordering, lease-local preparation deduplication/wakeup, full readiness/requalification, CoW/resource/flush-fault coverage, and the guarded scratch reaper remain outstanding. Killing a test lock holder proves native-lock release, not journal recovery or complete orphan reclamation. No power-loss assurance follows from a lock test.

No blocking defect was identified within this bounded, unactivated slice after the harness fixes. That is author self-review, not independent approval or acceptance of the entire work package.

## Five-axiom accounting and continuation

| Mandatory group | Evidence added / remaining boundary |
|---|---|
| Success Criteria | SC01.3 gains actual single-lease/key/cache exclusion; SC01.4 gains a fixed staged namespace with borrowed lifetime. SC01.1 remains partial until public integration; SC01.2 final-install/requalification is not implemented. |
| Quality Standards | QS01.1/2: RAII, checked native errors, finite waits, scoped observation, native FFI review, no global fault hook. QS01.3/4 still require final CoW/flush/resource recovery evidence. |
| Completeness Criteria | CC01.2/4/5 gain thread/process/alias/cache/partial-lock/cancellation/lifetime witnesses. CC01.1 C03 publication sites, preparation-dedup wakeup and final resource witnesses remain incomplete. CC01.3 formatter correction is a separate commit. |
| DoD | DOD01.1/2 verified for this slice with actual native/causal evidence and labeled self-review. DOD01.3 is NOT satisfied: no integration merge or G-FOUNDATION. DOD01.4 gains the namespace map, not a finished reaper/recovery proof. |
| Invariants | The stage cannot outlive its borrowed lease; no prepared-object proof is forged; existing refs/source data stay unchanged. No availability, attribution, activation or durability claim exceeds tested scope. |

The five groups and all 21 numbered SI-01 criteria remain unchanged. **SI-01 stays IN PROGRESS.** Next implementation is final CAS/readiness publication and requalification under these leases, then the missing preparation/resource/cancellation witnesses. Public routing remains unchanged and `production_protocol_enabled=false`. No G-FOUNDATION/G-ACTIVATION, integration/main merge, real-store migration, release or coding-agent dispatch occurred.

## Evidence retention

All five native archives were downloaded and independently checked: external archive digests, every declared internal artifact/binary hash, source-input manifests, exact checkout/parents/tree/fingerprint, required names and observed counts. The full archived source was reconstructed with Git to the expected tree; all nine submitted files matched byte-for-byte. Workspace, CLI and lease-control archives were inspected as well. No native binary from those archives was executed locally.

The durable delivery retains the raw archives, prior harness failures, the nine-file patch and an offline verifier. GitHub artifact copies expire on 2026-10-17. Checksums protect the retained evidence; they are not signed independent attestation or new execution of those tests.
