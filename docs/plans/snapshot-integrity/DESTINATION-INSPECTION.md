# SI-01: destination-only observation before materialization

Date: 2026-09-18. Package #147; campaign #152; integration #146.
Base: `0d9d5069a9a518ad37c36c9d5c141c3f7bf1aa79`, reviewed topology #165.
Contract: C12 of v2.2 at `e30811971f6a1c2a1e2fdb0a91dc8ccf9a06879d`, Accepted ADR-0020.

## Problem and boundary

C12 distinguishes a paired live-source/live-destination operation from materializing
an already stored snapshot. The existing TopologyInspection requires a live source.
A materializer must not invent one from a historical source name or from the Store.

DestinationInspection accepts only the actual destination and the complete
caller-supplied protected-root inventory. It reuses the existing native resolver,
identity/ancestry checks, bounded handles, absent-suffix rules and revalidation.
There is no extra normalizer, new filesystem syscall, mutable Store operation,
source read, output creation, lock allocation or public-path routing change.

For an existing target, existing_directory() borrows the exact retained directory
handle. For an absent target, it returns None: the internally retained nearest
existing ancestor is not exposed as though it were the requested output directory.
An observation remains an observation; revalidate does not refresh an old missing
observation into a newly created target. Obtain a new inspection for the new state.
The original source-only and paired TopologyInspection APIs remain unchanged.

This is a prerequisite to destination protection, NOT the writable capability.
It does not enforce the empty-output policy, prevent namespace moves, freeze
contents, borrow a Store lease, create missing roots, protect recursive writes or
qualify resource recovery. Holding a read-only File is not metadata immutability.
No materialization may be authorized from this object alone. The later anchored
writer must retain its real resource guard and use the same validated target.

Windows still rejects native inspection explicitly. Linux/macOS retain exactly
the supported subset and limits from #165: conservative filesystem/mount checks,
existing protected roots, no inferred cross-mount equivalence, native alias and
parent semantics, and explicit rejection when the observation is unsupported.
Cancellation is cooperative; synchronous filesystem calls cannot be preempted.

## Five acceptance groups

**Success criteria:** a stored snapshot's destination is checked without a live
source; protected overlap is rejected, existing-target identity is retained and
an absent target cannot be confused with its existing ancestor.

**Completeness criteria:** platform disposition; cancellation/deadline at entry
and revalidation; equal/above/below/all protected roots and a harmless sibling;
existing nonempty output remains untouched; directory alias/parent semantics;
missing output under an alias; changed output and created missing suffix;
missing protected root; wrong output type; original source-only/paired behavior.

**Quality standards:** reuse the bounded native inspector; no new dependency,
native syscall, lint suppression, skip, workflow or storage-format change. Keep
Rust files below the repository size limit. Every added native method has an
independent required-name entry on its applicable profile; prior entries remain.

**Invariants:** no invented live source, raw ancestor returned as output, implicit
write permission, output deletion or creation, source-path reopening, different
Store authority, platform-success fiction or public protocol activation.

**Definition of Done:** review the exact diff; execute full CI and all applicable
native/bootstrap/causal gates; verify every destination witness, including the
three portable Unsupported/entry contracts on Windows. Record coordinator
self-review, merge with expected head to integration only, then verify fresh
parent #146 CI and retire only this delivery branch. #147/#152 stay open.

## Tests and controls

The new library selection is `store::foundation::destination_tests::`:
eleven methods on Linux/macOS, three portable contracts on Windows. They use
only disposable test fixtures. The existing twenty-method topology selection
retains its own namespace and count; none of those tests is removed or replaced.
The native evidence policy retains every prior requirement and appends the new
profile-appropriate names. The topology-policy registration test checks its own
prefix, not exclusive ownership of the shared profile requirement list.

Five new Python methods check the native registration, a complete synthetic
receipt and every absent, ignored or failed required name. These fixtures are
validator tests, not native filesystem observations. The topology causal harness
retains its four existing variants and adds three destination cases: protected
overlap, ancestor-as-target confusion and omitted destination revalidation.
Each variant must compile, fail its exact named assertion, and restore cleanly;
both original and destination pristine/restored selections must pass.

Reproduce on a supported host with the repository's pinned compiler:

```sh
cargo +1.96.0 test --locked -p lightr-store --lib store::foundation::destination_tests::
cargo +1.96.0 test --locked -p lightr-store --lib
cargo +1.96.0 clippy --locked -p lightr-store --all-targets -- -D warnings
cargo +1.96.0 fmt --all -- --check
python3 -m unittest discover -s scripts/si00 -p 'test_*.py' -v
python3 scripts/si01/topology_controls.py --out /absolute/new/evidence-directory
```

The initial authoring check ran 98 Python support methods in the isolated Linux
conversation environment; CI-policy checks and Rust qualification are recorded
in the live delivery PR for the exact candidate, not inferred from the base.
The remote-terminal inspection was rejected before execution; no Mac build is
claimed from that attempt and no permissions were changed.

G-FOUNDATION/G-ACTIVATION remain unestablished. Complete C12, the anchored writer,
recursive traversal, Windows native inspection, actual resource measurements,
E18, domain-guarded reaping and typed consumers remain mandatory under #147.
