# Snapshot integrity — publication and execution state

**Date:** 2026-09-16  
**Specification:** [SNAPSHOT-INTEGRITY-REMEDIATION.md](../SNAPSHOT-INTEGRITY-REMEDIATION.md), version 2.1.  
**Campaign:** [#152](https://github.com/gmhelmold/hugr-lightr/issues/152).  
**Bootstrap:** [SI-00 #153](https://github.com/gmhelmold/hugr-lightr/issues/153).  
**Integration branch / PR:** `fix/snapshot-integrity` / [#146](https://github.com/gmhelmold/hugr-lightr/pull/146).  
**Previous published planning head:** `ad9bdcca42a7910f1a7a7513ce719cbb2159aecb`.  
**Unchanged audited Rust baseline:** `e4a53417f6fe7da8c4f44908af9525536443d0fa`.

## Authorized operation

The owner approved publishing v2.1 and organizing the campaign in GitHub. This is documentation and issue-tracking setup, NOT implementation authorization or execution of SI-00. No agents are launched, no source/workflows are edited, and no main merge, release, auto-merge, billing or permission change is authorized here.

The prepared v2.1 technical contract is adopted by this documentation commit. Only publication/issue routing metadata was adjusted: SI-00 has its own issue and the campaign has its own tracker. The original six work packages, 30 axiom sections, 123 numbered criteria, 14 global invariants and 22 experiment families are preserved. The published specification's Git blob is `ba085ba46426f4f2eb3030f02eff403f86a1a2ce` (SHA-256 `ba833101fabbc56ea98d0d53c4ec495957f312741d9706cb708adf715dc6f741`). The commit containing this file provides its version identity; no self-referential commit hash is required.

## Tracking and limitations

The issue-mother #152 and SI-00 #153 were created successfully. Existing #147–#151 are retained and synchronized to exact v2.1 criteria; their live bodies and campaign receipt record completion of that synchronization. Every issue points to a pinned specification commit, not a floating branch as its only contract reference. PR #146 remains draft.

**Native sub-issue relationships, native Blocked by edges, and a GitHub Project/board have NOT been created by this setup.** The connected GitHub actions expose document/issue writes but no hierarchy, dependency or Projects mutations; installed-provider discovery returned no usable additional executor for those operations. The current container also has no authenticated GitHub CLI route. This is a missing action surface, not a new 403 or a diagnosis of the user's permissions. Textual parent/dependency links and task lists below do not constitute native GitHub relationships.

## Work ledger

| Packet | Issue | Delivery prerequisites | Execution |
|---|---|---|---|
| SI-00 | #153 | Later implementation authorization; accepted contract/capability bootstrap | NOT STARTED |
| SI-01 | #147 | SI-00 accepted | NOT STARTED |
| SI-02 | #148 | Pure portion: SI-00; production integration/DoD: SI-01 G-FOUNDATION | NOT STARTED |
| SI-03 | #149 | SI-01 and SI-02 integrated; accepted composite protocol | NOT STARTED |
| SI-04 | #150 | Integrated SI-03 candidate | NOT STARTED |
| SI-05 | #151 | SI-04, frozen candidate and registered fixture-matched performance budgets | NOT STARTED |

Desired native hierarchy: #152 has children #153, #147, #148, #149, #150, #151. Desired delivery-blocking edges (dependent -> prerequisite): #147 -> #153; #148 -> #153 and #147; #149 -> #147 and #148; #150 -> #149; #151 -> #150. The #148 -> #147 edge governs production integration/closure, not the explicitly allowed pure portion after SI-00. Closing an issue is not sufficient proof of these gates: record reviewed evidence and the integrated commit.

Desired Project stages are Backlog, Ready, In progress, Review, Integrated and Done, with explicit blockers. Do not create a duplicate board or overwrite an existing Project's fields just to match this suggestion. Native setup remains a separate incomplete operation until read-back confirms it. No milestone or schedule is invented.

## Handoff and truthfulness

A WP issue is an executable extract, not a second independently edited specification. Change technical contracts in the plan first, then synchronize affected issue criteria and record the plan commit. One WP may have multiple reviewable PRs. Future worker PRs target `fix/snapshot-integrity`; use `Part of #...` rather than relying on closing keywords for a non-default target. Close a WP only after its own DoD, review, evidence and integration are recorded; global qualification remains SI-05/#152 and main merge remains the owner's separate action.

Before later execution, read the live plan, campaign, issues and actual integration SHA. Reconcile drift; verify prerequisites, ownership and native capability. No issue assignment or documentation publication implies an agent session. Historical CI/ENOENT and code-audit findings remain unresolved by this setup. Existing CI may trigger on a documentation push; that is not new implementation evidence.

The earlier pre-v2 Copilot assignment returned 403 and produced no executor receipt. It was not retried. No recurring/background monitor or automatic future dispatch is configured.
