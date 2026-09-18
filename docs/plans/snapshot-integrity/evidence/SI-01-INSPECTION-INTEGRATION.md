# SI-01 — existing-object inspection cancellation

**Date:** 2026-09-18. **Campaign:** #152. **WP:** #147 remains OPEN.
**Code delivery:** #161 MERGED into `fix/snapshot-integrity`, not main.
**Scope:** reviewed inactive increment; not full SI-01 acceptance.

## Exact identities

| Identity | Value |
|---|---|
| Source | `3629b310cfa004dec5918b862ec2acea05e4411f` |
| Base | `5254a3ec30561c5eb354f68bb3061fd0afb6efec` |
| Actual tested merge | `aa6e07ff26c49be94e5b8c2dd63df39d6ff07aef` |
| Actual integration | `2aa762fd9b5c4b90800e4752a4f63e57fb9bd46e` |
| Shared tree | `60ea55bb182e44c67bc55f26fb9cbec48c99542f` |
| Execution-input fingerprint | `5225739a9e04fe93bba4273455595a714507b999d7a3a8774ed97b48eb8fcdb8` |

The tested and actual merge have ordered parents [base, source] and the same
read-back tree. Merge used an expected-head check and unchanged protection:
Required CI from Actions app 15368, strict/up-to-date checks, enforce_admins.
This later documentary receipt is not a newly tested executable identity.

## Behavior and limits

Layout::inspect receives the operation checkpoint and reuses the existing
64 KiB Digest::of_reader_checked. It checks before/after bounded receipt
reading, around payload hash reads (including EOF), and before returning the
validated, rewound handle. No second hasher, mmap or full-file buffer.

Only a checkpoint's own Wait::check failure maps to Verify/NotPublished.
Actual I/O and integrity errors preserve their cause and RecoveryRequired,
even when cancellation is concurrently set. Cancellation does not install a
payload or receipt, overwrite their native identity, or reclaim another
allocation. Same-key waiters receive the leader's actual shared failure;
other digests can progress, and retry of a failed key requires a fresh lease.

This is cooperative: no preemption of an in-flight synchronous OS call and
no hard latency promise. NotPublished describes payload/receipt publication,
not absence of namespace/lock creation before the check. Receipt parsing is
bounded, not checked per byte. Public Store/CLI/snapshot/GC routing is unchanged.

## Candidate evidence

All seven candidate workflows completed successfully on attempt 1:
complete CI 35305557425; native/CLI 35305557374; workspace 35305557397;
contract 35305557368; foundation controls 35305557429; wire/publication
35305557402; benchmark-evidence 35305557386. They are reachable from
[the reviewed PR](https://github.com/gmhelmold/hugr-lightr/pull/161).

Workspace: 1796 passed, zero failed, one historical ignore. Native Store/index:
Linux x86_64 240, Linux ARM 240, Intel macOS 226, ARM macOS 226, Windows 211;
1143 passing executions, zero failures, four historical ignores. Totals overlap
and are not distinct/new tests. All seven added method names were checked in
each native archive and workspace log. The native mandatory-name policy itself
was not extended; observed execution does not mean new policy enforcement.

Seven archive digests, every 21-item native artifact inventory, executable
hashes, 820 execution inputs/profile and source identities were checked offline.
All 845 source files and committed modes reconstruct the exact Git tree;
the canonical input fingerprint also matches. No foreign binary was run by
the offline verifier. Local manifests are integrity records, not signatures.

CLI F0-F5: 107 measured iterations, 16 warmups, 369 successful commands;
all recorded tree-equality checks true, canonical fixture identities checked.
Recorded binary SHA-256:
`d2c6646da96b7240eac5d5db8126e7ba5bcdb415eb469f8d81aac28bc41dde9d`.
The receipt states runtime_qualified=false and budget_status=NOT_REGISTERED;
this remains regression/baseline evidence, not numeric SI-05 acceptance.

## Additional local review evidence

Fresh isolated Intel-Mac worktree, Rust 1.96.0, separate target/home/tmp,
two build jobs, four test threads, warnings-as-errors; debug information and
incremental compilation disabled to bound disk use. Full Store library passed
175/175 without skips; all-target Store Clippy, formatter and 13 CI-policy
methods passed. Other live worktrees were not modified.

Four local mutants compiled then failed the exact selected assertions:
missing read checkpoints; lost checkpoint-error provenance; late first check;
real I/O misclassified as cancellation. Source was restored after each control,
and the intact seven-method selection passed again. These are local review
controls, not newly installed CI jobs. Raw review archive SHA-256:
`516cece9c8e4ffc63ebbba74c62bc7d01e3bfc750e02f9bce5c23bfb417a317c`.

[Coordinator review](https://github.com/gmhelmold/hugr-lightr/pull/161#pullrequestreview-5244261319)
explicitly identifies self-review, not independent human/agent approval.
No failure, timeout or compiler error was relabeled as a causal pass.

## Post-integration and continuation

Parent #146's NEW full CI is 35307295711; companion benchmark-evidence is
35307295710. Confirm their final outcomes on actual head 2aa762f before
claiming post-integration verification. Later maintenance heads require fresh
checks too; current outcomes/closure are recorded on #146 and #161.

All five SI-01 axiom groups and 21 criteria remain unchanged, as do the v2.2
plan's 123 criterion IDs, Cargo.toml, Cargo.lock and existing CI configuration.
production_protocol_enabled=false; G-FOUNDATION/G-ACTIVATION NOT ESTABLISHED.
CoW/fallback, topology/resource/reaping, typed metadata consumers and remaining
qualification stay with #147. #146 remains draft and #152 remains open.
No main merge, release, real-data migration or account-permission change.
