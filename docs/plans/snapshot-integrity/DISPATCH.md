# Snapshot integrity — planning and dispatch status

**Date:** 2026-09-16  
**Current specification:** [SNAPSHOT-INTEGRITY-REMEDIATION.md](../SNAPSHOT-INTEGRITY-REMEDIATION.md), version 2.0.  
**Current instruction:** revise the PLAN ONLY. No implementation/agent launch was requested in this revision.  
**Code baseline:** `e4a53417f6fe7da8c4f44908af9525536443d0fa`.  
**Previous planning head:** `19b28d7ad2ffff66e176b589f75c6592bd47ebe8`.

## Actual state

The v1 plan was adversarially reviewed and replaced by v2. The earlier unconditional READY labels are superseded. Each of SI-00–SI-05 now has explicit Success Criteria, Quality Standards, Completeness Criteria, DoD and Invariants. R01–R14 have design responses and named evidence obligations; their Rust fixes are NOT implemented by this revision.

No new dispatch attempt, agent session, source edit, CI/workflow edit, main-branch update, merge, auto-merge, release or permission/billing change is part of this plan-only update. A documentation push may trigger the repository's existing CI automatically; that is not new implementation or qualification evidence.

| Packet | Current readiness | Execution |
|---|---|---|
| SI-00 / PR #146 | Awaiting later execution authorization; contract/capability ratification not run | NOT STARTED |
| SI-01 / #147 | Blocked on authorized SI-00 and accepted shared seam | NOT STARTED |
| SI-02 / #148 | Pure portion needs SI-00; production integration also needs G-FOUNDATION | NOT STARTED |
| SI-03 / #149 | Blocked on integrated foundation/capture and accepted pending-protocol ADR | NOT STARTED |
| SI-04 / #150 | Read-only design can occur earlier; proof runs require SI-03 candidate | NOT STARTED |
| SI-05 / #151 | Blocked on SI-04, frozen candidate and registered performance budget | NOT STARTED |

No native capability verification, budget value, ADR approval, independent review or executor availability is implied by this ledger. Historical code/CI failures remain open.

## Historical dispatch attempt — preserved, not repeated

On 2026-09-16, before v2, assignment of issue #147 to `copilot-swe-agent[bot]` was attempted via `GitHub.add_issue_assignees`. The API returned HTTP 403 Forbidden. A read-back confirmed no assignee and no execution/session receipt. The other packets were not assigned speculatively. This does not identify the exact credential/account eligibility failure and is not a recommendation to change permissions again.

The previous authoring environment exposed no native subagent spawn and no Rust/Cargo or coding-agent CLI on PATH. Future executors must inspect their own current capabilities rather than treating that historical observation as a permanent platform fact. There is no background ChatGPT monitor or automatic future dispatcher.

## Later authorized handoff

> Work only in gmhelmold/hugr-lightr after a new implementation instruction. Read the v2 specification, this ledger, repository instructions and issues #147–#151. Record the live branch SHA and reconcile changes. Execute SI-00's contract, native-capability, caller-map and test-support prerequisites first. Do not dispatch three implementation workers merely because v1 once called them READY. SI-01 owns the foundation; SI-02 may implement only its isolated capture/oracle portion against the accepted seam in parallel, and integrates after G-FOUNDATION. SI-03 owns references, recovery, GC and snapshot/history/reader callers together. SI-04 proves the integrated behavior; SI-05 qualifies the frozen candidate against registered budgets. Each package must meet all five axioms. Use separate worktrees and draft PRs targeting fix/snapshot-integrity. Report real session IDs/evidence or precise blockers; do not simulate delegation. No main merge, auto-merge, releases, sibling mutations or credential/billing changes.

Issue bodies are routing summaries. The v2 specification is authoritative; historical issue comments and receipts remain evidence of earlier states, not permission to run obsolete packets.
