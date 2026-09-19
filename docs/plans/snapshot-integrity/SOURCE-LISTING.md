# C12 — bounded source-directory name observation

Base: `deae93539a1ae0bd14b1c7f088f9159df5480341` (#178).
SI-01 #147 / campaign #152; parent #146 remains draft.
Current delivery PR records exact source, Actions, review and merge status.
This contract alone is not proof of compilation or native qualification.

## Interface

`TopologyInspection::list_source_directory(relative, DirectoryLimits, wait)`
returns exact UTF-8 component names in byte order. None selects the retained
source root; Some selects a normal relative directory via the existing no-follow
descendant adapter. It never accepts an output path or infers type from dirent.

The source directory stays pinned while a separate open-file description drives
fdopendir/readdir. The scan offset is independent even if another reader exhausted
the source handle's offset. Original topology and selected-directory identity are
revalidated before returning. Only exact dot and parent entries are excluded,
each at most once. Hidden names, directories and dangling links are included.
Invalid UTF-8, duplicate/malformed names, budgets and errors reject the whole list.

## Resource and failure boundary

Caller entry/name-byte budgets are checked before each copied name. Zero budgets
allow empty directories. Rust allocation is fallible and proportional to accepted
entries and name bytes; libc/filesystem buffering is not covered by these budgets.
At most max_entries plus one non-dot observations occur before termination.

Ordinary paths check closedir once, including after a scan error. A primary scan
error takes precedence if close also fails; a close error rejects an otherwise
successful result. Drop is best-effort panic cleanup, not an infallible receipt.
EOF does not bypass cancellation. Native calls and standard sorting are not
preemptible. Other File drops do not certify descriptor-close success.

This is NOT a coherent snapshot, hostile-directory sandbox, lease, content proof,
namespace lock, destination-representation certificate or permission to write.
Concurrent additions/removals can be unobserved; callers must validate captures
before acknowledging success. Revalidation cannot detect every ABA history.
Reading may update atime. Windows native enumeration remains Unsupported.
Public Store/capture/hydrate/GC routing and existing codecs remain unchanged.

## Five acceptance groups

### Success criteria

Return only a complete supported bounded observation; preserve exact names and
native errors. Prior shared offsets cannot fabricate an empty directory.

### Completeness criteria

Cover empty/zero budgets; hidden/directory/dangling/Unicode names; nested no-follow
selection; exact count/byte bounds; shared offsets; root/nested replacement;
fresh-versus-frozen semantics; cancellation/deadline/EOF; missing/type errors;
late read errors; malformed/duplicate/dot streams; checked-close precedence; and
Linux invalid UTF-8. Seventeen methods: sixteen Unix plus one Linux-only witness.

### Quality standards

Pinned Rust 1.96.0, denied warnings, formatter and full/native Actions. Review
pointer ownership, borrowed dirent lifetime, thread-local errno and checked close.
Preserve all 22 existing topology control tuples and their seven suite counts.
Add four compiling controls for offset independence, strict UTF-8, early EOF
cancellation bypass and nested identity. Each must fail its exact regression,
then the restored listing family must pass. Six Python policy methods enforce
registration, complete evidence and rejection of absent/ignored/failed methods.
Synthetic policy tests are not native Rust qualification.

### Invariants

No source payload read/write, output mutation, public scratch, Store initialization,
recursive deletion, inferred lock authority, retries until green, dependencies,
workflow/codec/protocol activation, protection bypass or main merge. Preserve the
accepted campaign criteria and both sides of any concurrent integration.

### Definition of Done

Exact-source complete/native/causal gates pass; labeled coordinator self-review;
expected-head merge into integration; fresh parent CI and scoped branch retirement.
SI-01, full C12 and G-FOUNDATION/G-ACTIVATION remain separate obligations.

## Historical local preparation — not current Actions evidence

The preceding local attachment had sixteen uncompiled Rust methods. Its Python
support checks and independent C/Linux directory experiment did not qualify Rust.
This publication adds checked-close coverage, a four-control Actions family and
an exact family-registry update without weakening old selectors or assertions.
The source root may be duplicated for identity ownership; only the scan stream
uses an independent offset. Native/candidate evidence belongs to the live PR.

One grouped remote clone/setup request was blocked before execution. Publication
uses GitHub Git-data objects and stdin-only formatting; no existing user checkout
is modified. No blocked request counts as an executed test.
