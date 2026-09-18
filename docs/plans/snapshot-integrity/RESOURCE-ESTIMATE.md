# C13 recovery capacity estimate — support contract

**Date:** 2026-09-18. **Parent work:** SI-01 #147 / campaign #152.
**Source basis:** integration `90ce0dc12ab0af43bdc51b13a0aa7d5863e74c1d`,
Git tree `23c2ba918ca1d9886747c3aa02de45a642e7139a`.
**State:** versioned support contract; PR #166 records current qualification and
integration. This is arithmetic, not new Rust Store behavior, a recovered
pending-operation inspector or a C13 qualification receipt. No new architectural
or persistent-format decision.

## Contract and boundary

The accepted [v2.2 specification](../SNAPSHOT-INTEGRITY-REMEDIATION.md), C13,
requires a conservative estimate of every required ordinary rewrite plus bounded
metadata and measured filesystem overhead. It must distinguish free bytes,
quota and inodes and must not invent a zero-space recovery guarantee.

`scripts/si01/resource_estimate.py` calculates that estimate from an explicit
caller-supplied inventory. It does not open a Store or a pending descriptor,
resolve path aliases, verify referenced evidence, acquire any lock, probe free
space, reserve resources, delete scratch, recover a transaction or authorize a
retry. The JSON is a transient support request, NOT another persistent ledger,
new CAS metadata, a readiness proof or an activation switch.

The caller must first supply a complete dependency/allocation inventory under
the actual resource-domain protection. A Store lock does not protect a shared
cache. Identity labels must already be resolved by the caller; this tool cannot
prove that two different labels are not aliases. Unknown classifications or
missing measurements stay explicit, never become zero cost.

## Scope of the calculation

For each Store/cache/workspace domain, equal digest dependencies are counted
once only when both their length and classification agree. Conflicting repeats
are errors. A confirmed dependency has no payload rewrite in this calculation;
its newly required receipt/history/pending writes must still be listed explicitly
under metadata. The script does not infer which writes a transaction needs.

A `rewrite` is the FULL ordinary byte rewrite required by C03, not the physical
size of a sparse/reflink source. No compression, CoW, hardlink, emergency reserve
or reclaimable-scratch credit is accepted. An unknown object prevents a complete
estimate even when its length is known. Scratch reclamation credit is always zero.

Different domains on the SAME capacity pool are summed BEFORE comparing free
space, quota or inode availability. Equal digests in separate Stores are not
deduplicated across their resource domains. Different pools cannot lend one
another space. This model assumes each domain allocates in one canonical
capacity pool with one compatible quota/inode accounting scope. Overlapping
project/user quotas, storage spanning pools or unproven alias equivalence need
an extended native inventory; they are not modeled as independently available
capacity by inventing labels.

For a measured profile with allocation unit `U`, file overhead `F`, directory
cost `D`, and file/directory inode costs `If`/`Id`:

```text
known bytes = sum(round_up(each rewrite length, U) + F)
            + sum(metadata count * (round_up(metadata bytes, U) + F))
            + new-directory count * D
known inodes = (unique rewrites + metadata-file count) * If
             + new-directory count * Id
```

Every addition/multiplication and the rounded result must fit u64. Rounding is
per allocation, not after summing logical lengths. Costs are summed as additional
allocations, with no credit for retiring old files; the caller must list all
simultaneously necessary staging/receipt/directory work. The profile is a supplied
estimate, not a theorem about filesystem allocation or future external consumption.
No universal allocation costs are built in. A measurement digest is retained as
a reference only: `evidence_references_verified=false` always.

## Request shape (schema 1)

All fields shown below are required; unknown fields and duplicate JSON keys are
rejected. The numbers and hash strings below are SYNTHETIC EXAMPLES, not
filesystem observations or input suitable for real recovery.

```json
{
  "schema": 1,
  "inspection_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "pools": [{
    "id": "fs:example",
    "profile": {
      "measurement_sha256": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "allocation_unit_bytes": 4096,
      "file_overhead_bytes": 128,
      "directory_bytes": 4096,
      "inodes_per_file": 1,
      "inodes_per_directory": 1
    },
    "capacity": {"free_bytes": 60000, "quota_bytes": "not_applicable", "free_inodes": 100}
  }],
  "domains": [{
    "id": "store:example",
    "kind": "store",
    "pool_id": "fs:example",
    "inventory_complete": true,
    "objects": [{
      "digest": "1111111111111111111111111111111111111111111111111111111111111111",
      "length": 4097,
      "state": "rewrite"
    }],
    "metadata": [{"id": "receipt:example", "bytes": 80, "count": 1}],
    "directories": 1
  }]
}
```

Domain kind is `store`, `cache` or `workspace`; object state is `rewrite`,
`confirmed` or `unknown`. Identity labels are opaque bounded tokens, never paths.
Metadata entries represent planned allocations, not whole payloads; the maximum
size is ADR-0020's largest frame (1,048,576-byte body plus 44-byte framing).
`count` is positive. Duplicate metadata allocation IDs are rejected.

`profile` may be null, which leaves physical-cost totals unknown. Capacity values
may be null (unknown), or u64. Only quota/inodes may be the explicit string
`not_applicable`, reported as a CALLER declaration, not independently verified
absence of a limit. Free bytes cannot be declared not applicable.

The input is bounded to 8 MiB, 16 nested container levels, 128 pools, 1,024 domains
and 100,000 object/metadata records in TOTAL. These are parser/engineering bounds,
not measured performance claims. Negative, fractional, exponential, non-finite,
boolean and overflowing integers are rejected. Source input is read-only.

Input must be a blocking binary stream. The bounded reader consumes through EOF;
a short read alone never certifies that the document is complete. The total read
budget is 8 MiB plus one byte to detect overflow, including fragmented input.
A later I/O error is propagated even if an earlier fragment was valid JSON.
`None` (nonblocking no-data), non-byte results and a reader returning more than
its requested budget fail explicitly; no read-error retry or partial success is
introduced. This does not set an I/O deadline or preempt an already blocked read.
See Python's [stream read contract](https://docs.python.org/3/library/io.html).

## Use and outcomes

```sh
python3 scripts/si01/resource_estimate.py --input inventory.json
# Or supply the JSON on stdin; stdout is the result, not a file replacement.
python3 scripts/si01/resource_estimate.py < inventory.json
python3 -m unittest discover -s scripts/si00 -p test_resource_estimate.py -v
```

| Exit | Status | Meaning |
|---|---|---|
| 0 | ESTIMATE_ONLY | Arithmetic is complete for the supplied inputs; each supplied applicable capacity is at least the estimate. NOT retry permission. |
| 2 | INVALID_INVENTORY | Malformed, contradictory, out-of-bounds or unreadable request. No estimate accepted. |
| 3 | INCOMPLETE_INFORMATION | Unknown dependency, incomplete inventory, missing profile or unknown applicable capacity. Known subtotals remain visible. |
| 4 | OBSERVED_RESOURCE_SHORTFALL | A supplied limit is below the known allocation estimate, including when other information remains incomplete. |

A zero-allocation estimate is possible but never certifies zero-space progress:
`retry_authorized=false`, `runtime_qualified=false` and
`production_protocol_enabled=false` always. Exit 0 validates calculation only.
A malformed request never falls back to a smaller workload. Counts marked
`known_*` are partial estimates when information is missing; `additional_*`
is null unless complete relative to the supplied inventory and profile.

## Five acceptance groups for this support increment

### Success criteria

All required rewrites are summed, shared capacity is not counted independently
per Store, and missing data cannot become permission to recover or delete.

### Completeness criteria

Cover same-domain duplicates and conflicts, separate Stores sharing capacity,
separate pools, metadata/directory overhead, per-file rounding, quota/inodes,
empty files, unknown information, u64 overflow, parser limits, fragmented reads,
EOF, late read errors and read-only CLI
success/error paths. Preserve the existing 50 support and 13 CI-policy tests.

### Quality standards

Only Python's standard library; no new native dependency, filesystem mutation,
process-global runtime failure hook, source inspection by path heuristics or
change to accepted C03/C12/C13. Synthetic examples must remain labeled.

### Invariants

No Store/GC changes, CoW credit for requalification, unverified scratch credit,
foreign-domain deduplication, inferred lock authority or retry authorization.
No original SI-01 axiom, criterion ID or required native test is changed.

### Definition of Done

Local tests and negative controls are necessary but not sufficient for merge.
Reconcile this exact diff with the live base, publish a scoped PR, pass complete
CI and applicable support/native workflows at the exact head, review it, merge
without bypass and verify the NEW parent #146 CI. The delivery PR and #147
record the actual qualification/integration state; this contract does not grant acceptance. Actual constrained-filesystem E18 recovery, typed inventory collection,
C12, scratch reaping, native capability gaps and SI-01 acceptance remain open.

## Publication review baseline — 2026-09-18

Reconciled with integration `90ce0dc12ab0af43bdc51b13a0aa7d5863e74c1d`.
The transferred implementation/test bytes are identical to the prior reviewed
three-file patch (SHA-256
`46353e45b591b0c12ac46c4d9ab617c85a59177790ff37369d001da7a6ba337f`).
Only this acceptance wording, the local evidence below and current DISPATCH
were updated for publication. No Rust, dependency, workflow or native-test
policy change; no Store access or actual constrained-resource experiment.

In a fresh isolated worktree on the owner's Intel Mac, Python 3.14.5:
83 support methods (50 existing +33 new), 13 CI-policy methods and all five
Python causal controls passed. The 400 seeded oracle cases are inside one
method, not 400 additional tests. Independently, the original delivery
verifier passed in the conversation's Linux environment with the same
83+13 checks and all five controls. No archived native executable was run.

Local evidence logs (SHA-256):
- `support.log`: `65fa8f8ca9bb09dece473f6dcaef2dd792163ae1eeede9b39383591a9ebdce18` (child exit 0).
- `ci-policy.log`: `7d9e6ded070118ebbfa72f2c9795866aa9dff3e275ad5ea0650f922b5e47f9eb` (child exit 0).
- `causal.log`: `9162cb05c861756eb67cb1fdbb1b6b6b9ff588f98636d05140ba1d45f9c608cc` (child exit 0).

## Stream review follow-up — 2026-09-18

Reconciled onto PR #166 source `f23e84df9ccb16f46f8da4316e083bb8237e661f`,
not reapplied as an add-file patch. The two Python files match the later
reviewed artifact; the current synthetic-example wording, acceptance boundary,
publication history and DISPATCH changes are preserved.

A legal short read is not end-of-input. The parser now reads through EOF with
a cumulative byte budget, rejects invalid reader results and preserves a late
I/O failure. Six new stream methods accompany the original 33 arithmetic/CLI
methods. This is API-level bounded stream validation, not a storage exploit or
a claim that regular-file reads reproduced the short-fragment case.

The six stream methods were exercised first against the published single-read
implementation and then against the corrected reader. The previous 50 support
methods and 13 CI-policy methods are retained. All examples and formula-oracle
inputs are synthetic; no filesystem capacity discovery or recovery is performed.
Actual qualification status and exact run identities belong to the PR review
and post-integration receipt, not to earlier green checks or this contract.
