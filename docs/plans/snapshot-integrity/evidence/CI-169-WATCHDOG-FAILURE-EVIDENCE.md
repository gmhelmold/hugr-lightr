# CI watchdog: preserve evidence when cleanup fails

Date: 2026-09-19. Follow-up to merged PR #169; part of #147 / #152.
Base: `624af3bdc208abab9153a1ed5b630c99e073ca60`, tree
`ce1a09705f4e2db4fe6c107e176683f4f8b50a74`.
The delivery PR records candidate and post-merge Actions qualification.

## Defect and correction

The prior helper caught the initial TimeoutExpired but let a second timeout
from child.wait(timeout=30) escape before appending the command receipt.
Spawn, signal and cleanup-wait OS errors could also lose that evidence.

The helper now records each stage's error separately. An unconfirmed direct
child reap has exit=null and reaped=false; no exit code or cleanup success is
invented. A timeout remains failure even if the child exits zero during cleanup.
There is one termination request and one bounded reap attempt, never a rerun.
Test-progress diagnostics distinguish completed methods, methods without a
result and explicit long-running notices. Missing result does not prove a hang.

The log hash and byte count describe a snapshot at return, not guaranteed final
contents. Unreaped children or escaped descendants can retain writers. Reaped
refers only to the direct child. Filesystem errors, process/interpreter death,
synchronous native calls and arbitrary descendants are not made infallible or
universally bounded by this change.

## Reconciliation and boundaries

The unpublished patch based on 97e3f09 is superseded by this delivery. The merged
#169 additionally requires three descriptor witnesses; their enumeration and
all existing test bodies remain intact. The new mocked inventory includes them.

#169's native ownership repair is already integrated. Its new parent complete
CI [35424345235](https://github.com/gmhelmold/hugr-lightr/actions/runs/35424345235)
and PTY workflow35424345289 passed. The macOS14 archive10578498056 was rehashed
(SHA256987a465c6b68414fc3cf6a7e47b359d672b3981314ec41c60c60dfcebd69a8f8)
and its source reconstructs the above base tree. Its 35-method complete CRI
receipt passed. That is prior-head evidence, not qualification of this patch.

This Python change neither diagnoses the precise stack of the historical hang
nor changes native PTY behavior. No Rust, workflow, dependency, runner, timeout,
test-thread count, public route, accepted criterion or protocol is modified.
The original failure/cancellation records remain in #169's history.

## Five acceptance groups

### Success criteria

Return and persist failed-command evidence when initial timeout cleanup also
fails. Never accept a timeout or a recorded process error as successful testing.

### Completeness criteria

Cover second timeout, signal denial, exit racing cleanup, cleanup wait error,
spawn failure, initial wait error, failed-suite receipt persistence and test
progress. Preserve all 22 existing CI methods and native descriptor obligations.

### Quality standards

Mock cleanup-error cases deterministically without signaling fabricated PIDs.
Keep the existing real portable watchdog tests. Reintroducing the old helper
must fail the same regression which passes with the corrected implementation.

### Invariants

No retry-until-green, relaxed native acceptance, inferred exit status, false
process-tree cleanup claim, storage activation or main merge. Original SI-01
criteria and the parent five axiom groups remain unchanged.

### Definition of Done

Review this scoped diff, pass all30 CI Python methods, preserve the failing
old-helper control, publish through a PR targeting fix/snapshot-integrity and
qualify its exact source in Actions, including the four native PTY profiles.
Integrate only after all applicable gates pass; verify fresh parent CI and
retire only this delivery branch. #147/#152 stay open; #146 stays draft.

## Local evidence

Conversation Linux / Python3.13.5: baseline22 and corrected30 methods passed.
Original existing test-method ASTs are unchanged. The old execute helper parses
but fails test_cleanup_timeout_still_returns_failed_command_record with the
escaping TimeoutExpired (unittest exit1); the restored suite passes (exit0).
These are Python controls, not new macOS/Rust executions.

- Baseline log SHA256: 5612383847a49ed5ea709a1303865d3abcf0c790e58a0c24ce6cd3a9598f93f8
- Old-helper log SHA256: 381bb0dc6b15fe0d2e16ddf7e595ea33577a0ede452432f555ea1e47ef25a20d
- Restored log SHA256: 72df427baa5e856a899622f3d4a9ec83ebbac79d1873497277200d4e616dbdc2

```sh
python3 -m unittest discover -s scripts/ci -p 'test_*.py' -v
```
