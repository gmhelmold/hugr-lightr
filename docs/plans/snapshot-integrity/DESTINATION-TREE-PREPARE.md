# C07/C12 — prepared anchored destination tree

Base: `390a4c6dd0806fa0f8df6d0801059c8efa89ff00` (accepted #182).
SI-01 #147 / campaign #152; parent #146 remains draft.
Authority: accepted ADR-0020 and remediation v2.2.
This increment remains inactive: no public hydrate/index routing is changed.

## Scope

`DestinationAnchor::prepare_tree(TreePlan, NameProbeLimits, Wait)` first executes
the accepted actual-destination NAME probe, which must clean the root completely.
Only then does it prepare the real namespace through the retained destination
handle. Explicit/implied directories use descriptor-relative create-only
`mkdirat`; regular files use `openat(O_CREAT|O_EXCL|O_NOFOLLOW|O_CLOEXEC)` and
remain held internally by the prepared object. Crate-internal
`PreparedDestinationTree::write_file_payload` streams one caller-supplied payload
through its retained handle, checks exact size
and digest, then applies the manifest mode and syncs. POSIX links use `symlinkat`
with the exact stored UTF-8 target text. No descendant pathname is reopened globally.

Directories and files start private (0700/0600 subject to umask); final manifest
modes are not applied during preparation. `write_all_payloads_from_store` opens and
revalidates each CAS object without buffering it, then fills every prepared file;
any failure rolls back all operation-owned output. Link creation is exact text only;
it does not follow targets. Windows remains
Unsupported because native destination topology is still unqualified there.

The whole-tree representability probe is a mandatory predecessor, not a
transferable certificate. A name appearing after the probe is still rejected by
create-only operations. The prepared tree validates every owned binding again,
and it enumerates the retained root plus each prepared directory through fresh
directory descriptions to require exactly the planned child count. Foreign names
therefore cannot coexist with a successful prepared result.

## Ownership, failure and cleanup

Every created name records its retained parent and native identity. Explicit
rollback removes links/files before directories and operates only when that
identity still matches. Directory removal is nonrecursive: unknown children,
replacement names or identity changes are preserved and set
`cleanup_complete=false`. Primary and cleanup errors remain separate.

A file whose native identity cannot be established after creation is not guessed
safe to remove. The failure is returned with uncertain cleanup rather than risking
deletion of a replacement. Drop performs the same cleanup best-effort only as a
panic/abandonment fallback; explicit rollback is required when cleanup evidence
matters. This primitive is not crash-transactional.

The prepared object borrows its `DestinationAnchor`; the anchor cannot disappear
while prepared descendants remain live. Final validation rechecks the root
binding and every owned descendant. Native identity cannot prove every theoretical
hostile ABA history; this closes the ordinary pathname-reopen/substitution races
inside the supported C12 profile.

## Five acceptance groups

### Success criteria

Run actual-destination representation preflight first, then create exactly the
planned directory/file/link namespace from retained handles. Never adopt an
existing name, follow a destination alias while descending, or return success
with an unplanned child. Preserve exact POSIX link target text and retain regular-
file handles internally. Payload writing verifies bytes before applying final mode.

### Completeness criteria

Cover empty trees; explicit/implied directories; exact dangling link text;
private temporary modes; CLOEXEC retained file handles; representation-before-
persistent-create ordering; late native errors; operation-created destination
roots; concurrent planned-name appearance; cancellation after creation;
replacement identity; unknown children; root replacement; leaf replacement;
foreign root occupancy; explicit rollback and best-effort Drop; CAS success,
missing-object rollback and payload cancellation. Fifteen original plus coordinator
Linux/macOS methods are mandatory.

### Quality standards

Rust 1.96.0, source files below the repository 400-LOC production guard, formatter,
denied-warning Clippy, full Store/doctests/support/CI and Windows-GNU compile.
Exact-head native/full Actions remain mandatory. Preserve all existing
topology/listing/anchor/representation causal tuples. Destination preparation now
has nine compiling controls, including descendant checks before final root binding.
Every mutant must fail its exact assertion, then the restored 31-test family must pass.

### Invariants

No public hydrate/index activation, snapshot publication, CAS mutation, recursive
foreign cleanup, pathname-based descendant creation,
pre-existing name adoption, Store mutation, dependency/codec/workflow/protection
weakening, retry-until-green, main merge or protocol activation. Existing
destination roots remain user-owned; rollback never removes the root itself.

### Definition of Done

Qualify the exact source with complete/native/causal Actions; publish an explicitly
labeled coordinator self-review; merge only the expected head into
`fix/snapshot-integrity`; verify fresh #146 CI independently; then retire only this
delivery branch and owned temporary checkout. SI-01, G-FOUNDATION and
G-ACTIVATION remain separate.

## Boundaries for the next slice

Payload coordination now consumes retained file handles, verifies exact CAS bytes
and applies supported final modes before `complete` disarms cleanup. `complete`
does not publish a ref or snapshot; failed completion rolls back. Windows native
link/output support, resource/E18 qualification and public commit orchestration
remain separate.
