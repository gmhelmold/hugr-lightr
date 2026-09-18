# Snapshot integrity — current execution and integration state

**Updated:** 2026-09-18. **Campaign:** #152. **Integration:** PR #146,
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
| SI-01 | #147 | IN PROGRESS; foundation/metadata #155/#156, inspection/policy #161/#163, capture #164 and estimator #166 integrated; topology #165 carries its live acceptance record; full package not accepted |
| SI-02 | #148 | Isolated portion permitted after SI-00; full integration/DoD waits for G-FOUNDATION |
| SI-03 | #149 | Waits for accepted SI-01/02; owns coordinated public activation |
| SI-04 | #150 | Qualifies actual SI-03 activated candidate, not nonexistent behavior |
| SI-05 | #151 | Final native/resource/performance qualification after SI-04 |

The six native child links and seven dependencies were created/read back in
[35163052860](https://github.com/gmhelmold/hugr-lightr/actions/runs/35163052860).
#147 is blocked by #153; #148 by #153/#147; #149 by #147/#148; #150 by #149;
#151 by #150. Closing #153 does not close #147 or release #148's full integration.
No relation was removed by this maintenance update.

## Current increment map — 2026-09-18

Inspection cancellation #161, its mandatory-name policy #163 and live-file
clone/copy #164 are integrated. Resource estimation #166 is integrated as
`7b3c802d2baac121bcf59320ad6a17df50e7d063`. Topology inspection #165 is integrated
as `0d9d5069a9a518ad37c36c9d5c141c3f7bf1aa79`. Fresh #165 parent CI 35395588711,
parent companion 35395588705, push bootstrap 35395584711 and push companion
35395584652 all passed. Those observations do not qualify later commits.

[Destination-only inspection](DESTINATION-INSPECTION.md) is the current additive
prerequisite for stored-snapshot materialization: do not invent a live source,
and do not expose an existing ancestor as a missing output's directory handle.
Its exact PR/head determines review and integration status. This remains read-only
observation, not an anchored writer, namespace lock or writable capability.

The actual macOS CoW witness and portable fallback results do not certify Linux
reflink or ReFS success. [Resource estimation](RESOURCE-ESTIMATE.md) is arithmetic
on caller inputs, not native inventory discovery, cleanup or recovery authority.
All original SI-01 criteria remain mandatory. Complete C12, destination writes,
recursive traversal, Windows native inspection, typed inventory/metadata callers,
resource/reaper implementation and actual constrained-resource E18 remain open.
Neither this helper nor a passing CI establishes G-FOUNDATION or G-ACTIVATION.

## Earlier accepted inspection increment — historical evidence

[Inspection cancellation receipt](evidence/SI-01-INSPECTION-INTEGRATION.md).
Source `3629b310cfa004dec5918b862ec2acea05e4411f`, tested merge
`aa6e07ff26c49be94e5b8c2dd63df39d6ff07aef`, actual integration
`2aa762fd9b5c4b90800e4752a4f63e57fb9bd46e`. Both merge trees are
`60ea55bb182e44c67bc55f26fb9cbec48c99542f`, with the same ordered parents.
All seven candidate workflows passed before integration. The new parent CI
was run `35307295711`; the associated receipt preserves that result.
Any later data-only maintenance head needs its own current checks as well.

Existing-payload inspection now observes cooperative cancellation during the
bounded hash. It preserves actual I/O failures, payload/receipt identities and
same-key waiter error propagation; a distinct digest may progress. Full Store
verification and four compiled local causal controls passed. All seven new
method names were checked in all five native profiles. The follow-up native
policy now requires those seven inspection-cancellation names explicitly.
Five policy test methods cover their registration plus complete and missing,
ignored (even allowlisted), or failed synthetic receipts. Synthetic controls
are not native execution evidence. Prior metadata and lock-release receipts
remain historical evidence; the policy follow-up requires its own current CI.

`production_protocol_enabled=false`. **G-FOUNDATION and G-ACTIVATION are not
established.** The inspection-cancellation increment is integrated; remaining
native CoW capability rows, topology/resource/reaping, typed metadata callers and
complete qualification must satisfy the original SI-01 criteria. No broad
public-path conversion or format activation is implied by this increment.

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

Repair #159 was merged as `5254a3ec30561c5eb354f68bb3061fd0afb6efec`.
Incident #160 was closed after NEW parent runs `35301946157` and
`35301946189` passed; the closure is recorded in #160 and #159. Do not
reapply superseded local listener patches or treat the incident as still open.
The same rule continues for #161 and every later increment: exact candidate
gates before merge, then fresh parent-head verification. Historical failing
observations remain in their receipts. No package criterion is waived.

## Resume

Read this file, the current issue/PR and accepted contract. Fetch and compare
exact head before writes; preserve concurrent commits and avoid force-push.
Use a fresh scoped branch for remaining work, not a resurrected merged PR.
Record exact binaries/checkouts; retain original failure evidence. Close a WP
only after all five axioms and its reviewed integration DoD are met.
