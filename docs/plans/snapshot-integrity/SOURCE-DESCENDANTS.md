# C12: anchored source-descendant acquisition

Date: 2026-09-19. Base integration: `671e3b7fec267fbf8ba5e015da7de680b7ddd709`
(accepted destination-emptiness #173). SI-01 #147 / campaign #152; ADR-0020.
Inactive additive adapter: no public capture, hydrate, Store or GC routing changes.
The delivery PR records the exact candidate, Actions results and reviewed merge.

## Scope and native boundary

`TopologyInspection::open_source_descendant(SourcePath, Wait)` accepts only
normal relative components beneath an already-inspected source DIRECTORY.
It does not reinterpret a stored snapshot as a live source. Unlike initial
public-path resolution, this descendant grammar rejects absolute, empty, dot,
parent, repeated-separator and trailing-slash paths. Byte spellings are retained;
there is no UTF-8 conversion on Linux and no backslash/colon normalization on Unix.

Each component is opened relative to its retained parent with O_NOFOLLOW,
O_CLOEXEC and O_NONBLOCK; directory components also require O_DIRECTORY.
Both intermediate and leaf symbolic links are rejected, not followed. This does
not remove link support from the public product: link-text capture is a separate
adapter and public routing is unchanged. Files must be regular and singly linked.
Native mount identity must match the source; cross-mount acquisition is rejected.
Existing qualified topology filesystem restrictions continue to apply.

The original inspection is revalidated before and after acquisition. Retained
component identities are checked again from their retained parents. Reopening
never uses the global descendant pathname. Every successful call returns a fresh
read-only File with an independent file offset. Later renaming/replacing a name
does not redirect that File to new contents; contents of the same inode are not
frozen. O_RDONLY does not forbid all metadata operations exposed by File.

This is not a lease, an immutable payload proof, a namespace exclusion guard,
a full directory walk, native whole-tree representation, a writable destination,
or confinement against hostile concurrent directory mutation. Ordinary observed
changes fail; no finite observation freezes changes after its last checkpoint.
No data is read from a candidate before its type is checked. Opening can have
native effects (including device-open effects under unsupported hostile races),
and reads can update access metadata; this is not a privileged sandbox boundary.

## Bounds, ownership and errors

Reject paths longer than4096 bytes or deeper than128 components. The retained
component vector uses fallible exact reservation after checking those bounds;
there is no payload or whole-tree buffer. The normalizer is not reused as an
independent test oracle. Source identity/type and all descriptor ownership are
checked with native handles. Partial acquisitions close through File ownership.

Wait checks bracket acquisition and revalidation steps. They are cooperative,
not asynchronous interruption of a synchronous native call. No sleeps, retry
loop, mutable global test hook, cwd change, Store initialization or lock acquisition
is added. Existing user bytes and authoritative roots are never written or cleaned.
Native I/O errors remain intact; custom invalid-shape/type and unsupported-profile
errors remain explicit. The implementation does not certify mount aliases that
its existing native identity primitive cannot establish.

## Five acceptance groups

### Success criteria

Acquire the requested supported regular file/directory via retained parents,
without following links or returning a substituted entry. Returned identity and
contents match an independently opened fixture. Preserve all earlier APIs.

### Completeness criteria

Nested files/directories, independent offsets, exact names, intermediate/leaf/
dangling links, malformed paths and work bounds, missing and wrong types,
multiply linked files, file-as-root refusal, original-root replacement,
component replacement during acquisition, later leaf rename, cancellation,
expired deadline and native/scoped errors. Twelve Unix methods and one additional
Linux raw-byte-name method are mandatory on their profiles. Windows explicitly
returns Unsupported; cross-platform compilation is not native Windows support.

### Quality standards

Pinned Rust1.96.0, denied warnings, rustfmt and complete Store tests. Native unsafe
openat ownership/flags receive line-level coordinator self-review. Preserve the
prior test files and all12 topology/emptiness control tuples. Three additional
controls remove no-follow, component-identity checking or hardlink rejection;
each must compile and fail its named assertion, then pass after restoration.
Full native original/restored descendant selection has13 methods on Linux.
No old selector/count is widened to absorb this family.

### Invariants

No public activation, write authority, source mutation, link-text rewriting,
inferred capability/lease, skipped assertion, retry-until-green, global hook,
new dependency, protocol/codec change or protection bypass. The accepted plan,
all21 SI-01 criteria and earlier tests remain unchanged.

### Definition of Done

Review the exact diff; pass formatting, Clippy, Store/doctests, policy and support
checks; publish a scoped PR to fix/snapshot-integrity; qualify its exact head in
full/native/causal Actions; record coordinator self-review explicitly; merge only
with the expected head and current required checks; verify fresh parent CI and
retire only the completed branch. Local tests alone do not meet this contract.
#147/#152 remain open and #146 draft; main and storage protocol remain unchanged.

## Reference semantics

POSIX openat/O_NOFOLLOW/O_CLOEXEC:
https://pubs.opengroup.org/onlinepubs/9799919799/functions/open.html
Rust File identity, descriptor cloning and shared offsets:
https://doc.rust-lang.org/std/fs/struct.File.html
Native framework and remaining obligations: accepted plan C12/I09, existing
TOPOLOGY-INSPECTION, PLANNED-ROOTS and DESTINATION-EMPTINESS delivery records.

## Local execution before publication

Owner Intel Mac / Rust1.96.0 / isolated clone and target: Store287 passed with
zero failed/ignored/filtered; Clippy for all Store targets, formatter and9 doctests
passed. Support125 Python methods and30 CI methods passed. Twelve new Unix
methods pass locally; the Linux raw-name method needs its native Actions run.
All3 new causal variants compiled and failed their named assertions (Cargo101);
restored code passed all12 local descendant methods. Counts overlap.
No native Linux/Windows execution or remote qualification is inferred here.

Local logs are preserved outside the checkout under `/tmp/lightr-c12-next-qx5DSM`.
The commit does not include compiled outputs or untracked scratch.

| Log | SHA-256 |
|---|---|
| store-full | `fb10b128199affbefa0a087627c99d672f933d64dac5abebddf5bb402be4a999` |
| clippy | `19701a965ecba8127c5711b281aad27d7a3c03c41090afa9fd85ec86647bcc12` |
| doctests | `0e07fdd4e55a7f8e1b997e6f1b7195d2d34a64ffb2fbe02f70ab770407661621` |
| support | `e1cff85192e92dc2d46d7295313c014a7db66c75a7e01018f3ccc1363835e210` |
| ci-policy | `038d60093794173bfd1427079c6ecc2296a6fc3fab3bf9890ecba04292fcb3db` |
| format | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| descendant-no-follow-build | `c02b11445ed60836c5905d40c204ac666558653f481f790b92b0a449bd78b60d` |
| descendant-no-follow-test | `b77e8a91148b5e24ffd3db0923c20b1eb486486e67929e376e10a29ee8ab535f` |
| descendant-component-identity-build | `655aa69343dcb10b5667cd251aa2d1a8fed714cd715b931c49f40ca35bbd4074` |
| descendant-component-identity-test | `a043942f0859c535a29e0545aa4303cd3219a14924b055ca2659a8565014fd82` |
| descendant-hardlink-build | `eb0dba1c56b04c9a855cb1dfbf420024c69d2fb883444be104006ba913e33e6d` |
| descendant-hardlink-test | `818f620c897f1a2072412b94f6e7f39747c639d4fb5eafac4d55231edde758be` |
| descendant-restored | `8adead0ea7465844ee3c736608ac8270b9562d54a7cd03461c88998862d6c3ac` |
