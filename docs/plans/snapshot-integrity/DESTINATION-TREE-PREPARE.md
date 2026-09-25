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
remain held internally by the prepared object. This slice deliberately exports no
production payload-handle accessor yet. POSIX links use `symlinkat` with the exact
stored UTF-8 target text. No descendant pathname is reopened globally.

Directories and files start private (0700/0600 subject to umask); final manifest
modes are not applied here. File payloads are not read from CAS or written by this
public preparation API. The retained file-handle seam is crate-internal. Link
creation is exact text only; it does not follow targets. Windows remains
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
file handles internally without exposing a premature payload-writing API.

### Completeness criteria

Cover empty trees; explicit/implied directories; exact dangling link text;
private temporary modes; CLOEXEC retained file handles; representation-before-
persistent-create ordering; late native errors; operation-created destination
roots; concurrent planned-name appearance; cancellation after creation;
replacement identity; unknown children; root replacement; leaf replacement;
foreign root occupancy; explicit rollback and best-effort Drop. Fifteen
Linux/macOS methods are mandatory.

### Quality standards

Rust 1.96.0, source files below the repository 400-LOC production guard, formatter,
denied-warning Clippy, full Store/doctests/support/CI and Windows-GNU compile.
Exact-head native/full Actions remain mandatory. Preserve all 33 existing
topology/listing/anchor/representation causal tuples. Add five compiling controls:
file no-adoption, identity-checked cleanup, exact final namespace, leaf binding and
final root binding. Every mutant must fail its exact assertion, then the restored
15-method family must pass.

### Invariants

No public hydrate/index activation, CAS payload read/write, final file-mode
publication, recursive foreign cleanup, pathname-based descendant creation,
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

A later payload writer may consume the retained file handles, verify exact bytes
and apply supported final modes before any success/publication transition. This
prepared object has no commit/publish method; dropping it cleans best-effort and
explicit rollback remains the evidence-bearing exit. Windows native link/output
support, resource/E18 qualification and public orchestration remain separate.
