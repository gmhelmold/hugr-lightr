# Snapshot integrity — current execution and integration state

**Updated:** 2026-09-17. **Campaign:** #152. **Integration:** PR #146,
`fix/snapshot-integrity`; main remains unchanged by this campaign.
**Technical contract:** [v2.2](../SNAPSHOT-INTEGRITY-REMEDIATION.md) at
`e30811971f6a1c2a1e2fdb0a91dc8ccf9a06879d`; Accepted design ADR-0020.
All 123 criterion IDs, five axiom groups per WP, 14 invariants and E01-E22 remain
unchanged. This current-state record supersedes old NOT STARTED summaries;
historical receipts remain versioned and must not be mistaken for current state.

## Authority and workflow

The owner authorized execution and explicitly delegated reviewed merge,
organization, maintenance and hygiene. No repeated blanket merge confirmation
is required for qualified in-scope increments. Keep source changes in reviewed
PRs with exact candidate evidence; never use status labels to waive criteria.
Main delivery requires full campaign qualification. No release, billing or
account permission change, production migration or hidden protocol activation.

One WP may use several PRs. An accepted **inactive increment** may integrate
without claiming the full WP DoD. G-FOUNDATION remains separate from PR merge,
G-ACTIVATION and SI-05 qualification. No coding subagent was launched.

## Current work graph

| Package | Issue | Current state |
|---|---|---|
| SI-00 | #153 | COMPLETED; #154 merged as `0c48886d2c82e671c33466ca0d5ff1640f90985d` |
| SI-01 | #147 | IN PROGRESS; #155 and #156 increments MERGED; latest code integration `1dff1b927c8e8f2aeda4a4145651fc48277f590d`; full package not yet accepted |
| SI-02 | #148 | Isolated portion permitted after SI-00; full integration/DoD waits for G-FOUNDATION |
| SI-03 | #149 | Waits for accepted SI-01/02; owns coordinated public activation |
| SI-04 | #150 | Qualifies actual SI-03 activated candidate, not nonexistent behavior |
| SI-05 | #151 | Final native/resource/performance qualification after SI-04 |

The six native child links and seven dependencies were created/read back in
[35163052860](https://github.com/gmhelmold/hugr-lightr/actions/runs/35163052860).
#147 is blocked by #153; #148 by #153/#147; #149 by #147/#148; #150 by #149;
#151 by #150. Closing #153 does not close #147 or release #148's full integration.
No relation was removed by this maintenance update.

## Latest accepted increment

[Metadata integration receipt](evidence/SI-01-METADATA-INTEGRATION.md).
Source `17dbfa5b0d455f7121cda68724f31736a2b658ef`, actual tested PR merge
`6647555a0bd9ab8553dcded5ad19d2e512cf03a1`, integrated as
`1dff1b927c8e8f2aeda4a4145651fc48277f590d`. Both merges have tree
`4caed236fbaea49ddc68f0caa03a81b49483deaf` and the same ordered parents.
Workspace, five native profiles, full F0-F5, immutable contract and causal
controls pass. The original Intel DNS failure is retained separately from the
successful same-source retry. Three new metadata/cleanup defects are fixed.
Earlier lock-release evidence remains in its own historical receipt.

`production_protocol_enabled=false`. **G-FOUNDATION and G-ACTIVATION are not
established.** Finish CoW/fallback, checkpointed existing-object inspection,
topology/resource/reaping, typed metadata caller integration and remaining
qualification before accepting SI-01. PR #156 no longer needs reconciliation;
use the integrated metadata helper and shared decoder in subsequent work.

## Hygiene and external boundary

Only verified merged/retired campaign branches are eligible for removal. The
admin branch's unique history is archived in the prior lock-release receipt;
no administrative workflow remains active. Branch #156 was retired only after
its reviewed merge, exact-tip and ancestry checks; its history is retained.
Do not mutate siblings, unrelated user checkouts, or non-campaign branches.

Account Project creation was previously denied to Actions; the current local
account query lacks read:project. The board is not marked complete and account
credentials/scopes are not changed as repo hygiene. This is not a runtime gate.

## CI integration incident and permanent acceptance rule

The parent PR's run `35285486823` at `c1d3ca9` was RED despite passing
increment-specific workflows. Both Windows-GNU checks rejected Unix-only
mutability under `RUSTFLAGS=-D warnings` in `lease_io.rs`. Treat this as a
coordinator integration failure, not a network or permission problem.
Earlier green slice receipts remain scoped historical evidence, not proof
that the parent PR was green. No main merge occurred.

The repair scopes mutability to Unix while preserving mode 0700. The shared
full CI now applies to both main and integration PR targets. Its `Required CI`
aggregate rejects any mandatory failure/cancellation/skip/missing result. The
integration branch requires that exact Actions check with strict base currency
and administrator enforcement; direct force-push/deletion are disabled.
Do not substitute native subset results for cross-compile or full-workspace
checks. The live repair PR/run records the actual verification outcome.

Finish this CI repair and verify parent #146 on its exact new head before
resuming feature work. This changes no package criterion or activation gate.

## Resume

Read this file, the current issue/PR and accepted contract. Fetch and compare
exact head before writes; preserve concurrent commits and avoid force-push.
Use a fresh scoped branch for remaining work, not a resurrected merged PR.
Record exact binaries/checkouts; retain original failure evidence. Close a WP
only after all five axioms and its reviewed integration DoD are met.
