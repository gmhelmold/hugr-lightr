# SI-01 read-only topology inspection

Date: 2026-09-18. Campaign #152, package #147, integration #146.
Base: `90ce0dc12ab0af43bdc51b13a0aa7d5863e74c1d` (reviewed CoW increment #164).
Contract: v2.2 at `e30811971f6a1c2a1e2fdb0a91dc8ccf9a06879d`, C12 and Accepted ADR-0020.

## Implemented boundary

`TopologyInspection::inspect` is **read-only observation**, not the final
`ValidatedTopology` mutation capability. It accepts a complete caller-supplied
inventory of existing protected directories, an existing source with explicit
file/directory intent, and an optional existing/proposed directory destination.
It checks source/destination versus every protected root and their mutual
relationship. The inspector never creates a destination or a Store, unlinks,
changes permissions, initializes caches, follows user commands or adopts data.

Linux/macOS use opened directory descriptors, `openat` and native ancestry.
Dot and parent components are processed by the kernel in order, not lexically
collapsed across an alias. Trailing slash/dot directory requirements remain.
The returned read-only source handle denotes the observed object; it is not
reopened by pathname. Revalidation checks identities, ancestry and the exact
missing suffix from a pinned working-directory handle, without changing process
working directory. All handles have RAII ownership and native opens use CLOEXEC.

This is NOT a namespace lock: handles do not prevent directory moves, and two
matching observations cannot exclude a later mutation. The inspector therefore
does not authorize writes, recursive traversal, missing-destination creation or
public protocol activation. Those adapters still need their own anchored use
and resource/lifetime protection. Contents may change and need the existing
capture/hash checks. It does not verify protection-inventory completeness or
classify overlapping Store/cache ownership domains on the caller's behalf.

## Conservative supported subset and explicit rejection

- Linux accepts the ext, XFS, Btrfs, tmpfs and overlay filesystem identifiers for
  this read-only inspector. Native mount ID must be returned by statx; missing
  information/errors are not replaced by guessed identity. Those allowed
  families are not a claim of a native witness on every filesystem/version.
- macOS accepts APFS identity inspection. File-system type is read using fstatfs.
- A participant on a different device/mount from a protected root is rejected
  with Unsupported after overlap checks. This deliberately does not attempt to
  infer cross-bind-mount equivalence or support network/unknown filesystems.
- **Windows returns Unsupported before resolving paths.** It is not a portable
  lexical fallback and does not change any existing Windows product route.
- Directory aliases use native resolution. Final regular-file symbolic links,
  multiply linked/unlinked regular files, unresolved dot/parent segments after
  a missing ancestor and dangling destination links are rejected, not guessed.
- Proposed missing names must be representable by this profile: invalid UTF-8
  suffixes are explicitly rejected on APFS, not accepted as creatable names.
- Empty/NUL paths, exactly two leading slashes, paths above 4096 bytes, more than
  128 components/ancestors and inventories outside 1..32 roots fail explicitly.
  Each resolution pass budgets 512 retained handles, with two observations held
  during revalidation; native allocation/descriptor errors remain real errors.
- Cancellation/deadlines are cooperative around native operations and loops.
  They cannot preempt a synchronous filesystem call already in progress.

Protected roots must exist: pre-initialization handling of missing protected
roots, Windows native inspection, writable-destination anchoring, recursive
traversal and complete C12/native topology qualification are NOT delivered here.
The existing live-file capture helper and public Store/CLI/GC routes are unchanged.

## Five acceptance groups for this increment

**Success criteria:** reject protected/paired overlap using opened identity and
ancestry; retain native alias/parent semantics; detect changed observations;
never create a proposed output or return a write/publication capability.

**Completeness criteria:** equal/above/below and harmless sibling names; all
configured roots; directory aliases, alias-parent and dot controls; trailing
slash on files; absent output under an alias; unresolved/dangling suffixes;
source replacement, read-only handles, regular-file alias refusal; native name
representation, bounds, absent sources and cancellation/deadline entry checks.

**Quality standards:** pinned Rust 1.96.0, bounded resources, original OS errors,
small audited unsafe syscall wrappers, no dependency or format change. Native
and unsupported profile outcomes must be separate; tests use disposable owned
fixtures, not live Store or user files. No timing-based race assertions.

**Invariants:** no mutations by inspection, no lexical-normalization approval,
no source pathname reopen, no assumed mount identity, no opaque proof minted
from existence, no test-policy/branch-protection relaxation and no activation.

**DoD:** exact committed candidate passes full CI, native evidence and causal
controls; explicitly labeled coordinator self-review checks the scoped limits;
expected-head merge to integration only, followed by fresh parent #146 CI.
No missing platform or later C12 obligation is silently marked complete.

## Tests and evidence collection

Three portable methods require explicit platform disposition and entry
cancellation/deadline failure. Sixteen Linux/macOS methods inspect actual local
filesystems; their names are required only on the corresponding native profiles.
The evidence validator gains an additive per-profile requirement map; shared
required names, native host identities, existing ignores and prior tests remain.
Four Python policy methods reject absent, ignored, failed, duplicated or malformed
profile requirements using synthetic receipts. These are not native executions.

Four compiled causal controls remove protected overlap, paired overlap,
identity revalidation or the unresolved-parent rejection. The bounded harness
requires a pristine/restored 19-test suite and exact failing assertions from
successfully compiled variants. A compiler error/timeout/empty suite is failure,
never a causal witness. Linux CI runs that harness as an additional job.

Initial local diagnostics exposed an overly broad test assumption: APFS
rejected creation of a non-UTF8 filename with EILSEQ while native lookup returned
ENOENT. The corrected test compares like-for-like native LOOKUP outcomes and
requires explicit creation rejection, not a substituted valid filename or skip.
A separate test first failed because the inspector accepted an unrepresentable
missing APFS suffix; the implementation now rejects it. Initial failed logs
remain outside the source worktree with the local session evidence.

Reproduce on a supported native host:

```sh
cargo +1.96.0 test --locked -p lightr-store --lib store::foundation::topology:: -- --nocapture
cargo +1.96.0 clippy --locked -p lightr-store --all-targets -- -D warnings
python3 -m unittest discover -s scripts/si00 -p 'test_*.py' -v
python3 scripts/si01/topology_controls.py --out /absolute/new/disposable/evidence-directory
```

Native reference contracts:
[POSIX pathname resolution](https://pubs.opengroup.org/onlinepubs/9799919799/basedefs/V1_chap04.html),
[Linux statx](https://man7.org/linux/man-pages/man2/statx.2.html),
[Apple fstatfs](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/statfs.2.html).
They explain syscall semantics, not empirical qualification of this repository.

`production_protocol_enabled=false`; G-FOUNDATION/G-ACTIVATION are not established.
#147 and #152 stay open; #146 stays draft. No main merge, release or migration.
