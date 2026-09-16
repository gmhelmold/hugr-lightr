# Snapshot integrity — publication and execution state

**Date:** 2026-09-16  
**Specification:** [SNAPSHOT-INTEGRITY-REMEDIATION.md](../SNAPSHOT-INTEGRITY-REMEDIATION.md), version 2.2; technical commit `e30811971f6a1c2a1e2fdb0a91dc8ccf9a06879d`.  
**Campaign:** #152. **Bootstrap:** SI-00 #153.  
**Integration:** `fix/snapshot-integrity` / PR #146 (draft).  
**Unchanged audited Rust baseline:** `e4a53417f6fe7da8c4f44908af9525536443d0fa`.  
**Tracking status:** six native children and seven blocking edges VERIFIED; Project creation attempted and denied for the Actions identity.

## Scope and technical contract

The owner requested completion of the campaign's GitHub organization. This tracking update does not execute SI-00, accept ADRs, change Rust or its tests, activate the storage protocol, migrate data, launch agents, merge, release or change app permissions/billing. A temporary administrative workflow was used on an isolated branch and then removed; it was never merged into main or this integration branch.

The technical v2.2 specification is unchanged: six WPs, 30 axiom sections, all 123 criterion IDs, 14 global invariants and E01–E22. The integrated-review refinements remain planning obligations: actual resource/lock domains, native symlink/parent resolution, inert foundation followed by G-ACTIVATION, independent pending pre-scan and rejection of empty/echo-only benchmark evidence.

**Specification Git blob:** `5317d313c7edee03f83742173888191c9a885745`. **Specification SHA-256:** `e9c44406190b154e3a7f7fec24889806249b42a24f1ac666bccb9609e4d7f23b`. WP criteria remain pinned to the technical commit above; this separate tracking receipt does not amend their contracts.

## Native hierarchy and dependencies — completed and verified

[Run 35163052860](https://github.com/gmhelmold/hugr-lightr/actions/runs/35163052860), workflow commit `93f2504061995cc2573e0ab6b03991dd172a686d`, created and read back the native relationships below. Existing parents were checked first; no unrelated parent was replaced and no dependency was removed.

**Parent #152:** native children #153, #147, #148, #149, #150, #151.

| Dependent | Native Blocked by | Meaning |
|---|---|---|
| #147 | #153 | SI-00 accepted |
| #148 | #153, #147 | SI-00 permits its isolated pure portion; SI-01/G-FOUNDATION is required for production integration and DoD |
| #149 | #147, #148 | Both primitives/capture integrated and accepted composite ADR |
| #150 | #149 | Actual post-G-ACTIVATION candidate |
| #151 | #150 | Frozen candidate plus registered fixture-matched budgets |

There are six child links and seven blocking edges, not just Markdown navigation. The [second run, 35163241378](https://github.com/gmhelmold/hugr-lightr/actions/runs/35163241378), updated all six WP relationship headers, verified complete body read-back and preserved all numbered criteria and their hashes. Native relationships do not prove the readiness gates or imply that an issue is implemented.

## Project — exact authorization boundary, not completed

The second administrative run queried existing Projects and attempted `createProjectV2` for the repository owner's account. GitHub returned GraphQL error type `FORBIDDEN` at `createProjectV2`:

```text
github-actions[bot] does not have permission to create projects on ownerId U_kgDOD9CVIg.
```

**No Project was created.** The run returned failure rather than claiming all setup was complete. The earlier absence of a connector mutation has been narrowed to an actual account-Project authorization denial. Repository issue access does not provide that account-level permission. No new secret, alternate credential or account permission was obtained or changed, and no bypass or automatic retry is scheduled.

Completion of the Project requires a separately authorized Projects connection or the owner's own authenticated GitHub session. Preserve this requirement; do not silently waive the requested board, create a replacement in an unrelated service, or turn the denied operation into a green checkbox. Do not request tokens/passwords in chat. The intended view remains Backlog, Ready, In progress, Review, Integrated and Done, with explicit blockers and no invented dates.

## One-off workflow cleanup

The administrative job token was limited to `contents: read` and `issues: write` plus GitHub's metadata read. Both runs used `chore/snapshot-tracking-setup`, with exact repository/branch checks and fixed campaign issue IDs. No runtime source was checked out or executed by the setup script.

The temporary `.github/workflows/snapshot-tracking-setup.yml` was removed from that branch in commit `c46c5ed34063312f6857120e4d8b953f08d9a617`. It was never merged into main or the runtime integration branch. No ongoing administrative workflow, recurring monitor or background dispatcher remains. Existing repository benchmark workflows may have triggered automatically on the administrative pushes; they are not claimed as runtime qualification.

## Work state — all NOT STARTED

| Package | Issue | Prerequisites / meaning of completion |
|---|---|---|
| SI-00 | #153 | Later implementation authorization; accepted domains/seams/activation plan, native capability and valid baseline-evidence bootstrap |
| SI-01 | #147 | SI-00; G-FOUNDATION certifies inert primitives and records production_protocol_enabled=false |
| SI-02 | #148 | Pure portion after SI-00; actual primitive integration after G-FOUNDATION; no production activation |
| SI-03 | #149 | Integrated SI-01/02 and accepted composite ADR; ordered slices culminate in coordinated G-ACTIVATION on disposable stores |
| SI-04 | #150 | Actual activated SI-03 candidate; causal proof of public paths and all required experiments |
| SI-05 | #151 | Frozen SI-04 candidate; native/resource qualification and real comparison against registered budgets |

G-ACTIVATION is a source-integration gate, not production rollout, SI-05 qualification or owner authorization to migrate data. One WP may use multiple reviewed PRs, but coupled acceptance remains one obligation. Worker PRs target `fix/snapshot-integrity`; issue closure requires its own DoD, evidence and integrated SHA. Completing GitHub relationships does not complete any work package.

## Review dispositions and resume

Integrated-review A01–A04 remain specified, not implemented. A05's evidence gate remains specified: the historical zero-artifact/echo-only benchmark aggregator has not been corrected by this tracking operation. A06 is now complete for native hierarchy/dependencies and still blocked for the account-level Project, with the exact attempt recorded above. The original CI/ENOENT and source defects remain unresolved by administrative setup.

Before later implementation, read the live specification, campaign and WP criteria, reconcile the actual integration head, verify prerequisites and ownership, and retain the accepted resource-domain, path-resolution, activation and pending-scan contracts. Preserve the earlier failed Copilot assignment receipt; no coding agent was launched or retried. Qualification remains SI-05, and main merge/release remains a separate owner decision.
