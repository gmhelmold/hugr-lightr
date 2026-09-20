# C07/C12 — actual-destination whole-tree name representability

Base: `1d6b234ea86a876a40a6ac774b77e085f4373874` (accepted #181).
SI-01 #147 / campaign #152; parent #146 remains draft.
Authority: accepted ADR-0020 and C07/C12 in remediation v2.2.
This is an inactive foundation increment. It does not activate hydrate/public routing.

## Scope

`DestinationAnchor::probe_tree_names(TreePlan, NameProbeLimits, Wait)` exercises
the complete logical tree's directory and leaf NAMES directly inside the exact
retained destination root. It uses the destination itself as the owned temporary
reservation area rather than transferring a result from Store-private scratch.

The root must still be empty. Planned directories are created with create-only,
no-follow native operations. Regular-file and symlink entries both use empty
regular-file markers: leaf-name coexistence is tested, but payload bytes, modes,
link target text, link kind and native link capability are NOT. All siblings in
one parent coexist before recursion or cleanup so early deletion cannot hide
case/Unicode collisions. Directories reproduce the planned hierarchy.

Every temporary name is removed before success. The root is then scanned empty
again and rebound through the original DestinationAnchor. A later materializer
must still use create-only anchored writes; this preflight is not a namespace lock.

## Native ownership and races

Creation never adopts an existing name. File markers use O_CREAT|O_EXCL|O_NOFOLLOW
and CLOEXEC. Directories use mkdirat followed by no-follow acquisition and native
identity comparison. If identity cannot be established after a native create, the
failure reports cleanup_complete=false rather than guessing destructive ownership.

Explicit cleanup checks the exact parent/name identity before unlinkat. Directory
removal is nonrecursive, so an unknown child blocks removal and is preserved.
Replacement names are also preserved. Primary operation errors and cleanup errors
remain separate. Cancellation after partial creation triggers explicit reverse
cleanup; synchronous native calls are not asynchronously preempted.

Before any mutation the complete plan is bounded by caller node/depth limits and
implementation ceilings of 4096 nodes and depth 64. The exact anchored root is
revalidated and scanned empty. After cleanup, a foreign root entry, destination
replacement or protected-topology change prevents success.

## Representation boundary

A success means only that all logical NAMES could coexist in the temporary tree
under this exact destination's native directory semantics at this time. It does
not prove bytes, permissions, timestamps, symlink semantics, durability, capacity,
resource reservation or immunity from a later race.

The same-directory oracle test intentionally accepts the platform's native
case/Unicode behavior rather than assuming APFS/ext4 policy. Windows remains
Unsupported because native C12 destination topology is not yet qualified there.
Reserved Windows names and native link-kind creation therefore remain a later
Windows-specific obligation, not a hidden success claim.

## Five acceptance groups

### Success criteria

Exercise every planned directory/leaf name directly under the retained destination,
preserve native collision/error semantics, clean every operation-owned temporary
name, and return success only after the root is empty and still bound to the anchor.

### Completeness criteria

Cover empty trees; explicit/implied hierarchy; link names as non-link markers;
same-directory case/Unicode outcomes; post-empty-check foreign-name races;
cancellation; replacement identity; unknown-child cleanup; final root occupancy;
final root replacement; node/depth bounds; late ENAMETOOLONG cleanup; CLOEXEC; and
a missing destination root that can still rollback after successful probing.
Fourteen Unix-native methods are mandatory on Linux/macOS.

### Quality standards

Pinned Rust 1.96.0, formatter, denied-warning Clippy, complete Store/doctests,
support/CI policy and exact-head Actions. Four compiling causal controls cover
O_EXCL/no-adoption, identity-checked cleanup, final empty-root verification and
final anchor rebinding. Preserve every previous topology/listing/anchor control
family and its pristine/restored suite count. No lint suppression is acceptance.

### Invariants

No payload bytes, symlink creation, target interpretation, public routing,
unanchored path writes, recursive cleanup, adoption of pre-existing names, Store
mutation, dependency/codec/workflow/protection weakening, retry-until-green,
main merge or protocol activation. Existing destination roots remain user-owned.

### Definition of Done

Qualify exact source with full/native/causal Actions; publish explicitly labeled
coordinator self-review; merge only the expected head to fix/snapshot-integrity;
verify fresh #146 CI independently; then retire only this delivery branch and its
owned temporary checkout. SI-01, G-FOUNDATION and G-ACTIVATION remain separate.

## Local preparation evidence

Owner Intel Mac, isolated clone, pinned Rust 1.96.0 / Python 3.14.5. The final
source layout keeps every new Rust file below the repository's 400-line godfile
convention: production modules 278/382 lines and test modules 345/192 lines.

Final local gates on the same source: focused destination-name family 14/14;
full Store library 371/371; 11/11 doctests; Clippy for all Store targets with
denied warnings and the repository Swift rpath; rustfmt; Windows-GNU
`cargo check --tests` with `-D warnings`; support 197 passed with one existing
Linux-only skip; CI-policy 30/30. Counts overlap.

Four final-source causal variants each compiled successfully, then their exact
target test failed with Cargo exit 101 and the preregistered assertion/message:
no-adoption/O_EXCL, identity-checked cleanup, terminal root-emptiness validation,
and terminal anchor rebinding. After all four variants were restored, the exact
14-method destination-name family passed again.

Failed observations are retained as engineering evidence rather than hidden:
the first denied-warning Clippy run rejected an over-wide traversal signature
and a redundant return; both were refactored without lint suppression. The first
Windows-GNU run rejected a Unix-only import, and the next exposed type inference
from Unix-only cleanup state. The final code cfg-isolates the native branch and
passes Windows-GNU denied warnings. No failed test, lint or platform gate was
removed or weakened.

Exact PR/Actions evidence still owns merge qualification. These local results do
not qualify Windows native topology, payload/link representation, public routing
or the complete SI-01 package.
