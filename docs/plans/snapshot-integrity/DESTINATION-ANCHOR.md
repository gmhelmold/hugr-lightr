# C12 — anchored actual-destination root

Base: `9bf1af67d0c6c4051a4bd7851138cd491688da0f` (accepted #179).
SI-01 #147 / campaign #152; parent #146 remains draft.
Authority: accepted ADR-0020 and C12 in remediation v2.2.
This is an inactive foundation increment. It does not activate hydrate or public routing.

## Scope

`DestinationInspection::anchor_empty(wait)` converts one read-only destination
observation into an exact retained destination directory. Existing empty destinations
are retained but never operation-owned. A missing suffix is created component by
component from the already retained nearest existing ancestor using native
descriptor-relative operations. No pathname is re-resolved to choose where to write.

Creation is create-only. If a component appears after preflight, EEXIST is a
failure; no file, directory or link is adopted. Created directories are opened with
no-follow/CLOEXEC semantics and their native identity is checked. Protected roots
are re-observed, and the requested destination must resolve to the retained final
handle before success. This intentionally permits the operation's own missing
suffix to become present only when it resolves to that exact handle.

The capability exposes no raw descriptor or path. It does not create descendants,
files or links, does not write payloads, does not certify actual-directory name
representability, and is not a Store lease or PreparedObject. A later foundation
adapter may consume the retained handle internally; public hydrate remains unchanged.

## Failure, ownership and rollback

Missing components start mode 0700 subject to umask. The operation retains every
created directory while acquiring deeper components. Explicit rollback removes only
the created suffix in reverse order, checks native identity, and uses nonrecursive
directory removal. Existing destination directories are never removed by rollback.

Unknown children, replacement entries or identity changes prevent cleanup and are
preserved. `DestinationAnchorFailure` keeps the primary operation error separately
from cleanup errors and reports `cleanup_complete=false` whenever the operation
cannot prove its created namespace was fully removed. No retry can turn an
uncertain cleanup into success. Failed identity acquisition after mkdir is reported
without destructive cleanup under an unproved binding.

Cancellation is cooperative around bounded calls. A cancellation after one or more
created components triggers checked rollback. Native synchronous calls cannot be
preempted. Abrupt process death is not made transactional by this primitive.

## Race and platform boundary

This capability closes the ordinary string-reopen gap; it is not a hostile-writer
namespace lock. Final binding validation rejects observed replacement before the
anchor is returned, and later `revalidate` checks the binding again. A cooperating
writer must continue to use the retained anchor for descendants and retain whatever
resource/lease guard its operation requires.

Linux and macOS use the already qualified C12 topology subset. Windows remains
Unsupported because native destination topology is still unqualified there. This
increment neither weakens that boundary nor infers cross-platform success.

## Five acceptance groups

### Success criteria

Create or retain exactly the inspected empty destination without reopening an
untrusted pathname, adopting concurrent names or touching protected/user data.
Return a capability only after the requested path binds to the retained handle.

### Completeness criteria

Cover existing empty/nonempty targets; multi-component missing suffixes; native
alias/parent resolution; concurrent name appearance; cancellation after partial
creation; replacement before final validation for created and existing targets;
unknown-child rollback; CLOEXEC; planned protected root appearance; and checked
reverse cleanup. Eleven Unix-native methods are mandatory on Linux/macOS.

### Quality standards

Pinned Rust 1.96.0, formatter, denied-warning Clippy, full Store/doctests and
exact-head Actions. Native unsafe calls require line-level ownership review. Add
three compiled causal controls for create-only acquisition, final handle binding
and identity-checked rollback. Preserve every previous topology/listing control
and suite count. Synthetic policy validation is not native qualification.

### Invariants

No hydrate/index routing change, payload write, descendant creation, link creation,
raw handle/path escape, adoption of pre-existing output, recursive deletion,
protection/gate weakening, dependency/codec change, retry-until-green, main merge
or protocol activation. Existing directories are never operation-owned.

### Definition of Done

Qualify exact source with full/native/causal Actions; publish explicitly labeled
coordinator self-review; merge only the expected head to `fix/snapshot-integrity`;
verify fresh #146 checks independently; retire only this delivery branch and owned
temporary checkout. SI-01, G-FOUNDATION and G-ACTIVATION remain separate.

## Local preparation evidence

The first targeted compile correctly rejected a test that attempted to let an
anchor outlive a temporary inspection (E0716); the test fixture was fixed without
weakening the borrow. The next targeted native run passed all eleven anchor methods.
A denied-warning build then found an internal handle getter with no production
consumer; it was restricted to tests rather than suppressing the warning or
exporting a premature seam. Complete local gate results are recorded in the live PR.

The private scratch-name probe #179 and source listing #180 remain prerequisites,
not substitutes for actual-destination anchoring. This anchor still does not certify
the destination's whole-tree name policy or implement the descendant writer.
