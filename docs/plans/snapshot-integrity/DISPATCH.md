# Snapshot integrity — dispatch receipt

**Date:** 2026-09-16  
**Repository:** `gmhelmold/hugr-lightr`  
**Campaign:** [execution contract](https://github.com/gmhelmold/hugr-lightr/blob/fix/snapshot-integrity/docs/plans/SNAPSHOT-INTEGRITY-REMEDIATION.md)  
**Plan commit:** `1ab1fcd638525d824b78137c522305036a6382bd`  
**Plan blob:** `8e173264b2bd316839dc7b29dc3f548ec8a5891c` (read back and matched to the local deliverable).  
**Audited code:** `e4a53417f6fe7da8c4f44908af9525536443d0fa`.

## Actual result

Planning and durable task preparation succeeded. **Agent execution did not start.** The campaign is not implemented or qualified by this receipt.

The session exposes GitHub repository writes but no native subagent-spawn tool. Its container has no Rust/Cargo or Codex/OpenCode/Claude executable on PATH. Plugin discovery did not provide a connected general-purpose coding-agent executor usable for this task. No new service was installed and no billing or permission setting was changed.

The documented GitHub cloud-agent route was attempted on the first ready packet, issue #147, through `GitHub.add_issue_assignees` with assignee `copilot-swe-agent[bot]`. The actual response was:

```json
{"message":"Forbidden","status":"403","is_error":true}
```

The endpoint documentation returned with the error was `https://docs.github.com/rest/issues/assignees#add-assignees-to-an-issue`. A subsequent issue read confirmed that #147 still had no assignee. No execution/session/agent-PR receipt was returned. The error alone does not establish whether the missing authorization is account eligibility, integration scope or another GitHub restriction. Do not prescribe more permission changes without evidence.

After that rejection, no speculative assignments were sent for the other packets. An issue existing is not delegation; an accepted assignment without a session receipt is not confirmed execution.

## Task ledger

| Packet | Issue | Dependency readiness | Execution state |
|---|---|---|---|
| SI-00 | PR #146 / plan | Planning file and packets persisted | Planning complete; dispatch blocked |
| SI-01 | [#147](https://github.com/gmhelmold/hugr-lightr/issues/147) | READY | BLOCKED: assignment rejected, HTTP 403; unassigned |
| SI-02 | [#148](https://github.com/gmhelmold/hugr-lightr/issues/148) | READY | Not dispatched; no executor |
| SI-03 | [#149](https://github.com/gmhelmold/hugr-lightr/issues/149) | READY | Not dispatched; no executor |
| SI-04 | [#150](https://github.com/gmhelmold/hugr-lightr/issues/150) | BLOCKED on SI-01/02/03 | Not dispatched |
| SI-05 | [#151](https://github.com/gmhelmold/hugr-lightr/issues/151) | BLOCKED on integrated implementation and SI-04 | Not dispatched |

There were no remediation-source edits during this planning step. The new commits contain documentation only. PR #146 remains draft; no main-branch update, merge, auto-merge or release was performed. Existing code-audit and CI failures remain unresolved. The documentation commit is not a newly tested source-code baseline.

## Executor handoff

An authorized execution session with real worker tools can resume without repeating the audit. Use this launch brief:

> Work only in gmhelmold/hugr-lightr. Read CLAUDE.md, docs/plans/SNAPSHOT-INTEGRITY-REMEDIATION.md, this dispatch receipt and issues #147–#151. Resolve and record the live fix/snapshot-integrity SHA before editing. Follow the plan's invariants, file ownership and evidence protocol. Dispatch SI-01, SI-02 and SI-03 in isolated branches/worktrees, maximum three concurrent workers. If your runtime has no subagent tool, report that rather than simulate a fleet. Keep each worker PR draft and target fix/snapshot-integrity. Reconcile any moved head and integrate reviewed results only on that branch. Execute SI-04 after dependencies are integrated; qualify with SI-05 against the final exact SHA. Do not merge into main, enable auto-merge, release, weaken tests, hide failures, change billing/permissions or mutate sibling repositories. Report actual worker/session IDs, code SHAs and evidence links. Unknown ENOENT causality remains a blocker until addressed with evidence.

No recurring/background ChatGPT monitoring or automatic future dispatch is configured. Update this receipt only after a real new execution attempt or verified worker transition.
