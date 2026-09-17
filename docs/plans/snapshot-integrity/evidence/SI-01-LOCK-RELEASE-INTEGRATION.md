# SI-01 lock release and reviewed incremental integration

**Date:** 2026-09-17. **Campaign:** #152. **WP:** #147 remains OPEN.
**Delivery:** #155 MERGED into `fix/snapshot-integrity`, not main.
**Authority:** owner explicitly delegated continued implementation, reviewed
merges, repository organization and hygiene. Reviewer is the author/coordinator;
no independent human/agent approval is represented.

## Identities and acceptance boundary

| Identity | Value |
|---|---|
| Technical contract | v2.2 at `e30811971f6a1c2a1e2fdb0a91dc8ccf9a06879d` |
| Reviewed source | `8d1276858277fcf507038fa10d41b64bf54b1877` |
| Actual tested PR merge | `e20ebe915fb67faf2437888e00fdbf27d3b2fb1e` |
| Actual integration merge | `b7ed8953df506f00f77515bcaabc117afc15ee40` |
| Tested and integrated tree | `fed8b882d52cb10571bb24d95430c2d7dabfb9de` |
| Input fingerprint | `ca13ee3578f3439236e9157a0a03ba47f7f97cbe6285c4fcb18a7cb254e3094a` |

Both merges have ordered parents `9ab0eb764d8f637b16c139199f3bbf39cc4f74f6`
and `8d1276858277fcf507038fa10d41b64bf54b1877`. GitHub accepted an
expected-head-checked merge; read-back confirmed tree/parents. This later
receipt/dispatch/archive changes data only, not the tested executable.

**Accepted: this inactive increment. Not accepted: the complete SI-01 WP.**
`production_protocol_enabled=false`; G-FOUNDATION and G-ACTIVATION remain
NOT ESTABLISHED. Existing Store/snapshot/CLI/GC routing is unchanged. No main
merge, release, real-data migration or coding-agent dispatch occurred.

## Lock-release correction

Unix NativeLock now stores its acquiring process ID and explicitly unlocks
only in that process's Drop, then closes its private handle. Duplicated file
descriptions cannot extend the normal owner lifetime; foreign-process cleanup
cannot prematurely release the live owner's lock. Windows is unchanged.
No unsafe code, dependency, format or ignored-test addition. Unlock errors in
Drop retain conservative close fallback; successful release on an OS failure
is not promised. Inherited leases cannot be used for new work after fork.

The original counterexample is byte-unchanged and now passes. Five additional
methods check independent shared owners, exclusive ownership, stable lock file,
foreign-owner cleanup, unwind and post-acquisition errors. The foreign-owner
method simulates ownership with an internal seam; it is not a real fork test.

## Exact candidate evidence

| Run | Result |
|---|---|
| [35280975132 workspace](https://github.com/gmhelmold/hugr-lightr/actions/runs/35280975132) | fmt, build, 1749 passes/zero failures/one historical ignore, all-targets Clippy and integration controls PASS |
| [35280975244 native/CLI](https://github.com/gmhelmold/hugr-lightr/actions/runs/35280975244) | complete five-profile matrix and full F0-F5 sequence PASS |
| [35280975400 foundation controls](https://github.com/gmhelmold/hugr-lightr/actions/runs/35280975400) | existing controls plus two compiling release mutants PASS |
| [35280975118 contract](https://github.com/gmhelmold/hugr-lightr/actions/runs/35280975118) | immutable contract PASS |

Native Store/index: Linux x86_64 208; Linux ARM 208; macOS Intel 194; macOS ARM
194; Windows 180. Total 984 executions, zero failures, four historical ignores;
formatter zero on each profile. These overlap workspace results and are not
984 new unique tests. All six release test names passed on every Unix profile.

The close-only and unconditional foreign-process-unlock mutants each compile
then fail the exact required assertion (exit 101). Pristine/restored six-test
suites pass. On the owner's Intel Mac, pinned Rust 1.96.0 reproduced the
original failure before the fix; afterwards six release tests and three full
parallel Store runs of 143 tests passed, as did targeted Clippy/formatter.

Independent offline verification checks all nine raw archive digests, the five
native artifact/binary inventories, exact parents/checkout/tree/fingerprint,
all 796 input files and required release names. The source archive's Git tree
reconstructs exactly. The verifier force-adds already-versioned archive files
rather than losing tracked-but-ignored files. Raw workspace log bytes remain
preserved; decoding for summary inspection tolerates intentional non-UTF8 test
output, not altered source. No foreign executable runs in that verifier.

CLI F0-F5: 107 measured iterations, 16 warmups, 369 successful commands, with
independent restored-tree comparisons. CLI SHA-256:
`8d971808adda27d090dda3dd3b6f182292a59624fd5aef98544ed9ba3b1399bf`.
This is regression coverage, not new-protocol public activation or final
performance-budget acceptance.

[Final self-review](https://github.com/gmhelmold/hugr-lightr/pull/155#pullrequestreview-5242060224)
records the exact scope, evidence and incremental integration decision.
Earlier failed receipts and tool refusals remain historical; neither is
rewritten as success or given an invented internal cause.

## Repository hygiene

The closed SI-00 and SI-01 delivery branches will be removed only after exact
head and merged-ancestry checks. The retired admin branch has zero net diff
against its base and no open PR. Its unique history is preserved in
[the compact Git bundle](archive/snapshot-tracking-setup-20260917.bundle),
8918 bytes, SHA-256 `df2a4644519e6e4e60fcf3adbd1014dc20d6ab4a951a7c7ece34ba93172f7a2b`.
The bundle requires base `e30811971f6a1c2a1e2fdb0a91dc8ccf9a06879d`, which remains
in repository history. `git bundle verify` passed. It is archival data, not an
active workflow. No secrets or executable deployment artifact are stored in it.

Cleanup status at authoring: PENDING read-back; update after actual deletion.
PR #156 / `work/si-01-wire-primitives` contains unmerged metadata changes and is
explicitly preserved. No blind deletion of other branches or user checkouts.
The duplicate CI path filter was removed. The account Project query still lacks
read:project; no credential/scope change or fictitious board completion occurred.

## Remaining package work

SI-01 stays OPEN. Preserve all five axioms/21 criteria: full CoW/fallback,
checkpointed existing-object inspection, C12 topology, resource/metadata/reaping
and remaining caller/qualification obligations. #156 needs reconciliation and
review against the integrated foundation. Inert incremental merges do not
release downstream production integration before full G-FOUNDATION.

Current nine raw evidence archives are assembled separately with an offline
verifier. GitHub copies expire 2026-10-17; Library persistence must be checked
before claiming a separate persistent copy. This GitHub receipt is durable.
