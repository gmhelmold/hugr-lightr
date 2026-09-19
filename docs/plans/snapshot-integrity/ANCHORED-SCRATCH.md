# C12/C13: lease-borrowed anchored scratch

Inactive SI-01 increment under #147/#152, based on integrated #173 at
`671e3b7fec267fbf8ba5e015da7de680b7ddd709`. ADR-0020 and v2.2 remain unchanged.
No public hydrate, capture, CLI, codec, GC or production protocol is activated.

## Boundary

`ScratchDirectory::reserve` selects `.si01-staging` through the supplied Store
lease; there is no caller-supplied output path. Atomic reservation tries at most
32 names. Descendants borrow their parent and the same lease. The API exposes
neither paths nor raw descriptors. It only creates new single-component names;
existing files, directories and dangling links cannot be adopted or truncated.

Linux/macOS use retained parents, mkdirat and openat with exclusive/no-follow
creation; every descriptor is close-on-exec. Directories begin with mode0700 and
files0600, subject to the process umask. Read/Write address retained descriptors.
No process-global cwd, permission, failure-hook or environment mutation is used.
Windows returns Unsupported before allocating staging; its lock API is unchanged.

This is a managed reservation for subsequent native tree/name probes, NOT the
probe itself or a public destination writer. Same filesystem alone does not prove
same per-directory case/Unicode policy. There is no link-creation API, payload
verification, PreparedObject conversion, namespace lock or durability promise.
A public adapter still needs full tree representability and C12 preflight before
output writes. Existing Store roots must be validated and remain stable, exactly
as required by the current StoreLocks/managed-staging contract.

## Cleanup and resource limits

Each entry records its native identity and owning directory. Explicit `finish`
checks that identity/type and removes only that name. It never recursively
removes a directory. Unknown children or replacement entries cause an error and
are preserved. Each guard makes one cleanup attempt, including on failure;
Drop is best-effort for normal unwinding. File descriptor close errors are not
reported by File::drop; `finish` promises checked name removal, not a flush or
close/durability barrier. Nothing in this API acknowledges published payloads.

The allocation is private to its owner: identity checks are not atomic exclusion
against a hostile writer concurrently replacing entries inside private scratch.
If identity cannot be acquired after creation, the error propagates and scratch
may remain rather than deleting by an unproved name. I/O/cleanup failures and
abrupt termination can leave scratch for the future matching-domain EX reaper.
No arbitrary user data, sibling Store or cache allocation is cleanup authority.
A lease prevents cooperating Store GC, not unrelated host filesystem activity.

## Five acceptance groups

### Success criteria

Create/read/write bounded-stream temporary data via retained directory handles;
never adopt existing names or redirect creation through replacement path spelling.
Parent/lease lifetime is retained, and explicit cleanup errors remain observable.

### Completeness criteria

Precancellation, deadline and unsupported-platform paths; nested round trip;
existing/dangling names; component rejection; renamed anchors; replacement and
unknown-child preservation; ordinary Drop; independent concurrent allocations;
private modes; Store EX exclusion; finite forced collisions and CLOEXEC for all
owned/duplicated descriptors. Two compile-fail doctests enforce parent/lease
lifetime. Fifteen native methods on Linux/macOS; three portable on Windows.

### Quality standards

Pinned Rust1.96.0, full formatter/Clippy/native gates, reviewed unsafe calls and
scoped fixtures. Three compiling negative controls remove exclusive creation,
cleanup identity checking or the exact collision bound. All old topology,
destination, planned-root and emptiness controls and counts remain unchanged.
Native mandatory-name policy is strictly additive. Synthetic policy checks are
not native execution, and a compilation error cannot count as a causal failure.

### Invariants

No public destination/path escape, overwrite, recursive cleanup, foreign-domain
lease, daemon, retry-until-green, protection bypass, dependency or format change.
No claim that a successful reservation qualifies whole-tree output. All original
SI-01 criteria and G-FOUNDATION/G-ACTIVATION boundaries remain mandatory.

### Definition of Done

Review the exact source; all applicable full/native/causal Actions gates pass;
label coordinator self-review honestly; integrate only the expected head into
`fix/snapshot-integrity`; verify new parent CI and retire only the completed branch.
No merge to main or release. A draft with green subsets is not delivery closure.

## Initial local checks — not Rust qualification

Conversation Linux/Python3.13.5: support126 methods and CI-policy30 methods passed.
The existing suite was retained; six new policy methods cover registration,
synthetic absence/failure/ignore rejection and causal-seam uniqueness. Native Rust,
compile-fail examples and controls still require the candidate's Actions results.

Native references: POSIX mkdirat/openat/unlinkat descriptor-relative semantics;
Rust File::try_clone and File::drop ownership/error behavior. See
https://pubs.opengroup.org/onlinepubs/9799919799/functions/mkdir.html and
https://doc.rust-lang.org/std/fs/struct.File.html .

## Reconciliation after #174 — 2026-09-19

The candidate now includes accepted integration b06a621. Three textual conflicts
(dispatch, required-name policy and the control runner) were resolved without
changing Rust. All prior native source/tests, the ten original topology controls,
two emptiness controls and three descendant controls remain. Three scratch
controls are additive: 18 total. Original20/destination11/planned17/empty12 and
descendant13 Linux selections keep their counts; scratch15 runs separately both
before and after the controls. A portable regression guards this composition.
The two compile-fail lifetime examples remain mandatory. Previous branch CI is
historical; the reconciled tree requires its own local and Actions qualification.

### Reconciled local evidence

Owner's Intel Mac, Rust1.96.0, isolated source/build target, denied warnings and
configured Swift rpath: Store302 passed (zero failed/ignored/filtered), eleven
doctests including the two new lifetime compile-fail examples, Clippy all Store
targets and rustfmt passed. Python3.14.5: support132 and CI-policy30 passed.
Removing either the descendant or scratch family from the composed control loop
parses but fails the new composition assertion; both pristine and restored pass.
Those two portable controls are not additional native executions. Rust files and
previous test bodies from both branches are unchanged by the reconciliation.
One initial grouped test invocation was blocked before execution; the subsequent
individual test commands produced the actual results above. No blocked command
was counted as a test. New-head Actions remain required before merge.

Local raw logs are retained outside the repository. SHA256:

- `store.log`: `bb81848311cad456906e7ce5dff6f77744ce4283bced468078d7116069d35980`
- `doctests.log`: `297b90233c7986031116f4a02d3ea3997db88eeb667250589f542c8f1fbd64ef`
- `support.log`: `8dab7c3548e8f5763e6d89fd61736d860bafde314a48b02d7dc7ccb426db7fe3`
- `ci-policy.log`: `2265aea600bb843b4298c38ac8063f7e6b4c3714c94186b9393f4fb012ec10ef`
- `clippy.log`: `04a5dd030834268f7dcb661e8764c4134c531267f96009ae6f117b0526cef073`
- `composition-controls.log`: `1f5e884daa2f94706ed337c9da90dd9d0da78596cf4b6853bc4beab495162d30`
