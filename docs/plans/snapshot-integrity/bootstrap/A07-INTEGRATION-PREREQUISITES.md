# A07 — Narrow legacy integration prerequisites

Date: 2026-09-17. Campaign #152; bootstrap #153; PR #154.
Base reviewed: `ef23ad1f0c530e812694dc6f6d1ba165e539dc10`.
Authority: the owner's instructions to continue through closure, followed by
`siga`. This is a recorded coordinator scope decision, not an invented separate
owner review. Main merge, release and real-data migration remain excluded.

## Delimited amendment to SI-00 ownership

The v2.2 bootstrap normally changes support/CI/contracts only. To avoid either
merging a known broken integration input or repeatedly collecting the same
failure, this amendment permits these existing-protocol repairs before SI-00
integration:

1. Replace unreserved CAS/metadata temporary filenames with atomically reserved,
   private per-operation directories. Copy/CoW destinations remain nonexistent;
   cleanup must not remove another allocation or recursively delete unknown
   entries. Cover the three users: ingestion, put_bytes and atomic_write.
2. Preserve literal Unix backslashes in selected filenames and exact symlink
   target text. Reject non-UTF8 selected paths/targets and propagate readlink
   errors rather than manufacturing a different path or an empty target.
3. Apply the pinned formatter to the affected files and the existing test-only
   hash example. No unrelated reformatting or dependency changes.

These changes repair existing ADR-0009 preservation behavior. They do NOT
implement readiness receipts, a new journal/history schema, requalification,
shared-cache leases, provenance navigation or protocol activation. SI-01/SI-02
retain their broader obligations; they cannot be closed from these repairs.
All six WPs, their five axioms and all 123 criterion IDs remain in force.

## Required evidence before integration

- Forced allocator collision: a second allocation exhausts its bounded attempts
  without modifying the first; failure is not established by timing or a hang.
- Normal allocation/cleanup and public concurrent same-digest ingestion, plus
  independent concurrent atomic metadata writes. Retain source bytes and verify
  every accepted object/readback. Unknown directory contents survive cleanup.
- Public snapshot/hydrate --verify roundtrip with distinct literal-backslash
  and slash paths and live/dangling symlink targets. Compare raw names/readlink
  text/bytes, not only the production manifest hash.
- Run negative controls against the previous implementation where meaningful;
  an allocator test that cannot compile without its new helper is not a causal
  mutation result. The public path-fidelity test must fail on the old scan.
- All five native store/index profiles, immutable schema checks, the full F0-F5
  baseline and formatting. Preserve existing diagnostics and failures; no
  allowlist, skip, retry-until-green, artifact omission or gate relaxation.
- Review the exact integrated source/test/workflow identities. Unrelated or
  unexplained failures remain blockers, not permission to widen this amendment.

The timestamp-collision experiment proves a possible shared-temporary defect.
A repair does not retrospectively attribute the historical macOS/F1/F3 failures
or measure natural collision frequency. Later positive samples do not erase
those incidents. Whole-runtime qualification remains SI-04/SI-05.

## State

Amendment recorded; repairs and evidence are not yet claimed complete. A final
receipt must enumerate changed runtime files separately from bootstrap support,
including any scope or evidence limitation. No production-format activation is
allowed by this amendment.
