# SI-01 — reviewed inactive metadata integration

**Date:** 2026-09-17. **Campaign:** #152. **Work package:** #147 remains OPEN.
**PR #156:** MERGED into `fix/snapshot-integrity`, not main.
Authority: owner's ongoing execution, reviewed-merge and hygiene mandate.
Review is author/coordinator self-review, not independent human/agent approval.

## Exact identities

| Identity | Value |
|---|---|
| Reviewed source | `17dbfa5b0d455f7121cda68724f31736a2b658ef` |
| Integration base | `77a25c2b395e08ddfaf79e9fc477c95e2db1a88f` |
| Actual tested PR merge | `6647555a0bd9ab8553dcded5ad19d2e512cf03a1` |
| Actual integration merge | `1dff1b927c8e8f2aeda4a4145651fc48277f590d` |
| Tested/integrated tree | `4caed236fbaea49ddc68f0caa03a81b49483deaf` |
| Execution-input fingerprint | `2ec98f6e2541012af1129b7963f19aaa02aedab4522c651d19e0fe944346301e` |

Expected-head-checked merge succeeded; independent read-back confirms identical
tree and ordered parents for test/actual merges. This later record changes data,
not the tested executable. Contract v2.2 and Accepted ADR-0020 are unchanged.

## Reconciliation and implementation

The existing metadata branch was reconciled with current integration by merge,
not a rewritten history. Two conflicts were resolved additively: module
registration and union of required test names. No prior tests, ignores or
shared foundation implementation were lost. All changed Rust files are below
400 lines. No dependency, Cargo.lock, unsafe code or public routing change.

The helper installs bounded metadata with actual byte/length verification,
retained file-identity verification through rename, checked writable-handle
synchronization, Unix namespace barriers and explicit owned cleanup. Physical
visibility outcomes remain distinct from logical reference-transaction outcomes.
ReadinessFrame preserves exact original bytes around the one shared decoder;
the two frozen SI-00 vectors were checked, never regenerated.

Three defects were reproduced then corrected: a replaced staging name could
install different bytes than its verified open handle; an invalid custom Read
count could panic; completed explicit cleanup could run Drop cleanup again and
delete a successor at the retired name. Corrections verify identity, bound the
reader count, and disarm completed cleanup. Four additional mandatory tests
also cover confirmation preserved after final cleanup-barrier failure.

## Executed evidence

| Gate | Result |
|---|---|
| [Workspace 35284114064](https://github.com/gmhelmold/hugr-lightr/actions/runs/35284114064) | fmt/build, 1773 passed, zero failed, one historical ignore; all-targets Clippy and integration controls PASS |
| [Native/CLI 35284113976](https://github.com/gmhelmold/hugr-lightr/actions/runs/35284113976) | Five profiles and aggregate PASS after infrastructure retry; full F0-F5 PASS |
| [Metadata 35284113978](https://github.com/gmhelmold/hugr-lightr/actions/runs/35284113978) | Frozen vectors, Store tests/Clippy/format and seven compiled causal controls PASS |
| [Foundation 35284114061](https://github.com/gmhelmold/hugr-lightr/actions/runs/35284114061) | Prior codec/lease/publication/cancellation/release controls PASS |
| [Contract 35284114066](https://github.com/gmhelmold/hugr-lightr/actions/runs/35284114066) | Immutable contract checks PASS |

Native Store/index totals: Linux x86_64/ARM 232 each; macOS Intel/ARM 218 each;
Windows MSVC 203. Total 1103 executions, zero failures, four historical ignores,
formatter zero on all profiles. These overlap workspace tests; they are not
1103 new methods. This increment adds 24 methods (23 portable, one Unix-only).

All seven mutants compile and fail one exact required assertion, then their
restored exact test passes. Four previous controls are retained; three new
ones remove staged-identity, reader-bound and cleanup-disarming protections.
Final metadata subset: 23 passed. Timeout/build failure/empty suite cannot pass
the causal gate. The local initial partial-name --exact selection executed
zero tests and was rejected as evidence; its corrected full-name test failed.

Local Intel Mac, isolated temporary clone, Rust 1.96.0/two build jobs: Store
167 passed, metadata 23 passed, seven compiled controls, targeted all-targets
Clippy/formatter and 42 Python support methods passed. Three new regressions
were observed before their fixes. No user's existing checkout was modified.

The initial hosted Intel runner failed BEFORE compilation because Cargo could
not resolve index.crates.io. Its original archive (10523298375) is preserved,
not counted as passing. A premature job retry returned workflow-still-running;
after completion, failed-job retry was accepted and the same source passed.
Successful Intel artifact is 10523478987; same-name archives are not conflated.

Offline verification rehashed nine raw archives, every native artifact/binary,
mandatory test names and each 809-file execution manifest. The source archive
reconstructs to the independently fetched Git tree. Metadata control receipts,
pristine/mutant/restored logs and workspace outputs were checked. No retained
native executable was run in the authoring sandbox.

CLI regression: 107 measured iterations, 16 warmups, 369 successful commands,
independent restored-tree checks. Binary SHA-256:
`8e3a3c90d4daa43fbe0ff4afb12befc64d44a738a8e6ce8fbeda9150ecfb353d`.
These unchanged-route tests are not activation or final performance acceptance.

[Final self-review](https://github.com/gmhelmold/hugr-lightr/pull/156#pullrequestreview-5242260311)
accepts this increment only. Prior failed receipts remain historical evidence.

## Hygiene and remaining work

The metadata branch was removed only after exact remote tip 17dbfa5, no open
PR, merged ancestry and remote-absence checks. Its history is retained in the
integration merge. No unrelated branch/checkout or sibling repository changed.
Current dispatch and issues must point to this integration, not say #156 awaits
reconciliation. This follow-up is documentation-only; execution inputs remain
identical to the verified candidate.

`production_protocol_enabled=false`; G-FOUNDATION/G-ACTIVATION NOT ESTABLISHED.
The installer still requires externally held resource/Store and key exclusion,
and stable caller-validated paths. No complete C12 hostile-path guarantee or
logical journal decision is supplied. SI-01 remains open for CoW/fallback,
checkpointed existing-object inspection, topology/resource/reaping, typed
caller integration and remaining qualification. All five axioms/21 SI-01
criteria and all 123 campaign IDs remain mandatory. No main merge, release,
real-data migration, account permission change or coding-agent dispatch.

Raw GitHub artifacts expire 2026-10-17. The final conversation package contains
their bytes and an offline verifier; Library persistence is reported only after
a successful save. This repository receipt is persistent independently.
