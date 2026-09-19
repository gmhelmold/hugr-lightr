# SI-01: static Windows link kinds and a native creation witness

Date: 2026-09-18. Package #147; campaign #152; integration #146.
Base: `2b912812a35aa1d866a89843821ed3ff7fad2af3` (#168).
Contract: C07/C12 in v2.2 and Accepted ADR-0020. No format or activation change.

## Purpose and boundary

LMF1 retains symbolic-link text, not a native file-versus-directory flag.
WindowsLinkPlan borrows an already validated TreePlan and derives each target's
kind from the captured tree only. It does not inspect a later live path or guess
from the permissions of the process. It is the static Windows representation
portion of the materializer, not the native whole-tree collision check or writer.

All entries first pass the conservative Win32 component syntax check. Each link
then resolves exact forward-slash relative components through represented
explicit/implied directories. Dot and parent components affect lookup only;
stored path/target text remains byte-identical. The captured root is a known
directory. Traversing any link, missing intermediate component, non-directory
component or parent outside the captured root is rejected. Chains and resolution
cycles never receive a guessed kind, including a link immediately followed by
parent traversal. No native public-path normalizer is used.

This subset rejects empty target components (including repeated/trailing slashes),
backslashes, drives/devices/absolute targets, Win32 forbidden characters/control
characters, trailing spaces/periods and reserved device basenames/extensions.
COM0/LPT0, CONIN$/CONOUT$ and space-suffixed reserved stems are conservatively
excluded too. The original logical TreePlan still preserves valid POSIX literal
characters; nothing globally renames or strips them.

A successful plan is NOT full UnsupportedRepresentation/UnsupportedCapability
qualification: case, Unicode and 8.3 aliases, actual filesystem length limits,
link privileges, source native link-kind agreement, topology, complete protected
inventory, leases and anchored writes remain separate. Case-distinct logical
entries may pass this static plan and still collide on a real destination.
Native collision preflight must reject that destination before output effects.
No current public hydrate, capture, CLI, Store or GC path is converted here.

## Interface and resources

WindowsLinkPlan::validate(&TreePlan, Wait) returns all links in original entry
order with borrowed path/target and File/Directory kind. The future writer must
schedule represented targets before links; input ordering is not publish order.
The plan retains an immutable borrow of TreePlan and its manifest. A static
failure has io::ErrorKind::Unsupported with downcastable UnsupportedRepresentation
(entry_index and RepresentationReason). Native capability errors are not produced
or swallowed by this pure API. Cancellation/deadline errors are returned unchanged.

TreePlan already enforces caller-selected entry, component and text budgets.
The additional entry-reference index and link vector use try_reserve_exact.
One temporary lookup String is bounded by path bytes + target bytes + 1 and
reserved fallibly before append operations. Lookup sorting and comparisons are
bounded work, not asynchronously preemptible. No decoder-memory, exact RSS or
survival-under-arbitrary-allocation-failure guarantee is inferred.

## Five acceptance groups

**Success criteria:** derive file/directory kind only from the whole immutable
captured tree; reject missing, indirect and out-of-tree resolution; retain original
link text and error attribution; never equate syntax/kind with write authority.

**Completeness criteria:** explicit/implied/root directories; files and unsorted
input; exact target text; relative parents; chains/cycles; missing/non-directory
intermediates; forbidden/reserved names; exact case/Unicode distinctions; empty
and long logical names; cancellation/deadline/checkpoints; native Windows flags
for both kinds including after target removal; preservation of earlier witnesses.

**Quality standards:** Rust 1.96.0, denied warnings, fallible bounded indices,
no new unsafe block, dependency, codec or native production syscall. Common
logical witnesses run on five profiles; the Windows-only creation witness is
mandatory on Windows, not a silently skipped capability test. Controls must
compile then fail their named assertions. New source files remain below400 lines.

**Invariants:** no live-target lookup, target rewriting, POSIX behavior change,
partial-plan return, guessed native alias/capability, user-output mutation,
missing lease substitution, relaxed prior test/gate or protocol activation.

**Definition of Done:** review exact diff and evidence; all current full/native/
causal gates pass; actual Windows creation is verified separately from Linux/Mac
execution of pure logic. Label coordinator self-review; expected-head integration
merge only; verify new parent #146 CI and retire this branch. #147/#152 stay open.

## Tests and evidence

Thirteen common Rust methods; one Windows-only native method; positive and
negative immutable-borrow doctests. The 24 entry permutations are cases within
one test, not 24 independent native runs. Five policy methods check registration
and complete/missing/ignored/failed synthetic receipts (42 negative cases).
Six compiled controls remove or alter file kind, missing-target refusal,
link traversal, root-parent bound, non-directory traversal and syntax rejection.

The native test creates only its own temporary file/directory and two links using
the derived kinds, verifies both links independently, removes the targets and
checks that native link kinds/text remain. This witnesses actual capability on
that runner, not a public materializer, every target syntax or whole-volume
name-collision qualification. A capability failure fails that witness explicitly.

```sh
cargo +1.96.0 test --locked -p lightr-store --lib store::foundation::windows_link_plan::
cargo +1.96.0 test --locked -p lightr-store --doc
cargo +1.96.0 clippy --locked -p lightr-store --all-targets -- -D warnings
python3 -m unittest discover -s scripts/si00 -p 'test_*.py' -v
python3 scripts/si01/windows_link_controls.py --out /absolute/new/evidence-directory
```

Current results belong to the exact-candidate PR/review. Writing this contract
is not native qualification. G-FOUNDATION/G-ACTIVATION remain unestablished.

## Primary references

- [Microsoft Win32 naming conventions](https://learn.microsoft.com/en-us/windows/win32/fileio/naming-a-file): component restrictions, device names, case and short-name caveats.
- [Rust symlink_file](https://doc.rust-lang.org/std/os/windows/fs/fn.symlink_file.html) and [symlink_dir](https://doc.rust-lang.org/std/os/windows/fs/fn.symlink_dir.html): distinct native kinds and capability limitations.
- [Accepted execution contract](../SNAPSHOT-INTEGRITY-REMEDIATION.md), C07 Windows link subset; public-path C12 resolution is a separate operation.
