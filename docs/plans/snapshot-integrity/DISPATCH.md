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
| SI-01 | #147 | IN PROGRESS; #155 increment merged as `b7ed8953df506f00f77515bcaabc117afc15ee40`; #156 metadata work remains unmerged |
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

[Lock release/integration receipt](evidence/SI-01-LOCK-RELEASE-INTEGRATION.md).
Source `8d1276858277fcf507038fa10d41b64bf54b1877`, actual CI merge
`e20ebe915fb67faf2437888e00fdbf27d3b2fb1e`, tree
`fed8b882d52cb10571bb24d95430c2d7dabfb9de`. The actual #155 integration merge
has that exact tree and ordered parents. Workspace, native matrix, full F0-F5,
contract and causal controls pass; owner-specific Unix release is fixed.

`production_protocol_enabled=false`. **G-FOUNDATION and G-ACTIVATION are not
established.** Complete CoW/fallback, inspection cancellation, topology,
resource/metadata/reaping and caller obligations before accepting SI-01.
Preserve #156's separate changes and reconcile its diff against the integrated
foundation; do not restore an obsolete duplicate codec or discard shared code.

## Hygiene and external boundary

Only verified merged/retired campaign branches are eligible for removal. The
admin branch's unique history is archived as a small Git bundle in the latest
receipt; no administrative workflow remains active. Branch #156 remains live.
Do not mutate siblings, unrelated user checkouts, or non-campaign branches.

Account Project creation was previously denied to Actions; the current local
account query lacks read:project. The board is not marked complete and account
credentials/scopes are not changed as repo hygiene. This is not a runtime gate.

## Resume

Read this file, the current issue/PR and accepted contract. Fetch and compare
exact head before writes; preserve concurrent commits and avoid force-push.
Use a fresh scoped branch for remaining work, not a resurrected merged PR.
Record exact binaries/checkouts; retain original failure evidence. Close a WP
only after all five axioms and its reviewed integration DoD are met.
