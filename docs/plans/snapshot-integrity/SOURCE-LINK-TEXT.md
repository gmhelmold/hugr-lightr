# Anchored POSIX source-link text — inactive C07/C12 prerequisite

Base: `a66dc86426160eacbf5c48fc997fda42c1677659` (#175). Part of #147/#152;
parent #146 remains draft. No public Store/capture/hydrate routing or codec change.

## Scope and platform boundary

`TopologyInspection::read_source_link(relative, max_target_bytes, wait)` reads
exact UTF-8 target text without following the final link. A dangling, absolute,
chained or protected-target spelling is text to preserve, not an object to open.
Normal relative path components use the existing no-follow descendant adapter;
4096 path bytes/128 components and a caller target bound of 1..=65535 apply.
A single read uses one extra byte to detect overflow/truncation, with no retry,
metadata-size trust or replacement-character decoding. Native errors propagate.

The original topology, retained parent identity and final name binding are checked
before/after reading. A handle keeps the link identity alive across checks. Linux
reads the O_PATH|O_NOFOLLOW link descriptor using readlinkat(fd, "", ...). Darwin
pins using O_SYMLINK and reads the single name from its retained parent, checking
its binding on both sides; O_SYMLINK|O_NOFOLLOW is not the Linux combination and
returned ELOOP/62 on the owner's Mac probe. O_CLOEXEC is atomic on both profiles.
Darwin additionally uses O_NONBLOCK|O_NOCTTY, and known nonlinks are rejected before
opening. The native descriptor is owned by File on every successful open.

This is a bounded observation, not a hostile namespace sandbox, atomic directory
snapshot, lease, prepared-byte proof, native Windows link-kind witness or output
writer. In particular, Darwin's binding checks do not make readlinkat and namespace
observation atomic against adversarial ABA changes. Subsequent changes are not
frozen. Reading may update filesystem access metadata. Windows retains explicit
Unsupported topology; the new method does not invent Windows support. Linux raw
source names can be addressed, but a later LMF1 capture must separately enforce its
UTF-8 path contract. Invalid UTF-8 target text is rejected, never rewritten.

## Five acceptance groups

### Success criteria

Preserve exact supported target text including dangling targets; never follow a
final link into payload/protected data; reject truncation and observed binding
changes. Existing public topology and descendant semantics stay unchanged.

### Completeness criteria

Existing file/directory and dangling/cyclic/absolute targets, intermediate-link
refusal, malformed paths and work bounds, exact byte limits, missing/wrong types,
file source root, root/parent/leaf replacement, owned result after deletion,
initial/terminal cancellation, scoped I/O errors, pinned symlink/CLOEXEC, and Linux
raw names/non-UTF-8 targets. Fourteen methods on Unix, plus two Linux methods.
No Linux-only method is claimed exercised on macOS or Windows.

### Quality standards

Pinned Rust 1.96.0; denied warnings; native calls reviewed individually; retained
ownership and bounded fallible allocation. Required native names are additive.
All eighteen preceding topology/empty/descendant/scratch controls and six suite
counts remain; four added controls must compile and fail named assertions before
restoration. Extend the exact family registry, never remove an old requirement.

### Invariants

No source/payload writes, live-target type guessing, silent text rewriting, broad
serialization, process-global injection, dependency/workflow/codec changes,
retry-until-green, ignored assertion, claimed namespace exclusion or activation.
All original SI-01 and campaign criteria remain mandatory.

### Definition of Done

Targeted and full Store/doctest/formatter/Clippy validation; exact-candidate full,
native and causal Actions; explicitly labeled coordinator self-review; expected-head
merge into integration; new parent CI verified; retire only this delivery branch.
Native whole-tree representation, output writing, enumeration/reaping and typed
consumers remain. G-FOUNDATION/G-ACTIVATION are not established by this increment.

## Local verification — not Actions qualification

Owner's Intel Mac (macOS-15.3.2-x86_64-i386-64bit-Mach-O), Rust1.96.0, Python3.14.5,
owned isolated clone/target. Full Store:316 passed, zero failed/ignored/filtered;
all11 existing doctests, Clippy for all Store targets and formatting passed.
Support138 and CI-policy30 methods passed. Counts overlap;14 Unix link-text methods
are new, and the two Linux-only cases require native Actions.

Three native Mac controls compiled, then failed their exact leaf-identity,
byte-budget and terminal-cancellation assertions (Cargo101). Restored14-method
suite passed. The Linux O_PATH/no-follow control remains for Actions, not inferred
from macOS compilation. No native call failure or compile failure was counted as
a causal assertion. Prior policy equality and all18 earlier control tuples were
checked against the base; the source-link registry adds4 controls and16 Linux names.

Raw logs are retained outside the checkout under the owned execution directory.

| Check | Exit | SHA256 |
|---|---|---|
| store | 0 | `ede0eb798204e46ace885a5e9ad7f1cd0acad2746fbc002775d885027f7b7bdc` |
| doctests | 0 | `124a2f38135fee6fca70643101e346352b11c1ba8045345afd2fb02dae6c911e` |
| clippy | 0 | `c0e9be4696bcd27e426a4d0049ceb18ff4f547ba73d4aec1ffb51cf41f0a2339` |
| format | 0 | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| support | 0 | `8867309e85a52c57d93c051d198205424194e5ec4729a5910541b58f771735ea` |
| ci-policy | 0 | `e89803b92f11c7fe4b59324b448594ce99577ad545bd191475b4b7813e5bccd8` |
| link-leaf-identity-build | 0 | `b045e9f265bad71b158c1cb3dc09b9d3d6ce5a4fac15798cf6e93a0e91cdbd0e` |
| link-leaf-identity-test | 101 | `1294f21ce16a5a29bdc6a2fdd3862cb05c70947af45543e42d1d3a9f2c969a25` |
| link-byte-bound-build | 0 | `bc9c2f67aa275e1918047284a9bb120dc4684860fbf27d04fa70ca4414763f2b` |
| link-byte-bound-test | 101 | `d0024e1f29fbefe085c0fadfa80399320d62650a0e67d2b30b165c49a66ca788` |
| link-final-cancellation-build | 0 | `53e0af81297ed922293145d308ca19465491afaa4e878ce7a3633136cce0d6c4` |
| link-final-cancellation-test | 101 | `19a7d846c5ebc6d7eab5fec90f4c1d1657cab6599077605aaf21a1d583dcef90` |
| links-restored | 0 | `851e6058104ab81a3680b41aa7470ebcff677ae81c42514b2e6c007754e5410b` |

Exact-candidate full/native/causal Actions and reviewed integration remain required.
No claim of merge qualification is made solely from these local results.

## Native references

- Linux readlinkat empty-path and truncation contract:
  https://man7.org/linux/man-pages/man2/readlink.2.html
- Linux O_PATH|O_NOFOLLOW and descriptor ownership:
  https://man7.org/linux/man-pages/man2/open.2.html
- Apple O_SYMLINK selects the link itself:
  https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/open.2.html
