# C13 — native capacity-counter observation

Part of SI-01 #147 / campaign #152; parent #146 remains draft.
Base: `239ead0d4fba64f1409a4ab20b29881c36426215` (integrated #176).
This support increment follows ADR-0020 and the accepted C13 resource runbook.
Its delivery PR records current qualification and integration; old attachment
results or previous PR checks never approve a newer candidate.

## Scope

`scripts/si01/resource_capacity.py --directory PATH` observes one existing
Linux/macOS directory. The exact caller path is opened read-only; identity and
statvfs counters come from that same retained descriptor. No lexical alias/parent
normalization, fallback to an ancestor, Store initialization, payload read,
staging allocation, lease, recovery or deletion is performed.

Capacity uses checked `f_frsize * f_bavail`; preferred I/O size `f_bsize` and
reserved blocks in `f_bfree` do not replace those counters. The accounting unit
is not a measured allocation-overhead profile. Finite inode availability is
reported separately. Sentinel counters and unavailable accounting stay unknown;
contradictory counters and u64 overflow fail. Quota always remains `null`.

Device/inode identify the opened directory, NOT an independent capacity pool,
mount-provenance proof, lock or writability guarantee. Path replacement after
open cannot substitute the measured object, but pathname binding is not verified.
Do not automatically sum directory observations or infer independent backing
pools. Shared storage, quotas/subvolumes and resource-domain membership still
require the caller's complete inventory. Counters can change immediately.

Primary and close failures retain separate phases/errno. Any error withholds the
capacity projection. Close is attempted once; an error leaves descriptor
ownership disposition unknown and is never retried. Checkpoints also run after
measurement and successful close, but cannot interrupt a blocked synchronous
call. MemoryError, interpreter termination, native hangs and stdout failures are
not made infallible receipt paths. Read-only opens may have native metadata effects.

## Output and composition

Exit 0 / CAPACITY_OBSERVED means only measurement and checked close completed.
Exit 2 reports failure; exit 3 reports unsupported capability. Failure has no
capacity projection. Pool identity, pathname binding, quota, writability,
allocation profile, inventory completeness, retry authorization and runtime/
protocol qualification remain unverified/false. Scratch-reclamation credit is zero.

The output is NOT a complete `resource_estimate.py` inventory. A caller may use
its capacity fragment only with separately established domains, dependencies
and measured costs. Unknown quota keeps the estimator's result
INCOMPLETE_INFORMATION; CLI success does not authorize a recovery retry.
Windows native capacity is unsupported. No native Windows measurement or E18
no-space/quota/recovery qualification is claimed by this increment.

## Five acceptance groups

### Success criteria

Observe finite native capacity through one retained directory descriptor without
user-data mutation or invented quota/pool/profile information. Preserve primary
and cleanup failure context; never accept a capacity projection after error.

### Completeness criteria

Cover fragment/reserved-block distinctions; zero/sentinel/inconsistent counters;
invalid types and overflow; missing/invalid/non-directory paths; native aliases
and post-open replacement; cancellation before/after measurement and close;
read/close failures; unsupported capability; descriptor cleanup/CLOEXEC; JSON/CLI
exit behavior; composition with the existing estimator's unknown quota.

### Quality standards

Standard-library Python only; reuse existing estimator checks and scoped syscall
seams. Preserve all prior support/CI tests and runtime assertions. Distinguish
real filesystem/CLI execution from synthetic counter/failure controls. All four
negative variants must parse and fail the named assertions, then pass restored.

### Invariants

No Rust, public Store/CLI routing, workflow, dependency, stored codec, account,
protection or accepted-criterion changes. No invented resource domain, quota,
allocation-cost profile, recovery authority or space reservation. No disk filling,
recovery replay, recursive cleanup, release, main merge or protocol activation.
Only DISPATCH documentation is updated besides the three original candidate files.

### Definition of Done

Review the four-file delta, pass existing discovery-based support and complete CI
at the exact source/base, verify native Linux names and the recorded Mac results,
record explicitly labeled coordinator self-review, merge using the expected head,
verify the fresh parent checks and retire only this delivery branch. This does
not close SI-01 or establish G-FOUNDATION/G-ACTIVATION. Windows native support and
actual constrained-resource recovery remain separate outstanding obligations.

## Reproduction and local evidence

```sh
python3 -B -m unittest discover -s scripts/si00 -p 'test_resource_capacity.py' -v
python3 -B -m unittest discover -s scripts/si00 -p 'test_*.py' -v
python3 -B -m unittest discover -s scripts/ci -p 'test_*.py' -v
```

Prior attachment: conversation Linux/Python3.13.5, on #175's source, passed
29 capacity methods (8 native filesystem/CLI,21 synthetic/composition),161 support
methods and30 CI methods. This is historical evidence, not current-base CI.
The implementation/tests were preserved byte-for-byte during publication on #176.

Current owner-Mac execution uses an owned isolated clone, Python3.14.5 and only
temporary fixtures. Capacity:28 passing/1 Linux-only raw-name skip. Full support:
166 passing/that same skip (167 total). CI-policy:30 passing. Counts overlap.
All four local negative variants parsed, failed their intended assertions (exit1)
and passed after source restoration (exit0); no import or syntax error counted.
The changes cover only Python/support documentation; Rust has not been rebuilt
locally for this increment. Exact Actions qualification is required by the PR.

Local raw log SHA256 values:

- `capacity-macos.log`: `bd0a5b63195dc8e4b0a025208ba888e18d93ae3f8d4161725cb139274c241026`.

- `support-macos.log`: `16726597882c7e9e45f3832684b64c352574e2976dc1e6aa76b3eb59d889fe9e`.

- `ci-policy-macos.log`: `67f52c8569ca9638d14226703032177207683190b7ffeff955144cd3e470649a`.

Scoped controls (not additional test methods):
- `fragment-unit-mutant`: exit1, SHA256 `96c3f46878da3c7f849b51d3fb95acb07d0b16c64415911cb060307387013430`.
- `fragment-unit-restored`: exit0, SHA256 `5728de11aef791f5454653d9c00a8f2052f31ebc9415d4c73ff9c91374b650d8`.
- `reserved-blocks-mutant`: exit1, SHA256 `77759aa3cb5c3dc42d48732c2404b112803e9a4c15aa2b0aec0b786261b6c4fb`.
- `reserved-blocks-restored`: exit0, SHA256 `5728de11aef791f5454653d9c00a8f2052f31ebc9415d4c73ff9c91374b650d8`.
- `invented-quota-mutant`: exit1, SHA256 `2c402e02f3052222b1972a184e0d1141236f2535d42dc885eaacffe1501d9713`.
- `invented-quota-restored`: exit0, SHA256 `ed8bd9813bdd0b6b2e5a51ae5cc2eba42fe837ddede19bfcc7170ec42168d0b8`.
- `lost-close-error-mutant`: exit1, SHA256 `7da19aecbc30f01c9031b802ea9e152c08248b430627d1458e99e710c726f262`.
- `lost-close-error-restored`: exit0, SHA256 `c0b310de4535b1ef55c95f6ec5053aefb02e7b91bca73e5b0e5443879cbb5ced`.

Native field reference: https://docs.python.org/3/library/os.html#os.fstatvfs .
No broader quota, durability or independent-pool guarantee is inferred.
