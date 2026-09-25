# C07/C12 — complete logical-name probe in managed scratch

Base: `deae93539a1ae0bd14b1c7f088f9159df5480341` (accepted #178).
Part of SI-01 #147 / campaign #152; parent #146 remains draft.
Design authority: accepted ADR-0020 and the existing C07/C12 owned-probe contract.
This increment is inactive: it does not connect public capture/hydrate routing.

## Scope and result

`probe_scratch_names(TreePlan, StoreLease, NameProbeLimits, Wait)` exercises all
explicit/implicit directory names and non-directory names inside a new private
reservation selected by the existing Store lease. It never accepts an arbitrary
output pathname, creates an additional lock domain, reads payloads or changes
LMF1. The immutable TreePlan remains the logical prerequisite.

Directories reproduce the hierarchy. Each regular-file OR symlink name uses an
empty regular-file marker. Markers test leaf-name coexistence only, NOT file
contents, permissions, link target text, native link kinds or link capability.
No target is interpreted, followed or silently recoded. In each parent, ALL
sibling names coexist before descending or removing any sibling. Native exclusive
creation detects collisions rather than a guessed case-folding/Unicode rule.
The traversal visits every planned node and preserves native failure causes.

`ScratchNameObservation` contains counts for this completed, cleaned reservation;
it is not a writable capability, PreparedObject, destination certificate or
resource reservation. In particular, a private staging directory and a public
destination can have different name policies even on the same filesystem. A
future actual-destination probe MUST establish its own applicable native context;
it may not turn this observation into a transferable success token. Windows
returns Unsupported through the existing scratch contract, including empty plans.

## Bounds and lifetime

Before scratch allocation, validate the complete plan against caller node/depth
budgets and the implementation ceilings (4096 nodes, depth 64). Node counts include
implicit directories, but not the reservation root. Zero budgets accept only an
empty tree. These are work limits, NOT measured OS descriptor/storage capacity.
The name index uses fallible allocation; native resource failures remain errors.
Sibling-range lookups use the sorted parent/name index, not repeated pathname
resolution. Borrowed scratch children retain their parent and original Store lease.

Checkpoints surround bounded reservation/creation, traversal and terminal cleanup;
they do not interrupt a blocked syscall. Each successful allocation is retained
BEFORE the post-allocation checkpoint. Scratch's bounded internal reservation uses
Wait::Try, with the outer operation observing cancellation before/after the call.
Native calls and ownership enforcement are reused without new unsafe code.

## Failure and cleanup

Stop creating after the first operation failure. Explicitly finish acquired nodes
in reverse sibling order and children before their parent. Even after cancellation,
perform cleanup, then return failure. Preserve the primary io::Error independently
from every explicit cleanup error. Cleanup errors alone prohibit success. A vector
sized before allocation holds at most one error per acquired node plus the root.
Unknown/replaced names follow existing checked, nonrecursive scratch cleanup and
are never recursively removed. A failed nested cleanup stops further traversal.

This composition does not upgrade ScratchDirectory's failure guarantees: a failed
low-level creation, process death or panic can still leave owned scratch; native
File::drop does not report descriptor-close errors. No hostile private-directory
writer exclusion, reaper, all-or-nothing public output or crash durability is
claimed. Stable private Store roots remain a prerequisite. Final resource-domain
reaping and actual-destination qualification remain separate campaign obligations.

## Five acceptance groups

### Success criteria

Exercise every planned directory/leaf name with exclusive native creation; detect
actual sibling conflicts; never accept after operation/cancellation/cleanup error.
Keep names, manifest, caller data and public storage routing unchanged.

### Completeness criteria

Empty tree/platform contract; explicit and implied parents; sibling coexistence;
native case/Unicode observations; identical names in distinct parents; complete
preallocation bounds; late native failure; cancellation after creation/cleanup;
primary plus cleanup errors; cleanup-only errors; unknown/replaced-entry retention.
Four common and ten Unix methods are registered in their native profile policies.
Leaf markers are explicitly distinguished from actual link creation or payloads.

### Quality standards

Rust 1.96.0, denied warnings, formatter, complete Store/doctests, support/CI tests,
exact-head full/native Actions, and explicit review. Preserve every old test and
native requirement. The six structural causal tuples remain unchanged; three
new compiled controls cover depth budgets, final cancellation and cleanup failure.
Both original 15-method and new 14-method Linux selections run before/after controls.

### Invariants

No arbitrary public path, new syscall/unsafe/dependency, source or payload writes,
recursive foreign cleanup, lossy name normalization, inferred destination policy,
lease reacquisition, protocol/codec/activation change, gate weakening or main merge.
All accepted WP criteria and the five groups are cumulative, not replaced by CI.

### Definition of Done

Publish reviewed source on the current integration, qualify its own full/native/
causal Actions, label coordinator self-review honestly, merge with expected-head
verification, verify NEW parent CI, then retire only this delivery branch and its
owned clean checkout. SI-01/G-FOUNDATION/G-ACTIVATION remain unestablished.

## Evidence

The current delivery PR and its exact Actions runs own qualification. Local
native and synthetic results are recorded separately before integration. No
previous head's green check qualifies this source; counts overlap across suites.

## Local qualification — owner Intel Mac

macOS 15.3.2 (24D81), Rust 1.96.0, Python 3.14.5; isolated owned clone/target.
Store library: 330 passed, zero failed/ignored/filtered; new name family: 14 passed.
All 11 existing doctests, Clippy all Store targets with denied warnings and
format check passed. Support: 178 passed plus one existing Linux-only skip
(179 methods); CI policy: 30 passed. Counts overlap; no Windows native-name
support or actual-destination qualification is inferred from Mac execution.

Three new negative controls each compiled, failed its exact assertion with
Cargo 101, then passed after restoration. The restored 14-method family passed.
The existing six structural controls are byte-equivalent in their tuple registry
and must also run in this candidate's Actions. The sibling-coexistence native
regression passed and remains mandatory. An optional fourth local mutant command
was blocked before execution, changed nothing, and is not counted as evidence
or included in the three-control registry. No failed test was suppressed.

Raw local logs remain outside the checkout. Selected SHA256 values:

- `format.log`: `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`.
- `store.log`: `79179bd50b6dcf8a4c450060185938989c560bb5cfb3d65ebeb82ad658891129`.
- `clippy.log`: `52e020707a2a63a93aac332b60185ac355242a9e5bd6dd09b10130729906a92e`.
- `doctests.log`: `074a5c0d5d0a0b926a53a14f2c0e07e79f0d6a6ddc0164604d587c88f4aabb76`.
- `ci-policy.log`: `86aee1c40d2da652ab0ef080c7861542433369b9da392cdc0ff4a6cf67938a87`.
- `name-probe-depth-build.log`: `4744651cd250aae03a24aa4a35360f491b2a0baf88e033bced51036b072b0a1f`.
- `name-probe-depth-test.log`: `cafdc2058ea379403c02bbfde658d778c03c60924c8a59753617539e2db66b6d`.
- `name-probe-depth-restored.log`: `b19c9baf0b18f4ed88bc40a05144df4129e75569eb9337ffe55f66a6b24a21e9`.
- `name-probe-terminal-cancel-build.log`: `b5337f368cfa01f43e5e0452736e65746c45142fe07023be406693086bc15632`.
- `name-probe-terminal-cancel-test.log`: `f1cac06605b15f7588517b8f65fed7e6a182b81e5fb5bcf38c7b3eae2f446c52`.
- `name-probe-terminal-cancel-restored.log`: `e6de2d519a5c442264b3944219a9dcca9a7259bcc0af6621ca467e0706ea2f20`.
- `name-probe-cleanup-build.log`: `c12159aa55b4275182f38b727f9ca2ad39529a0d46eb7bfe58b39da257d0b030`.
- `name-probe-cleanup-test.log`: `95f6d81145a2008583e2070d8aa9a056a7ce1f2974b4ca28e7dc489a77b3505a`.
- `name-probe-cleanup-restored.log`: `5fbc219e4daddeaf18fa858d224666613fba64cd2d8a9be4df64b74b2bbe14a3`.
- `name-probe-restored.log`: `d726737ab31c4acd8e705277ba6b4f427715c844b1e876b68b522922797e6c8e`.

## Composition with accepted source listing #180

Reconciled against `aac0e8174ca6a8076bb62cc61a0c67302af83f46` by an ordinary
merge, without rewriting either branch. Only DISPATCH and the native required-name
registry conflicted. Both Rust implementations and all old test bodies remain
byte-identical; removing only scratch-name requirements reproduces the accepted
#180 policy exactly. The structural/name control runner (6 + 3 controls) and the
accepted topology runner (26 controls) remain unchanged and run separately.

On the owner Intel Mac / Python 3.14.5, the combined support suite ran 185 methods:
184 passed and one existing Linux-only case was skipped. All 30 CI-policy methods
and pinned Rust 1.96.0 formatting passed. Full Store/doctest/Clippy execution and
exact-head Actions qualification are tracked in the PR; no older candidate green
qualifies this composition. Public routing/main and the five acceptance groups
above remain unchanged. Exact results and review belong to the current PR head.
