# C12 destination-emptiness observation

Date: 2026-09-19. Base408dd9436ef05a612dfec4cd8bea7948e6bf767d;
SI-01 #147 / campaign #152 / integration #146. An inactive prerequisite,
not the anchored writer and not final C12 or G-FOUNDATION acceptance.

## Implementation and boundaries

`DestinationInspection::require_empty(wait)` revalidates topology before and
after a handle-grounded directory scan. An absent destination is neither created
nor adopted after appearance. Existing inspect/revalidate contracts remain.
Only `.` and `..` are omitted. Hidden files, directories, dangling links and raw
non-UTF-8 names count as occupied without following or deleting their entries.
Occupancy preserves native ENOTEMPTY; readdir distinguishes EOF from errno.

The scan opens `.` relative to the retained directory with the existing open_at
helper. This creates an independent open-file-description offset; duplication
alone shares a possibly exhausted offset. fdopendir takes ownership only after
success. All normal paths call checked closedir; Drop handles unwinding. A scan
or cancellation error takes precedence if close also fails. A close error after
successful scanning is returned. No close retry or independent fd close occurs.

At most three logical entries are examined: an empty stream may contain only
`.` and `..` before EOF. Repeating dots fail rather than loop. Cancellation checks
bracket each read, including EOF; blocked synchronous native calls cannot be
asynchronously interrupted. Directory access-time updates are possible.

This is an observation, not namespace exclusion or write authority. Concurrent
insertions remain possible. The future writer must hold its own guards and use
exclusive anchored creation. No public Store/hydrate/CLI/GC route is changed;
Windows native topology remains Unsupported. No new dependency or workflow.
Planned-root work in #172 must be preserved when reconciling the integration base.

## Success Criteria

Reject an occupied observed destination without modifying its entries. Repeated
checks must not inherit a previous EOF offset. Missing targets remain missing;
changed topology fails without replacing the retained handle.

## Completeness Criteria

Existing/absent targets, hidden/regular/directory/dangling entries, independent
offsets, content changes, replacement/adoption, native alias-parent resolution,
cancellation/deadline/EOF, read errors and bounded dot streams. Nine new methods
are mandatory on every Linux/macOS profile; a tenth non-UTF-8 case is Linux-only.
No native Windows success is inferred from compilation. All old cases remain.

## Quality Standards

Rust1.96.0, formatter, deny-warnings Clippy, native Store suites and exact-head
Actions. Unsafe ownership/errno paths receive explicit review. Isolated fixtures
use no process-global failure hooks. The existing seven topology controls stay
required; two added mutants must compile and fail their exact new assertions:
shared offset, and omitted EOF checkpoint. Original topology selection is scoped
to its own tests module, preserving20 cases instead of absorbing the new family.
The new family gets separate pristine/restored10-case Linux runs. A Python test
pins that separation and all registered native names.

## Invariants

No public activation, codec/dependency changes, skipped failures, inferred empty
success, root exemptions, parent-handle substitution, user-data cleanup or claim
that observation freezes concurrent contents. All accepted parent criteria remain.

## Definition of Done

Pass exact-candidate full/native/causal Actions, with all required methods present.
Record coordinator self-review explicitly. Merge only the expected reviewed head
into fix/snapshot-integrity; verify new parent146 CI and retire only this branch.
Main unchanged; #147/#152 open; #146 draft; production_protocol_enabled=false.

## Executed local checks — not Actions qualification

Owner's Intel Mac, Rust1.96.0, isolated checkout and target directory:
Store256 passed, zero failed/ignored/filtered; all8 existing doctests and Clippy
all Store targets passed. The first format check rejected layout in the new test
file only; rustfmt corrected it before publication without altering assertions.
These totals overlap and are not256 new tests. Native Linux and Windows, full
workspace and compiled controls remain Actions obligations before integration.

Native/tool outcomes and hashes from this continuation:
- format-new-source: exit0; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`.
- native-store: exit0; SHA256 `82637ef7cf1121969094aca2ea9a6f3f99c5a991679dcd3b278f597519bbe403`.
- store-clippy: exit0; SHA256 `935978b9081cbbaf06b219f206b050986688ebe527f9af5755a904a3e205c02e`.
- store-doctests: exit0; SHA256 `4d784ede6acb57d9325d45fdb153069206f55f09acde616c6398c4d512e235a5`.

Historical conversation-Linux C primitive experiments and Python policy results
remain in the prior candidate attachment; they are not new Rust runs or proof
of macOS behavior. Publication/qualification status is recorded in the delivery
PR, not inferred from the preceding integration's green badge.
