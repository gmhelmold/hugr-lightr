# C13 — partial-counter consistency follow-up

Base: `fbabc05859c4caa440b2a41f3c42aaed865bb909` (integrated #177).
Part of SI-01 #147 / campaign #152; parent #146 remains draft.
This repairs support arithmetic; it does not activate storage or public writing.

## Defect and selected correction

The integrated collector checked ordering only when all three counters were
known. A missing total could therefore hide available > free; a missing free
counter could hide available > total. The same defect affected inode counters.
These are synthetic contradictory inputs, not a claimed observed OS defect.

Retain the native counter order total >= free >= available, remove only unknown
observations, and validate the remaining finite subsequence. This preserves
constraints across a missing intermediate counter without fabricating any value.
Unknown capacity, quota, pool identity and allocation costs remain unknown.
An invalid observation withholds its projection and closes the descriptor once.
Native calls, resource_estimate.py, existing tests and public routing are unchanged.

## Five acceptance groups

### Success criteria

Reject every contradiction among known block/inode counters even if another
counter is unknown; compatible partial observations preserve existing semantics.

### Completeness criteria

Cover every missing position, all three existing sentinel representations, block
and inode families, compatible partial states and failed-observation cleanup.
Keep all 29 existing capacity methods and assertions unchanged; add six methods.

### Quality standards

Standard-library Python only; bounded arithmetic over three counters. Preserve
all prior tests and gates. Demonstrate the same regressions fail before and pass
after the repair; distinguish subcase counts from methods. Use an independent
pairwise oracle and native owned-directory tests, without filling a host disk.

### Invariants

No guessed quota, pool independence, allocation profile or recovery authority.
No Rust, dependencies, workflows, codec, native assertions, accepted campaign
criteria, protection settings, main integration or protocol activation changes.
Unknown inputs must not conceal contradictions between known observations.

### Definition of Done

Current-head support and complete Actions gates pass; explicitly labeled
coordinator self-review, expected-head merge into fix/snapshot-integrity,
verification of fresh parent checks, and retirement of only this delivery branch.
G-FOUNDATION/G-ACTIVATION and full SI-01 remain separate and unestablished.

## Executed local evidence — 2026-09-19

Owner Intel Mac / Python 3.14.5, isolated clone and owned temporary fixtures:
six new methods on the integrated old helper fail (five failing methods,
19 failing subcases; no import/compilation errors); repaired capacity suite has
34 passes and one Linux-only skip (35 methods). Full support has 172 passes
and that same skip (173 methods); CI policy has 30 passes. Counts overlap.
The independent oracle covers 432 synthetic combinations of two counter families
and six values per position; this is not 432 native filesystem executions.
Only decode_counters differs among production top-level definitions. Every old
test method body remains structurally identical. macOS exercises seven existing
native filesystem/CLI methods; Linux raw-name behavior awaits current Actions.
No Rust or constrained-resource recovery experiment was run for this repair.

Local log SHA256 values (raw logs retained outside the checkout):

- `before-regressions.log`: `129bd98abe1da04a03c26023412eb5fab6b9fa60cd40fa82ac36eb62b1639003`.

- `capacity-after.log`: `d94cea14b575a2389b0f5af32d9012cd02dcca630967021b588245cb1911ae82`.

- `ci-policy.log`: `5c6657e37afc09e13edc4ca4d67ece8543690564a6ac83fdbd9ffe695a80ba4a`.

- `oracle.log`: `24345c6d0a48e745854dbb9e71dcd687487c2da4cc7ebe1f77d0d7957f818ba8`.

- `support.log`: `fa16d81f2f4c763e7f520063fe62b1a9b17de39514dde3dde278eec7960189e4`.
