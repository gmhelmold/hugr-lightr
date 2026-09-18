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
capture/hash checks. O_RDONLY restricts data I/O; the exposed File is not a
metadata-immutability sandbox. Callers remain responsible for their own methods
and must not treat this observation as authorization to change attributes. It does not verify protection-inventory completeness or
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
cancellation/deadline failure. Seventeen Linux/macOS methods inspect actual local
filesystems; their names are required only on the corresponding native profiles.
The evidence validator gains an additive per-profile requirement map; shared
required names, native host identities, existing ignores and prior tests remain.
Four Python policy methods reject absent, ignored, failed, duplicated or malformed
profile requirements using synthetic receipts. These are not native executions.

Four compiled causal controls remove protected overlap, paired overlap,
identity revalidation or the unresolved-parent rejection. The bounded harness
requires a pristine/restored 20-test suite and exact failing assertions from
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

## Reconciliation with the accepted estimator — 2026-09-18

The resumed coordinator review uses a separate clean worktree. The existing
source d1df4c896b24aedcfaf43a74c8a86b6ffac3cddd was merged with accepted #166
base 7b3c802d2baac121bcf59320ad6a17df50e7d063 as local candidate
f8eeabd90f56cda62a5110e88e14f2dbb3048820. The only conflict was DISPATCH.md;
its current-state resolution keeps both deliveries and their separate limits.
Every Rust byte remains equal to d1df4c8; all three estimator files remain equal
to #166. Cargo, full CI, the technical plan, prior required names and allowed
historical ignores are unchanged. The original source was not authored by this
continuation; this is coordinator reconciliation and self-review, not an
independent human approval.

On the owner's Intel Mac in /tmp/lightr-si01-topology-review-4vu5_644/repo,
Rust 1.96.0, two build jobs, denied warnings, isolated target, debug info and
incremental disabled: full Store 208 passed / zero failed / zero ignored;
all-target Store Clippy, formatting and three existing doctests passed.
Python support passed 93 methods (including all 39 estimator methods and four
topology-policy methods), and CI-policy passed 13. These totals overlap other
suite results; this reconciliation adds no new Rust test methods.

Four controls each compiled and failed the named assertion, followed by the
restored 20-method topology selection passing. Raw logs and source snapshots
were preserved separately from the clean worktree. Exact log hashes:

| Check | SHA-256 |
|---|---|
| format | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| support | `75a18cce53165b40d1eb29da57bd83e9f5a8f782f7fea34c85b0cefc9e613f72` |
| ci-policy | `2bff9fccf1f1d1feb262ca9172c557c3ac27454fd479aba29885878f692e942d` |
| store | `94e3c7bc31490067f292d22983774b359b1dea5a503fc4a7668ef1f3dfdacdc8` |
| clippy | `66ccd5d73d419bebd813c7926dc0647f0e5c80af7a3a327ec6b26819f3dbfd76` |
| doctests | `fc953c7bacb05b0a87b8fb74fc69b2d6b1b06b92c627ce6e13c129c4cabde125` |
| topology-controls | `c7dc38861f5a113c988adb34f812e9727cd658cfb7969bdb4dcf7803d7245c60` |

Execution-input fingerprint: `32e177b818fa6fb0cf30fe62943d3dd510f5a4143bc507ee96690bfc9030e0e2`.
This receipt changes no executable inputs. Remote checks must still execute on
the published candidate and its current integration base. The live #165 review
records exact remote runs and the final disposition; the earlier 09ada62/d1df4c8
observations are not relabeled as current-head CI. Do not infer full C12,
Windows implementation, E18, G-FOUNDATION or activation from this increment.
